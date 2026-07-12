use super::*;

pub struct EngineThreadState {
    pub engine: Engine,
    pub rx: mpsc::Receiver<EngineCommand>,
    pub producer: Producer<StereoFrame>,
    pub scope_producer: Producer<StereoFrame>,
    pub scope_enabled: Arc<AtomicBool>,
    pub midi_input_queue: Arc<ArrayQueue<RealtimeMidiMessage>>,
    pub priority_actions: Arc<PriorityActions>,
    pub transport_metrics: Arc<TransportMetrics>,
    pub input_metrics: Arc<InputMetrics>,
    pub recording_metrics: Arc<RecordingMetrics>,
    pub left: Vec<f32>,
    pub right: Vec<f32>,
    pub note_events: Vec<Scheduled<NoteEvent>>,
    pub controller_events: Vec<Scheduled<ControllerEvent>>,
    pub snapshot_requests: Vec<mpsc::Sender<EngineSnapshot>>,
    pub output_recorder: Option<OutputRecorder>,
    pub recording_workers: Vec<OutputRecordingWorker>,
}

impl EngineThreadState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        engine: Engine,
        rx: mpsc::Receiver<EngineCommand>,
        producer: Producer<StereoFrame>,
        scope_producer: Producer<StereoFrame>,
        scope_enabled: Arc<AtomicBool>,
        midi_input_queue: Arc<ArrayQueue<RealtimeMidiMessage>>,
        priority_actions: Arc<PriorityActions>,
        transport_metrics: Arc<TransportMetrics>,
        input_metrics: Arc<InputMetrics>,
        recording_metrics: Arc<RecordingMetrics>,
    ) -> Self {
        Self {
            engine,
            rx,
            producer,
            scope_producer,
            scope_enabled,
            midi_input_queue,
            priority_actions,
            transport_metrics,
            input_metrics,
            recording_metrics,
            left: vec![0.0; ENGINE_RENDER_BLOCK_FRAMES],
            right: vec![0.0; ENGINE_RENDER_BLOCK_FRAMES],
            note_events: Vec::with_capacity(MIDI_INPUT_QUEUE_CAPACITY),
            controller_events: Vec::with_capacity(MIDI_INPUT_QUEUE_CAPACITY),
            snapshot_requests: Vec::with_capacity(4),
            output_recorder: None,
            recording_workers: Vec::new(),
        }
    }

    pub fn run(mut self) {
        loop {
            if !self.apply_priority_actions() {
                break;
            }
            self.drain_realtime_midi_nonblocking();
            if !self.drain_commands_nonblocking() {
                break;
            }

            self.flush_snapshot_requests();
            self.reap_recording_workers();

            if self.queue_needs_audio() {
                self.render_audio_block();
                continue;
            }

            match self.rx.recv_timeout(ENGINE_IDLE_SLEEP) {
                Ok(command) => {
                    if !self.handle_command(command) {
                        break;
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }

        self.shutdown_recorders();
    }

    pub fn drain_commands_nonblocking(&mut self) -> bool {
        loop {
            match self.rx.try_recv() {
                Ok(command) => {
                    if !self.handle_command(command) {
                        return false;
                    }
                }
                Err(TryRecvError::Empty) => return true,
                Err(TryRecvError::Disconnected) => return false,
            }
        }
    }

    pub fn drain_realtime_midi_nonblocking(&mut self) {
        while let Some(message) = self.midi_input_queue.pop() {
            match message {
                RealtimeMidiMessage::Note(event) => self.note_events.push(Scheduled {
                    frame_offset: 0,
                    event,
                }),
                RealtimeMidiMessage::Controller(event) => self.controller_events.push(Scheduled {
                    frame_offset: 0,
                    event,
                }),
            }
        }
    }

    pub fn handle_command(&mut self, command: EngineCommand) -> bool {
        match command {
            EngineCommand::Note(event) => self.note_events.push(Scheduled {
                frame_offset: 0,
                event,
            }),
            EngineCommand::Controller(event) => self.controller_events.push(Scheduled {
                frame_offset: 0,
                event,
            }),
            EngineCommand::RequestSnapshot(reply) => self.snapshot_requests.push(reply),
            EngineCommand::RequestPatch(reply) => {
                let _ = reply.send(self.engine.export_patch());
            }
            EngineCommand::SetGfmLayerMode(mode, reply) => {
                let _ = reply.send(Ok(self.engine.set_gfm_layer_mode(mode)));
            }
            EngineCommand::SetBcsLayerMode(mode, reply) => {
                let _ = reply.send(Ok(self.engine.set_bcs_layer_mode(mode)));
            }
            EngineCommand::SetMozaikMode(mode, reply) => {
                let _ = reply.send(Ok(self.engine.set_mozaik_mode(mode)));
            }
            EngineCommand::SetMozaikParam(param, value, reply) => {
                let _ = reply.send(Ok(self.engine.set_mozaik_param(param, value)));
            }
            EngineCommand::StartOutputRecording(request, reply) => {
                let _ = reply.send(self.start_output_recording(request));
            }
            EngineCommand::StopOutputRecording(reply) => {
                let _ = reply.send(self.stop_output_recording());
            }
            EngineCommand::Shutdown => return false,
        }
        true
    }

    pub fn apply_priority_actions(&mut self) -> bool {
        let Some(action) = self.priority_actions.take_action() else {
            return true;
        };

        self.note_events.clear();
        self.controller_events.clear();
        while self.midi_input_queue.pop().is_some() {}

        loop {
            match self.rx.try_recv() {
                Ok(EngineCommand::Note(_)) | Ok(EngineCommand::Controller(_)) => {}
                Ok(EngineCommand::RequestSnapshot(reply)) => self.snapshot_requests.push(reply),
                Ok(EngineCommand::RequestPatch(reply)) => {
                    let _ = reply.send(self.engine.export_patch());
                }
                Ok(EngineCommand::SetGfmLayerMode(mode, reply)) => {
                    let _ = reply.send(Ok(self.engine.set_gfm_layer_mode(mode)));
                }
                Ok(EngineCommand::SetBcsLayerMode(mode, reply)) => {
                    let _ = reply.send(Ok(self.engine.set_bcs_layer_mode(mode)));
                }
                Ok(EngineCommand::SetMozaikMode(mode, reply)) => {
                    let _ = reply.send(Ok(self.engine.set_mozaik_mode(mode)));
                }
                Ok(EngineCommand::SetMozaikParam(param, value, reply)) => {
                    let _ = reply.send(Ok(self.engine.set_mozaik_param(param, value)));
                }
                Ok(EngineCommand::StartOutputRecording(request, reply)) => {
                    let _ = reply.send(self.start_output_recording(request));
                }
                Ok(EngineCommand::StopOutputRecording(reply)) => {
                    let _ = reply.send(self.stop_output_recording());
                }
                Ok(EngineCommand::Shutdown) => return false,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return false,
            }
        }

        match action {
            PriorityAction::Panic => self.engine.panic(),
            PriorityAction::ResetControllers => self.engine.reset_controllers(),
        }

        true
    }

    pub fn flush_snapshot_requests(&mut self) {
        if self.snapshot_requests.is_empty() {
            return;
        }

        let snapshot = self.engine.snapshot();
        for reply in self.snapshot_requests.drain(..) {
            let _ = reply.send(snapshot.clone());
        }
    }

    pub fn queue_needs_audio(&self) -> bool {
        let capacity_frames = self.producer.buffer().capacity();
        let queued_frames = capacity_frames.saturating_sub(self.producer.slots());
        let target_frames = self.transport_metrics.queue_target_frames(capacity_frames);
        self.transport_metrics.set_queued_frames(queued_frames);
        queued_frames < target_frames
    }

    pub fn render_audio_block(&mut self) {
        let controllers_coalesced = coalesce_controller_events(&mut self.controller_events);
        self.input_metrics
            .record_controllers_coalesced(controllers_coalesced);
        self.engine.process_block(ProcessBlock {
            frame_count: ENGINE_RENDER_BLOCK_FRAMES,
            note_events: &self.note_events,
            controller_events: &self.controller_events,
            macro_state: None,
            output: Some(StereoBlockMut::new(&mut self.left, &mut self.right)),
        });
        self.note_events.clear();
        self.controller_events.clear();

        push_rendered_channels_into_queue(
            &mut self.producer,
            &self.transport_metrics,
            &self.left,
            &self.right,
        );
        self.publish_scope_block();
        self.record_output_block();
    }

    pub fn publish_scope_block(&mut self) {
        if !self.scope_enabled.load(Ordering::Relaxed) {
            return;
        }
        let _ = push_recording_channels_into_queue(
            &mut self.scope_producer,
            &self.left,
            &self.right,
            self.left.len(),
        );
    }

    pub fn start_output_recording(
        &mut self,
        request: OutputRecordingRequest,
    ) -> std::result::Result<PathBuf, String> {
        self.reap_recording_workers();
        if self.output_recorder.is_some() {
            return Err("output recording is already active".to_string());
        }
        if !self.recording_workers.is_empty() {
            return Err("previous output recording is still finalizing".to_string());
        }
        let recorder = OutputRecorder::start(request, Arc::clone(&self.recording_metrics))?;
        let path = recorder.path.clone();
        self.output_recorder = Some(recorder);
        Ok(path)
    }

    pub fn stop_output_recording(&mut self) -> std::result::Result<Option<PathBuf>, String> {
        let Some(recorder) = self.output_recorder.take() else {
            return Ok(None);
        };
        let path = recorder.path.clone();
        self.recording_workers.push(recorder.stop_async());
        Ok(Some(path))
    }

    pub fn record_output_block(&mut self) {
        let finished = match self.output_recorder.as_mut() {
            Some(recorder) => recorder.record_block(&self.left, &self.right),
            None => false,
        };
        if finished {
            let _ = self.stop_output_recording();
        }
    }

    pub fn reap_recording_workers(&mut self) {
        let active_worker_finished = self
            .output_recorder
            .as_ref()
            .is_some_and(|recorder| recorder.worker.is_finished());
        if active_worker_finished {
            if let Some(recorder) = self.output_recorder.take() {
                self.recording_workers.push(recorder.stop_async());
            }
        }

        let mut index = 0;
        while index < self.recording_workers.len() {
            if self.recording_workers[index].is_finished() {
                let worker = self.recording_workers.swap_remove(index);
                worker.join();
            } else {
                index += 1;
            }
        }
    }

    pub fn shutdown_recorders(&mut self) {
        if let Some(recorder) = self.output_recorder.take() {
            self.recording_workers.push(recorder.stop_async());
        }
        for worker in self.recording_workers.drain(..) {
            worker.join();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControllerCoalesceKey {
    PitchBend,
    ModWheel,
    ChannelAftertouch,
    GfmLayerAmount,
    BcsLayerAmount,
    Macro(MacroId),
    DirectParam(ParamId),
}

pub fn controller_coalesce_key(event: ControllerEvent) -> Option<ControllerCoalesceKey> {
    match event {
        ControllerEvent::PitchBend { .. } => Some(ControllerCoalesceKey::PitchBend),
        ControllerEvent::ModWheel { .. } => Some(ControllerCoalesceKey::ModWheel),
        ControllerEvent::ChannelAftertouch { .. } => Some(ControllerCoalesceKey::ChannelAftertouch),
        ControllerEvent::GfmLayerAmount { .. } => Some(ControllerCoalesceKey::GfmLayerAmount),
        ControllerEvent::BcsLayerAmount { .. } => Some(ControllerCoalesceKey::BcsLayerAmount),
        ControllerEvent::Macro { id, .. } => Some(ControllerCoalesceKey::Macro(id)),
        ControllerEvent::DirectParam { id, .. } => Some(ControllerCoalesceKey::DirectParam(id)),
        ControllerEvent::Sustain { .. } | ControllerEvent::BcsLayerEnabled { .. } => None,
    }
}

pub fn coalesce_controller_events(events: &mut Vec<Scheduled<ControllerEvent>>) -> usize {
    let original_len = events.len();
    let mut write_len = 0;
    let mut coalesced = 0;
    for read_index in 0..original_len {
        let event = events[read_index];
        let Some(key) = controller_coalesce_key(event.event) else {
            events[write_len] = event;
            write_len += 1;
            continue;
        };
        if let Some(existing_index) =
            (0..write_len).find(|index| controller_coalesce_key(events[*index].event) == Some(key))
        {
            events[existing_index] = event;
            coalesced += 1;
        } else {
            events[write_len] = event;
            write_len += 1;
        }
    }
    events.truncate(write_len);
    coalesced
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_engine_thread(
    engine: Engine,
    rx: mpsc::Receiver<EngineCommand>,
    producer: Producer<StereoFrame>,
    scope_producer: Producer<StereoFrame>,
    scope_enabled: Arc<AtomicBool>,
    midi_input_queue: Arc<ArrayQueue<RealtimeMidiMessage>>,
    priority_actions: Arc<PriorityActions>,
    transport_metrics: Arc<TransportMetrics>,
    input_metrics: Arc<InputMetrics>,
    recording_metrics: Arc<RecordingMetrics>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        EngineThreadState::new(
            engine,
            rx,
            producer,
            scope_producer,
            scope_enabled,
            midi_input_queue,
            priority_actions,
            transport_metrics,
            input_metrics,
            recording_metrics,
        )
        .run()
    })
}
