use super::*;

impl RuntimeSession {
    pub fn print_status(&self) -> Result<()> {
        if !self.terminal_output_enabled {
            return Ok(());
        }

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
        println!("{}", mozaik_status_line(&snapshot));
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

    pub fn request_snapshot(&self) -> Result<EngineSnapshot> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(EngineCommand::RequestSnapshot(reply_tx))
            .map_err(|_| anyhow!("audio runtime is no longer available"))?;
        reply_rx
            .recv_timeout(Duration::from_millis(500))
            .map_err(|_| anyhow!("timed out waiting for engine snapshot"))
    }

    pub fn request_patch(&self) -> Result<PatchFileV1> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(EngineCommand::RequestPatch(reply_tx))
            .map_err(|_| anyhow!("audio runtime is no longer available"))?;
        reply_rx
            .recv_timeout(Duration::from_millis(500))
            .map_err(|_| anyhow!("timed out waiting for engine patch"))
    }

    pub fn export_sound_lab_patch(&self, requested_name: &str) -> Result<PathBuf> {
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

    pub fn start_output_recording(&self, seconds: u64, path: Option<PathBuf>) -> Result<PathBuf> {
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

    pub fn start_tagged_output_recording(&self, seconds: u64, tag: &str) -> Result<PathBuf> {
        let path = tagged_output_capture_path(&self.patch_path, tag, seconds)?;
        self.start_output_recording(seconds, Some(path))
    }

    pub fn stop_output_recording(&self) -> Result<Option<PathBuf>> {
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

    pub fn start_midi_trace_recording_log(
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

    pub fn reconcile_midi_trace_recording_log(&self, recording: &RecordingMetricsSnapshot) {
        if recording.state != RecordingState::Active {
            self.midi_trace_log.finish_with_trace_drops(
                &format!("recording state {}", recording.state.label()),
                self.input_metrics.snapshot().trace_records_dropped,
            );
        }
    }
}
