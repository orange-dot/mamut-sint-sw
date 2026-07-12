use super::*;

impl RuntimeSession {
    pub fn install_runtime(
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
        let scope_was_enabled = self.scope_enabled.load(Ordering::Relaxed);
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
            scope_consumer,
            scope_enabled,
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
        self.scope_consumer = scope_consumer;
        self.scope_enabled = scope_enabled;
        self.set_scope_enabled(scope_was_enabled);
        Ok(())
    }

    pub fn rebuild_current_runtime_after_stream_release(
        &mut self,
        old_mode_was_demo: bool,
    ) -> Result<()> {
        let scope_was_enabled = self.scope_enabled.load(Ordering::Relaxed);
        let prepared = build_audio_runtime(
            &self.patch_path,
            self.audio_selector.as_deref(),
            self.sample_rate_hz,
            self.alsa_tuning,
            self.gfm_layer_seed,
            self.bcs_layer_scenario,
            self.mozaik_seed,
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
            scope_consumer,
            scope_enabled,
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
        self.scope_consumer = scope_consumer;
        self.scope_enabled = scope_enabled;
        self.set_scope_enabled(scope_was_enabled);
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

    pub fn current_live_slot(&self) -> Option<usize> {
        live_slot_for_path(&self.patch_path)
    }

    pub fn input_metrics_snapshot(&self) -> InputMetricsSnapshot {
        self.input_metrics.snapshot()
    }

    pub fn transport_metrics_snapshot(&self) -> TransportMetricsSnapshot {
        self.transport_metrics.snapshot()
    }

    pub fn recording_metrics_snapshot(&self) -> RecordingMetricsSnapshot {
        let snapshot = self.recording_metrics.snapshot();
        self.reconcile_midi_trace_recording_log(&snapshot);
        snapshot
    }

    pub fn drain_scope_frames(&mut self, output: &mut Vec<StereoFrame>) -> usize {
        let initial_len = output.len();
        while let Ok(frame) = self.scope_consumer.pop() {
            output.push(frame);
        }
        output.len().saturating_sub(initial_len)
    }

    pub fn print_runtime_control_messages(&mut self) -> Result<()> {
        let recording = self.recording_metrics.snapshot();
        self.reconcile_midi_trace_recording_log(&recording);
        for message in self.poll_runtime_control_messages()? {
            if self.terminal_output_enabled {
                println!("{message}");
            }
        }
        Ok(())
    }

    pub fn poll_runtime_control_messages(&mut self) -> Result<Vec<String>> {
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
