use super::*;

impl RuntimeSession {
    pub(crate) fn new(options: &PlayOptions) -> Result<Self> {
        let alsa_tuning = AlsaPlaybackTuning::from_play_options(options)?;
        let recording_metrics = Arc::new(RecordingMetrics::default());
        let midi_trace_log = Arc::new(MidiTraceLog::default());
        let controller_profile = options
            .controller_profile_path
            .as_deref()
            .map(load_controller_profile)
            .transpose()?
            .map(Arc::new);
        let input_metrics = Arc::new(InputMetrics::default());
        let prepared_runtime = build_audio_runtime(
            &options.patch_path,
            options.audio_selector.as_deref(),
            options.sample_rate_hz,
            alsa_tuning,
            options.gfm_layer_seed,
            options.bcs_layer_scenario,
            Arc::clone(&input_metrics),
            Arc::clone(&recording_metrics),
        )?;
        let runtime = start_prepared_audio_runtime(prepared_runtime)?;
        let midi_trace_worker = MidiTraceWorker::spawn(
            options.trace_midi,
            options.midi_channel,
            controller_profile.clone(),
            Arc::clone(&input_metrics),
            Arc::clone(&midi_trace_log),
        );
        let runtime_control_queue = Arc::new(ArrayQueue::new(RUNTIME_CONTROL_QUEUE_CAPACITY));
        let sound_lab_midi_focus = Arc::new(SoundLabMidiFocus::default());
        let mut session = Self {
            patch_path: options.patch_path.clone(),
            patch_name: runtime.patch_name,
            audio_selector: Some(runtime.audio_selector.clone()),
            alsa_tuning: runtime.alsa_tuning,
            midi_selector: options.midi_selector.clone(),
            midi_channel: options.midi_channel,
            controller_profile,
            trace_midi: options.trace_midi,
            gfm_layer_seed: options.gfm_layer_seed,
            bcs_layer_scenario: options.bcs_layer_scenario,
            bend_range: runtime.bend_range,
            tx: runtime.tx,
            midi_input_queue: runtime.midi_input_queue,
            runtime_control_queue,
            priority_actions: runtime.priority_actions,
            worker: runtime.worker,
            stream: Some(runtime.stream),
            driver: PerformanceDriver::Idle,
            audio_device_name: runtime.audio_device_name,
            sample_rate_hz: runtime.sample_rate_hz,
            channels: runtime.channels,
            transport_metrics: runtime.transport_metrics,
            input_metrics,
            recording_metrics,
            midi_trace_log,
            midi_trace_worker,
            sound_lab_midi_focus,
        };
        session.driver = session.open_driver_for_tx(
            session.tx.clone(),
            Arc::clone(&session.midi_input_queue),
            session.bend_range,
            Arc::clone(&session.runtime_control_queue),
            Arc::clone(&session.priority_actions),
            Arc::clone(&session.input_metrics),
            session.midi_trace_worker.publisher(),
            options.force_demo,
        )?;
        Ok(session)
    }

    pub(crate) fn open_driver_for_tx(
        &self,
        tx: mpsc::Sender<EngineCommand>,
        midi_input_queue: Arc<ArrayQueue<RealtimeMidiMessage>>,
        bend_range: f32,
        runtime_control_queue: Arc<ArrayQueue<RuntimeControlMessage>>,
        priority_actions: Arc<PriorityActions>,
        input_metrics: Arc<InputMetrics>,
        midi_trace_publisher: MidiTracePublisher,
        force_demo: bool,
    ) -> Result<PerformanceDriver> {
        open_driver_for_selector(
            tx,
            midi_input_queue,
            bend_range,
            self.midi_selector.as_deref(),
            self.midi_channel,
            self.controller_profile.clone(),
            self.trace_midi,
            runtime_control_queue,
            priority_actions,
            input_metrics,
            Arc::clone(&self.midi_trace_log),
            midi_trace_publisher,
            Arc::clone(&self.sound_lab_midi_focus),
            force_demo,
        )
    }

    pub(crate) fn set_sound_lab_midi_focus(&self, page: Option<SoundLabPage>) {
        self.sound_lab_midi_focus.set(page);
    }

    pub(crate) fn restore_demo_driver(&mut self) {
        self.driver = PerformanceDriver::Demo(DemoPerformer::spawn(self.tx.clone()));
    }

    pub(crate) fn print_startup_summary(&self) {
        println!(
            "play patch: {} ({})",
            self.patch_name,
            self.patch_path.display()
        );
        println!(
            "audio: {} @ {} Hz, {} channels",
            self.audio_device_name, self.sample_rate_hz, self.channels
        );
        println!(
            "alsa: selector={} period={} buffer={} start_threshold={}",
            self.audio_selector.as_deref().unwrap_or("unset"),
            self.alsa_tuning.period_frames,
            self.alsa_tuning.buffer_frames,
            self.alsa_tuning.start_threshold_frames
        );
        println!("mode: {}", self.driver.detail());
        match self.midi_channel {
            Some(channel) => println!("midi channel filter: channel {channel}"),
            None => println!("midi channel filter: all channels"),
        }
        if let Some(profile) = &self.controller_profile {
            println!(
                "controller profile: {} ({})",
                profile.name,
                profile.path.display()
            );
        } else {
            println!(
                "pc4 legacy live profile: cc16=gravitacija, cc17=bloom, cc18=heat, cc19=ruin, cc20=swarm, cc1=mod wheel, cc64=sustain, aftertouch=expressive, program change 0..7=live slot"
            );
        }
        println!(
            "midi trace: {}",
            if self.trace_midi {
                "enabled"
            } else {
                "disabled"
            }
        );
        println!("midi startup guard: {} ms", MIDI_STARTUP_GUARD.as_millis());
        println!("interactive controls active; type `help` for commands");
    }

    pub(crate) fn command_loop(&mut self) -> Result<()> {
        self.print_startup_summary();
        self.print_status()?;

        let stdin = io::stdin();
        let mut lines = stdin.lock().lines();

        loop {
            self.print_runtime_control_messages()?;
            print!("epm1> ");
            io::stdout().flush().context("failed to flush prompt")?;

            let Some(line_result) = lines.next() else {
                println!();
                println!("stdin closed; stopping session");
                break;
            };

            let line = line_result.context("failed to read command input")?;
            match parse_runtime_ui_command(&line) {
                Ok(RuntimeUiCommand::Noop) => continue,
                Ok(RuntimeUiCommand::Help) => print_runtime_help(),
                Ok(RuntimeUiCommand::Status) => self.print_status()?,
                Ok(RuntimeUiCommand::Patches) => list_factory_patches()?,
                Ok(RuntimeUiCommand::Favorites) => list_favorite_patches()?,
                Ok(RuntimeUiCommand::Favorite(slot)) => {
                    self.load_favorite_slot(slot)?;
                    self.print_status()?;
                }
                Ok(RuntimeUiCommand::Patch(argument)) => {
                    let path = resolve_patch_argument(Some(argument.as_str()))?;
                    self.switch_patch(path)?;
                    self.print_status()?;
                }
                Ok(RuntimeUiCommand::NextFavorite) => {
                    self.switch_favorite(1)?;
                    self.print_status()?;
                }
                Ok(RuntimeUiCommand::PrevFavorite) => {
                    self.switch_favorite(-1)?;
                    self.print_status()?;
                }
                Ok(RuntimeUiCommand::DemoPatch) => {
                    self.switch_patch(default_patch_path())?;
                    self.print_status()?;
                }
                Ok(RuntimeUiCommand::Macro(id, value)) => {
                    self.set_macro(id, value)?;
                    println!("macro {} -> {:.3}", macro_display_name(id), value);
                }
                Ok(RuntimeUiCommand::Panic) => {
                    self.panic()?;
                    self.print_status()?;
                }
                Ok(RuntimeUiCommand::ResetControllers) => {
                    self.reset_controllers()?;
                    self.print_status()?;
                }
                Ok(RuntimeUiCommand::BcsLayer(scenario)) => {
                    self.set_bcs_layer_scenario(scenario)?;
                    self.print_status()?;
                }
                Ok(RuntimeUiCommand::Record { seconds, path }) => {
                    let path = self.start_output_recording(seconds, path)?;
                    println!("recording {seconds}s -> {}", path.display());
                    println!(
                        "midi log -> {}",
                        output_recording_midi_log_path(&path).display()
                    );
                }
                Ok(RuntimeUiCommand::RecordStop) => match self.stop_output_recording()? {
                    Some(path) => println!(
                        "recording stop requested -> {} (midi log {})",
                        path.display(),
                        output_recording_midi_log_path(&path).display()
                    ),
                    None => println!("no active output recording"),
                },
                Ok(RuntimeUiCommand::AudioList) => list_audio_devices()?,
                Ok(RuntimeUiCommand::AudioSelect(selector)) => {
                    self.switch_audio(selector)?;
                    self.print_status()?;
                }
                Ok(RuntimeUiCommand::MidiList) => list_midi_devices()?,
                Ok(RuntimeUiCommand::MidiSelect(selector)) => {
                    self.switch_midi(selector)?;
                    self.print_status()?;
                }
                Ok(RuntimeUiCommand::Demo) => {
                    self.enable_demo();
                    self.print_status()?;
                }
                Ok(RuntimeUiCommand::Quit) => break,
                Err(error) => eprintln!("command error: {error}"),
            }
        }

        Ok(())
    }

    pub(crate) fn block_forever(&mut self) -> Result<()> {
        loop {
            self.print_runtime_control_messages()?;
            thread::sleep(Duration::from_millis(50));
        }
    }

    pub(crate) fn switch_patch(&mut self, path: PathBuf) -> Result<()> {
        let keep_demo = self.driver.is_demo();
        let runtime = build_audio_runtime(
            &path,
            self.audio_selector.as_deref(),
            self.sample_rate_hz,
            self.alsa_tuning,
            self.gfm_layer_seed,
            self.bcs_layer_scenario,
            Arc::clone(&self.input_metrics),
            Arc::clone(&self.recording_metrics),
        )?;
        self.install_runtime(path.clone(), runtime, keep_demo)?;
        println!("loaded patch: {} ({})", self.patch_name, path.display());
        Ok(())
    }

    pub(crate) fn set_macro(&self, id: MacroId, value: f32) -> Result<()> {
        self.tx
            .send(EngineCommand::Controller(ControllerEvent::Macro {
                id,
                value: value.clamp(0.0, 1.0),
            }))
            .map_err(|_| anyhow!("audio runtime is no longer available"))
    }

    pub(crate) fn set_direct_param(&self, id: ParamId, value: f32) -> Result<()> {
        let spec = param_spec(id);
        self.tx
            .send(EngineCommand::Controller(ControllerEvent::DirectParam {
                id,
                value: value.clamp(spec.min, spec.max),
            }))
            .map_err(|_| anyhow!("audio runtime is no longer available"))
    }

    pub(crate) fn panic(&self) -> Result<()> {
        self.priority_actions.request_panic();
        Ok(())
    }

    pub(crate) fn reset_controllers(&self) -> Result<()> {
        self.priority_actions.request_reset_controllers();
        Ok(())
    }

    pub(crate) fn toggle_param(&self, id: ParamId) -> Result<f32> {
        let snapshot = self.request_snapshot()?;
        let current = match id {
            ParamId::ChorusEnabled => snapshot.direct.chorus_enabled,
            ParamId::ReverbEnabled => snapshot.direct.reverb_enabled,
            _ => {
                return Err(anyhow!(
                    "runtime toggle is only supported for chorus_enabled and reverb_enabled"
                ));
            }
        };
        let value = if current { 0.0 } else { 1.0 };
        self.tx
            .send(EngineCommand::Controller(ControllerEvent::DirectParam {
                id,
                value,
            }))
            .map_err(|_| anyhow!("audio runtime is no longer available"))?;
        Ok(value)
    }

    pub(crate) fn print_status(&self) -> Result<()> {
        let snapshot = self.request_snapshot()?;
        let description = snapshot
            .patch_description
            .as_deref()
            .unwrap_or("no description");
        let tags = if snapshot.patch_tags.is_empty() {
            "none".to_string()
        } else {
            snapshot.patch_tags.join(", ")
        };

        println!("patch: {} - {}", snapshot.patch_name, description);
        println!(
            "favorite: {}",
            if snapshot.patch_favorite { "yes" } else { "no" }
        );
        println!(
            "live slot: {}",
            self.current_live_slot()
                .map(|slot| slot.to_string())
                .unwrap_or_else(|| "-".to_string())
        );
        println!(
            "audio: {} @ {} Hz, {} channels",
            self.audio_device_name, self.sample_rate_hz, self.channels
        );
        println!(
            "alsa: selector={} period={} buffer={} start_threshold={}",
            self.audio_selector.as_deref().unwrap_or("unset"),
            self.alsa_tuning.period_frames,
            self.alsa_tuning.buffer_frames,
            self.alsa_tuning.start_threshold_frames
        );
        println!("mode: {}", self.driver.detail());
        println!("{}", gfm_layer_status_line(&snapshot));
        println!("{}", bcs_layer_status_line(&snapshot));
        println!("tags: {tags}");
        let transport = self.transport_metrics.snapshot();
        let input = self.input_metrics.snapshot();
        println!(
            "voices: active={} sustain={} held={:?} peak={:.3} clip={}",
            snapshot.active_voice_count,
            snapshot.sustain_down,
            snapshot.held_notes,
            snapshot.peak_output,
            snapshot.clip_detected
        );
        println!(
            "midi activity: messages={} accepted={} midi_dropped={} runtime_dropped={} trace_dropped={} controllers_coalesced={}",
            input.midi_messages,
            input.midi_messages_accepted,
            input.midi_messages_dropped,
            input.runtime_controls_dropped,
            input.trace_records_dropped,
            input.controllers_coalesced
        );
        println!(
            "transport: queued={} target={} write_hint={} underrun_batches={} underrun_frames={} xrun_recoveries={} overflow_batches={} overflow_frames={}",
            transport.queued_frames,
            transport.queue_target_frames,
            transport.write_frames_hint,
            transport.underrun_batches,
            transport.underrun_frames,
            transport.xrun_recoveries,
            transport.overflow_batches,
            transport.overflow_frames
        );
        let recording = self.recording_metrics_snapshot();
        println!(
            "recording: {} path={} written_frames={} dropped_frames={} target_frames={}{}",
            recording.state.label(),
            recording
                .path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "-".to_string()),
            recording.frames_written,
            recording.frames_dropped,
            recording
                .target_frames
                .map(|frames| frames.to_string())
                .unwrap_or_else(|| "-".to_string()),
            recording
                .error
                .as_ref()
                .map(|error| format!(" error={error}"))
                .unwrap_or_default()
        );
        println!(
            "macros: gravitacija={:.3} bloom={:.3} heat={:.3} ruin={:.3} swarm={:.3}",
            snapshot.effective_macros.gravitacija,
            snapshot.effective_macros.bloom,
            snapshot.effective_macros.heat,
            snapshot.effective_macros.ruin,
            snapshot.effective_macros.swarm
        );
        println!(
            "identity: horizont_open={:.3} pec_mass={:.3} baklja_ready={:.3} grav_pull={:.3}",
            snapshot.identity.horizont_open,
            snapshot.identity.pec_mass,
            snapshot.identity.baklja_ready,
            snapshot.identity.grav_pull
        );
        println!(
            "derived: mass={:.3} strain={:.3} headroom={:.3} threshold={:.3}",
            snapshot.derived.mass,
            snapshot.derived.strain,
            snapshot.derived.headroom,
            snapshot.derived.rupture_threshold
        );
        Ok(())
    }

    pub(crate) fn request_snapshot(&self) -> Result<EngineSnapshot> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(EngineCommand::RequestSnapshot(reply_tx))
            .map_err(|_| anyhow!("audio runtime is no longer available"))?;
        reply_rx
            .recv_timeout(Duration::from_millis(500))
            .map_err(|_| anyhow!("timed out waiting for engine snapshot"))
    }

    pub(crate) fn request_patch(&self) -> Result<PatchFileV1> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(EngineCommand::RequestPatch(reply_tx))
            .map_err(|_| anyhow!("audio runtime is no longer available"))?;
        reply_rx
            .recv_timeout(Duration::from_millis(500))
            .map_err(|_| anyhow!("timed out waiting for engine patch"))
    }

    pub(crate) fn export_sound_lab_patch(&self, requested_name: &str) -> Result<PathBuf> {
        let mut patch = self.request_patch()?;
        let snapshot = self.request_snapshot()?;
        let original_name = patch.meta.patch_name.clone();
        let patch_name = if requested_name.trim().is_empty() {
            format!("{original_name} Lab")
        } else {
            requested_name.trim().to_string()
        };
        patch.meta.patch_name = patch_name.clone();
        patch.x = Some(sound_lab_extension_table(
            patch.x.take().unwrap_or_default(),
            &snapshot,
            &self.patch_path,
        ));

        validate_patch_v1(&patch).context("exported Sound Lab patch failed validation")?;
        let serialized = save_patch_toml(&patch).context("failed to serialize Sound Lab patch")?;
        let dir = user_patch_dir();
        fs::create_dir_all(&dir)
            .with_context(|| format!("failed to create user patch directory {}", dir.display()))?;

        let filename = generated_user_patch_filename(&patch_name);
        let stem = filename
            .strip_suffix(".toml")
            .unwrap_or(filename.as_str())
            .to_string();
        for index in 0..1000 {
            let candidate = if index == 0 {
                dir.join(&filename)
            } else {
                dir.join(format!("{stem}-{index}.toml"))
            };
            match File::options()
                .write(true)
                .create_new(true)
                .open(&candidate)
            {
                Ok(mut file) => {
                    file.write_all(serialized.as_bytes()).with_context(|| {
                        format!("failed to write Sound Lab patch {}", candidate.display())
                    })?;
                    return Ok(candidate);
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(anyhow!(
                        "failed to create Sound Lab patch {}: {error}",
                        candidate.display()
                    ));
                }
            }
        }

        Err(anyhow!(
            "failed to find a free Sound Lab patch filename for `{patch_name}`"
        ))
    }

    pub(crate) fn start_output_recording(
        &self,
        seconds: u64,
        path: Option<PathBuf>,
    ) -> Result<PathBuf> {
        if seconds == 0 {
            return Err(anyhow!("record duration must be greater than zero seconds"));
        }
        let sample_rate = u64::from(self.sample_rate_hz);
        let target_frames = seconds
            .checked_mul(sample_rate)
            .and_then(|frames| usize::try_from(frames).ok())
            .ok_or_else(|| anyhow!("record duration is too large"))?;
        let path = match path {
            Some(path) => path,
            None => default_output_capture_path()?,
        };
        let midi_log_path = self.start_midi_trace_recording_log(&path, Some(target_frames))?;
        let request = OutputRecordingRequest {
            path,
            sample_rate_hz: self.sample_rate_hz,
            max_frames: Some(target_frames),
        };
        let (reply_tx, reply_rx) = mpsc::channel();
        if self
            .tx
            .send(EngineCommand::StartOutputRecording(request, reply_tx))
            .is_err()
        {
            self.midi_trace_log.abort_and_remove();
            return Err(anyhow!("audio runtime is no longer available"));
        }
        match reply_rx.recv_timeout(RECORDING_REPLY_TIMEOUT) {
            Ok(Ok(path)) => Ok(path),
            Ok(Err(error)) => {
                self.midi_trace_log.abort_and_remove();
                Err(anyhow!(error))
            }
            Err(_) => {
                self.midi_trace_log.abort_and_remove();
                Err(anyhow!(
                    "timed out starting output recording after creating MIDI log {}",
                    midi_log_path.display()
                ))
            }
        }
    }

    pub(crate) fn start_tagged_output_recording(&self, seconds: u64, tag: &str) -> Result<PathBuf> {
        let path = tagged_output_capture_path(&self.patch_path, tag, seconds)?;
        self.start_output_recording(seconds, Some(path))
    }

    pub(crate) fn stop_output_recording(&self) -> Result<Option<PathBuf>> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(EngineCommand::StopOutputRecording(reply_tx))
            .map_err(|_| anyhow!("audio runtime is no longer available"))?;
        let path = reply_rx
            .recv_timeout(RECORDING_REPLY_TIMEOUT)
            .map_err(|_| anyhow!("timed out stopping output recording"))?
            .map_err(|error| anyhow!(error))?;
        self.midi_trace_log.finish_with_trace_drops(
            "record-stop requested",
            self.input_metrics.snapshot().trace_records_dropped,
        );
        Ok(path)
    }

    pub(crate) fn start_midi_trace_recording_log(
        &self,
        wav_path: &Path,
        max_frames: Option<usize>,
    ) -> Result<PathBuf> {
        let log_path = output_recording_midi_log_path(wav_path);
        let controller_profile = self
            .controller_profile
            .as_ref()
            .map(|profile| format!("{} ({})", profile.name, profile.path.display()));
        let started = self
            .midi_trace_log
            .start(MidiTraceLogStart {
                wav_path: wav_path.to_path_buf(),
                log_path: log_path.clone(),
                patch_path: self.patch_path.clone(),
                patch_name: self.patch_name.clone(),
                sample_rate_hz: self.sample_rate_hz,
                max_frames,
                midi_channel: self.midi_channel,
                controller_profile,
            })
            .with_context(|| {
                format!("failed to create MIDI trace sidecar {}", log_path.display())
            })?;
        if let Some(max_frames) = max_frames {
            let midi_trace_log = Arc::clone(&self.midi_trace_log);
            let input_metrics = Arc::clone(&self.input_metrics);
            let generation = started.generation;
            let seconds = max_frames as f64 / f64::from(self.sample_rate_hz.max(1));
            thread::spawn(move || {
                thread::sleep(Duration::from_secs_f64(seconds) + MIDI_ACTIVITY_FLASH);
                midi_trace_log.finish_generation_with_trace_drops(
                    generation,
                    "recording target duration elapsed",
                    input_metrics.snapshot().trace_records_dropped,
                );
            });
        }
        Ok(started.path)
    }

    pub(crate) fn reconcile_midi_trace_recording_log(&self, recording: &RecordingMetricsSnapshot) {
        if recording.state != RecordingState::Active {
            self.midi_trace_log.finish_with_trace_drops(
                &format!("recording state {}", recording.state.label()),
                self.input_metrics.snapshot().trace_records_dropped,
            );
        }
    }

    pub(crate) fn set_gfm_layer_seed(
        &mut self,
        seed: Option<u64>,
    ) -> Result<GfmVoiceProgramSelection> {
        if self.gfm_layer_seed == seed {
            return Ok(self.request_snapshot()?.gfm_layer.selection);
        }

        let mode = gfm_layer_mode_from_seed(seed);
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(EngineCommand::SetGfmLayerMode(mode, reply_tx))
            .map_err(|_| anyhow!("audio runtime is no longer available"))?;
        let selection = reply_rx
            .recv_timeout(Duration::from_millis(500))
            .map_err(|_| anyhow!("timed out applying GFM layer mode"))?
            .map_err(|error| anyhow!(error))?;

        self.gfm_layer_seed = seed;
        println!(
            "gfm layer {}",
            seed.map(format_gfm_seed)
                .unwrap_or_else(|| "disabled".to_string())
        );
        Ok(selection)
    }

    pub(crate) fn set_bcs_layer_scenario(
        &mut self,
        scenario: Option<BcsScenario>,
    ) -> Result<BcsLayerSnapshot> {
        if self.bcs_layer_scenario == scenario {
            return Ok(self.request_snapshot()?.bcs_layer);
        }

        let mode = bcs_layer_mode_from_scenario(scenario);
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(EngineCommand::SetBcsLayerMode(mode, reply_tx))
            .map_err(|_| anyhow!("audio runtime is no longer available"))?;
        let snapshot = reply_rx
            .recv_timeout(Duration::from_millis(500))
            .map_err(|_| anyhow!("timed out applying BCS layer mode"))?
            .map_err(|error| anyhow!(error))?;

        self.bcs_layer_scenario = scenario;
        println!(
            "bcs layer {}",
            scenario.map(format_bcs_scenario).unwrap_or("disabled")
        );
        Ok(snapshot)
    }

    pub(crate) fn switch_audio(&mut self, selector: String) -> Result<()> {
        let keep_demo = self.driver.is_demo();
        let runtime = build_audio_runtime(
            &self.patch_path,
            Some(selector.as_str()),
            self.sample_rate_hz,
            self.alsa_tuning,
            self.gfm_layer_seed,
            self.bcs_layer_scenario,
            Arc::clone(&self.input_metrics),
            Arc::clone(&self.recording_metrics),
        )?;
        self.install_runtime(self.patch_path.clone(), runtime, keep_demo)?;
        println!(
            "audio switched to `{}`",
            self.audio_selector.as_deref().unwrap_or(selector.as_str())
        );
        Ok(())
    }

    pub(crate) fn switch_midi(&mut self, selector: String) -> Result<()> {
        if self.midi_selector.as_deref() == Some(selector.as_str())
            && matches!(self.driver, PerformanceDriver::Midi(_))
        {
            println!("midi already switched to `{selector}`");
            return Ok(());
        }

        let mut old_driver = std::mem::replace(&mut self.driver, PerformanceDriver::Idle);
        let old_mode_was_demo = old_driver.is_demo();
        let old_midi_selector = self.midi_selector.clone();
        old_driver.stop();
        drop(old_driver);

        let connection = match open_midi_input(
            Arc::clone(&self.midi_input_queue),
            self.bend_range,
            Some(selector.as_str()),
            self.midi_channel,
            self.controller_profile.clone(),
            self.trace_midi,
            Arc::clone(&self.runtime_control_queue),
            Arc::clone(&self.priority_actions),
            Arc::clone(&self.input_metrics),
            Arc::clone(&self.midi_trace_log),
            self.midi_trace_worker.publisher(),
            Arc::clone(&self.sound_lab_midi_focus),
        ) {
            Ok(Some(connection)) => connection,
            Ok(None) => {
                self.driver = restore_driver_state(
                    self.tx.clone(),
                    Arc::clone(&self.midi_input_queue),
                    self.bend_range,
                    old_midi_selector.as_deref(),
                    self.midi_channel,
                    self.controller_profile.clone(),
                    self.trace_midi,
                    Arc::clone(&self.runtime_control_queue),
                    Arc::clone(&self.priority_actions),
                    Arc::clone(&self.input_metrics),
                    Arc::clone(&self.midi_trace_log),
                    self.midi_trace_worker.publisher(),
                    Arc::clone(&self.sound_lab_midi_focus),
                    old_mode_was_demo,
                );
                return Err(anyhow!("no MIDI input devices available"));
            }
            Err(error) => {
                self.driver = restore_driver_state(
                    self.tx.clone(),
                    Arc::clone(&self.midi_input_queue),
                    self.bend_range,
                    old_midi_selector.as_deref(),
                    self.midi_channel,
                    self.controller_profile.clone(),
                    self.trace_midi,
                    Arc::clone(&self.runtime_control_queue),
                    Arc::clone(&self.priority_actions),
                    Arc::clone(&self.input_metrics),
                    Arc::clone(&self.midi_trace_log),
                    self.midi_trace_worker.publisher(),
                    Arc::clone(&self.sound_lab_midi_focus),
                    old_mode_was_demo,
                );
                return Err(error);
            }
        };

        self.driver = PerformanceDriver::Midi(connection);
        self.midi_selector = Some(selector.clone());
        println!("midi switched to `{selector}`");
        Ok(())
    }

    pub(crate) fn enable_demo(&mut self) {
        let mut old_driver = std::mem::replace(&mut self.driver, PerformanceDriver::Idle);
        old_driver.stop();
        self.restore_demo_driver();
        println!("demo performer enabled");
    }

    pub(crate) fn switch_favorite(&mut self, direction: isize) -> Result<()> {
        let path = adjacent_live_patch(&self.patch_path, direction)?;
        self.switch_patch(path)
    }

    pub(crate) fn load_favorite_slot(&mut self, slot: usize) -> Result<()> {
        let path = live_patch_path(slot)?;
        self.switch_patch(path)
    }

    pub(crate) fn install_runtime(
        &mut self,
        patch_path: PathBuf,
        prepared_runtime: PreparedAudioRuntime,
        keep_demo: bool,
    ) -> Result<()> {
        self.midi_trace_log.finish_with_trace_drops(
            "runtime replaced while recording",
            self.input_metrics.snapshot().trace_records_dropped,
        );
        let same_hw_device = same_hw_selector(
            self.audio_selector.as_deref(),
            &prepared_runtime.audio_selector,
        );
        let mut old_driver = std::mem::replace(&mut self.driver, PerformanceDriver::Idle);
        let old_mode_was_demo = old_driver.is_demo();
        let old_midi_selector = self.midi_selector.clone();
        let old_bend_range = self.bend_range;
        old_driver.stop();
        drop(old_driver);

        if same_hw_device {
            if let Some(old_stream) = self.stream.take() {
                old_stream.shutdown();
            }
        }

        let runtime = match start_prepared_audio_runtime(prepared_runtime) {
            Ok(runtime) => runtime,
            Err(error) => {
                if same_hw_device {
                    if let Err(rollback_error) =
                        self.rebuild_current_runtime_after_stream_release(old_mode_was_demo)
                    {
                        return Err(anyhow!(
                            "{error}; rollback to previous runtime also failed: {rollback_error}"
                        ));
                    }
                } else {
                    self.driver = restore_driver_state(
                        self.tx.clone(),
                        Arc::clone(&self.midi_input_queue),
                        old_bend_range,
                        old_midi_selector.as_deref(),
                        self.midi_channel,
                        self.controller_profile.clone(),
                        self.trace_midi,
                        Arc::clone(&self.runtime_control_queue),
                        Arc::clone(&self.priority_actions),
                        Arc::clone(&self.input_metrics),
                        Arc::clone(&self.midi_trace_log),
                        self.midi_trace_worker.publisher(),
                        Arc::clone(&self.sound_lab_midi_focus),
                        old_mode_was_demo,
                    );
                }
                return Err(error);
            }
        };

        let AudioRuntime {
            tx,
            worker,
            stream: replacement_stream,
            audio_selector: resolved_audio_selector,
            audio_device_name,
            sample_rate_hz,
            channels,
            alsa_tuning,
            bend_range,
            patch_name,
            midi_input_queue,
            priority_actions,
            transport_metrics,
        } = runtime;

        let replacement_driver = match self.open_driver_for_tx(
            tx.clone(),
            Arc::clone(&midi_input_queue),
            bend_range,
            Arc::clone(&self.runtime_control_queue),
            Arc::clone(&priority_actions),
            Arc::clone(&self.input_metrics),
            self.midi_trace_worker.publisher(),
            keep_demo,
        ) {
            Ok(driver) => driver,
            Err(error) => {
                replacement_stream.shutdown();
                worker.shutdown(tx);
                if same_hw_device && self.stream.is_none() {
                    if let Err(rollback_error) =
                        self.rebuild_current_runtime_after_stream_release(old_mode_was_demo)
                    {
                        return Err(anyhow!(
                            "{error}; rollback to previous runtime also failed: {rollback_error}"
                        ));
                    }
                    return Err(error);
                }
                self.driver = restore_driver_state(
                    self.tx.clone(),
                    Arc::clone(&self.midi_input_queue),
                    old_bend_range,
                    old_midi_selector.as_deref(),
                    self.midi_channel,
                    self.controller_profile.clone(),
                    self.trace_midi,
                    Arc::clone(&self.runtime_control_queue),
                    Arc::clone(&self.priority_actions),
                    Arc::clone(&self.input_metrics),
                    Arc::clone(&self.midi_trace_log),
                    self.midi_trace_worker.publisher(),
                    Arc::clone(&self.sound_lab_midi_focus),
                    old_mode_was_demo,
                );
                return Err(error);
            }
        };

        let old_stream = self.stream.replace(replacement_stream);
        let old_tx = std::mem::replace(&mut self.tx, tx);
        let old_worker = std::mem::replace(&mut self.worker, worker);
        self.driver = replacement_driver;
        if let Some(old_stream) = old_stream {
            old_stream.shutdown();
        }
        old_worker.shutdown(old_tx);

        self.patch_path = patch_path;
        self.patch_name = patch_name;
        self.audio_selector = Some(resolved_audio_selector);
        self.alsa_tuning = alsa_tuning;
        self.audio_device_name = audio_device_name;
        self.sample_rate_hz = sample_rate_hz;
        self.channels = channels;
        self.bend_range = bend_range;
        self.midi_input_queue = midi_input_queue;
        self.priority_actions = priority_actions;
        self.transport_metrics = transport_metrics;
        Ok(())
    }

    pub(crate) fn rebuild_current_runtime_after_stream_release(
        &mut self,
        old_mode_was_demo: bool,
    ) -> Result<()> {
        let prepared = build_audio_runtime(
            &self.patch_path,
            self.audio_selector.as_deref(),
            self.sample_rate_hz,
            self.alsa_tuning,
            self.gfm_layer_seed,
            self.bcs_layer_scenario,
            Arc::clone(&self.input_metrics),
            Arc::clone(&self.recording_metrics),
        )
        .context("failed to rebuild previous runtime")?;
        let runtime = start_prepared_audio_runtime(prepared)
            .context("failed to restart previous ALSA stream")?;
        let AudioRuntime {
            tx,
            worker,
            stream,
            audio_selector,
            audio_device_name,
            sample_rate_hz,
            channels,
            alsa_tuning,
            bend_range,
            patch_name,
            midi_input_queue,
            priority_actions,
            transport_metrics,
        } = runtime;

        let old_tx = std::mem::replace(&mut self.tx, tx.clone());
        let old_worker = std::mem::replace(&mut self.worker, worker);
        old_worker.shutdown(old_tx);
        self.stream = Some(stream);
        self.patch_name = patch_name;
        self.audio_selector = Some(audio_selector);
        self.alsa_tuning = alsa_tuning;
        self.audio_device_name = audio_device_name;
        self.sample_rate_hz = sample_rate_hz;
        self.channels = channels;
        self.bend_range = bend_range;
        self.midi_input_queue = midi_input_queue;
        self.priority_actions = priority_actions;
        self.transport_metrics = transport_metrics;
        self.driver = restore_driver_state(
            self.tx.clone(),
            Arc::clone(&self.midi_input_queue),
            self.bend_range,
            self.midi_selector.as_deref(),
            self.midi_channel,
            self.controller_profile.clone(),
            self.trace_midi,
            Arc::clone(&self.runtime_control_queue),
            Arc::clone(&self.priority_actions),
            Arc::clone(&self.input_metrics),
            Arc::clone(&self.midi_trace_log),
            self.midi_trace_worker.publisher(),
            Arc::clone(&self.sound_lab_midi_focus),
            old_mode_was_demo,
        );
        Ok(())
    }

    pub(crate) fn current_live_slot(&self) -> Option<usize> {
        live_slot_for_path(&self.patch_path)
    }

    pub(crate) fn input_metrics_snapshot(&self) -> InputMetricsSnapshot {
        self.input_metrics.snapshot()
    }

    pub(crate) fn transport_metrics_snapshot(&self) -> TransportMetricsSnapshot {
        self.transport_metrics.snapshot()
    }

    pub(crate) fn recording_metrics_snapshot(&self) -> RecordingMetricsSnapshot {
        let snapshot = self.recording_metrics.snapshot();
        self.reconcile_midi_trace_recording_log(&snapshot);
        snapshot
    }

    pub(crate) fn print_runtime_control_messages(&mut self) -> Result<()> {
        let recording = self.recording_metrics.snapshot();
        self.reconcile_midi_trace_recording_log(&recording);
        for message in self.poll_runtime_control_messages()? {
            println!("{message}");
        }
        Ok(())
    }

    pub(crate) fn poll_runtime_control_messages(&mut self) -> Result<Vec<String>> {
        let mut messages = Vec::new();
        while let Some(message) = self.runtime_control_queue.pop() {
            match message {
                RuntimeControlMessage::ProgramChange(slot) => {
                    let slot_index = usize::from(slot);
                    match self.load_favorite_slot(slot_index) {
                        Ok(()) => messages.push(format!(
                            "program change -> live slot {slot_index} ({})",
                            self.patch_name
                        )),
                        Err(error) => {
                            messages.push(format!("program change {slot_index} ignored: {error}"))
                        }
                    }
                }
                RuntimeControlMessage::Panic => {
                    self.panic()?;
                    messages.push("controller action -> panic".to_string());
                }
                RuntimeControlMessage::ResetControllers => {
                    self.reset_controllers()?;
                    messages.push("controller action -> reset controllers".to_string());
                }
                RuntimeControlMessage::NextFavorite => match self.switch_favorite(1) {
                    Ok(()) => {
                        messages.push(format!("controller action -> next ({})", self.patch_name))
                    }
                    Err(error) => messages.push(format!("next favorite ignored: {error}")),
                },
                RuntimeControlMessage::PrevFavorite => match self.switch_favorite(-1) {
                    Ok(()) => {
                        messages.push(format!("controller action -> prev ({})", self.patch_name))
                    }
                    Err(error) => messages.push(format!("prev favorite ignored: {error}")),
                },
                RuntimeControlMessage::FavoriteSlot(slot) => match self.load_favorite_slot(slot) {
                    Ok(()) => messages.push(format!(
                        "controller action -> live slot {slot} ({})",
                        self.patch_name
                    )),
                    Err(error) => messages.push(format!("favorite slot {slot} ignored: {error}")),
                },
                RuntimeControlMessage::ToggleParam(id) => match self.toggle_param(id) {
                    Ok(value) => messages.push(format!(
                        "controller action -> toggle {} {}",
                        param_spec(id).name,
                        if value >= 0.5 { "on" } else { "off" }
                    )),
                    Err(error) => {
                        messages.push(format!("toggle {} ignored: {error}", param_spec(id).name))
                    }
                },
            }
        }
        Ok(messages)
    }
}

pub(crate) fn sound_lab_extension_table(
    mut extensions: toml::Table,
    snapshot: &EngineSnapshot,
    source_patch_path: &Path,
) -> toml::Table {
    let mut sound_lab = toml::Table::new();
    sound_lab.insert(
        "intent_version".to_string(),
        toml::Value::String("1".to_string()),
    );
    sound_lab.insert(
        "note".to_string(),
        toml::Value::String(
            "Runtime layer intent only; GFM/BCS state is not auto-loaded from this patch."
                .to_string(),
        ),
    );
    sound_lab.insert(
        "source_patch_path".to_string(),
        toml::Value::String(source_patch_path.display().to_string()),
    );

    match snapshot.gfm_layer.mode {
        GfmLayerMode::Enabled { seed } => {
            sound_lab.insert("gfm_enabled".to_string(), toml::Value::Boolean(true));
            sound_lab.insert(
                "gfm_seed".to_string(),
                toml::Value::String(format_gfm_seed(seed)),
            );
        }
        GfmLayerMode::Disabled => {
            sound_lab.insert("gfm_enabled".to_string(), toml::Value::Boolean(false));
            sound_lab.insert(
                "gfm_seed".to_string(),
                toml::Value::String("disabled".to_string()),
            );
        }
    }

    match snapshot.bcs_layer.mode {
        BcsLayerMode::Enabled { scenario } => {
            sound_lab.insert("bcs_enabled".to_string(), toml::Value::Boolean(true));
            sound_lab.insert(
                "bcs_scenario".to_string(),
                toml::Value::String(format_bcs_scenario(scenario).to_string()),
            );
        }
        BcsLayerMode::Disabled => {
            sound_lab.insert("bcs_enabled".to_string(), toml::Value::Boolean(false));
            sound_lab.insert(
                "bcs_scenario".to_string(),
                toml::Value::String("disabled".to_string()),
            );
        }
    }
    sound_lab.insert(
        "bcs_gain".to_string(),
        toml::Value::Float(snapshot.bcs_layer.gain as f64),
    );
    sound_lab.insert(
        "bcs_effective_gain".to_string(),
        toml::Value::Float(snapshot.bcs_layer.effective_gain as f64),
    );

    extensions.insert("sound_lab".to_string(), toml::Value::Table(sound_lab));
    extensions
}
