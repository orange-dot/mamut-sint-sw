use super::*;

impl RuntimeSession {
    pub fn new(options: &PlayOptions) -> Result<Self> {
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
            terminal_output_enabled: true,
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
            scope_consumer: runtime.scope_consumer,
            scope_enabled: runtime.scope_enabled,
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

    #[allow(clippy::too_many_arguments)]
    pub fn open_driver_for_tx(
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

    pub fn set_sound_lab_midi_focus(&self, page: Option<SoundLabPage>) {
        self.sound_lab_midi_focus.set(page);
    }

    pub fn set_terminal_output_enabled(&mut self, enabled: bool) {
        self.terminal_output_enabled = enabled;
        self.midi_trace_worker
            .set_terminal_trace_enabled(enabled && self.trace_midi);
    }

    pub fn set_scope_enabled(&self, enabled: bool) {
        self.scope_enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn restore_demo_driver(&mut self) {
        self.driver = PerformanceDriver::Demo(DemoPerformer::spawn(self.tx.clone()));
    }

    pub fn print_startup_summary(&self) {
        if !self.terminal_output_enabled {
            return;
        }

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
}
