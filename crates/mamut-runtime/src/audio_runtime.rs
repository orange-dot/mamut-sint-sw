use super::*;

pub fn run_alsa_playback_loop(
    opened: OpenedAlsaPlayback,
    mut consumer: Consumer<StereoFrame>,
    transport_metrics: Arc<TransportMetrics>,
    stop: Arc<AtomicBool>,
) {
    let OpenedAlsaPlayback {
        pcm,
        audio_selector,
        audio_device_name,
        sample_rate_hz: _sample_rate_hz,
        channels,
        sample_format,
        tuning,
    } = opened;

    let mut writer = match AlsaPlaybackWriter::new(
        &pcm,
        sample_format,
        tuning.period_frames * channels.max(1),
    ) {
        Ok(writer) => writer,
        Err(error) => {
            eprintln!(
                "audio stream error: failed to create ALSA io for {}: {error}",
                audio_selector
            );
            return;
        }
    };
    let channel_count = channels.max(1);
    let mut output = vec![0.0_f32; tuning.period_frames * channel_count];

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }

        match pcm.wait(Some(ALSA_WAIT_TIMEOUT_MS)) {
            Ok(true) => {}
            Ok(false) => continue,
            Err(error) => {
                if recover_alsa_playback_error(
                    &pcm,
                    &transport_metrics,
                    error,
                    &audio_selector,
                    &audio_device_name,
                )
                .is_ok()
                {
                    continue;
                }
                break;
            }
        }

        let available_frames = match pcm.avail_update() {
            Ok(frames) if frames > 0 => frames as usize,
            Ok(_) => continue,
            Err(error) => {
                if recover_alsa_playback_error(
                    &pcm,
                    &transport_metrics,
                    error,
                    &audio_selector,
                    &audio_device_name,
                )
                .is_ok()
                {
                    continue;
                }
                break;
            }
        };

        let frames_to_write = available_frames.min(tuning.period_frames).max(1);
        let sample_count = frames_to_write * channel_count;
        drain_queue_into_output(
            &mut consumer,
            &transport_metrics,
            &mut output[..sample_count],
            channel_count,
        );

        let mut written_frames = 0_usize;
        while written_frames < frames_to_write {
            let start = written_frames * channel_count;
            let end = frames_to_write * channel_count;
            match writer.write_interleaved(&output[start..end]) {
                Ok(0) => break,
                Ok(written_now) => written_frames += written_now,
                Err(error) => {
                    if recover_alsa_playback_error(
                        &pcm,
                        &transport_metrics,
                        error,
                        &audio_selector,
                        &audio_device_name,
                    )
                    .is_ok()
                    {
                        break;
                    }
                    return;
                }
            }
        }
    }
}

pub enum AlsaPlaybackWriter<'a> {
    Float32(IO<'a, f32>),
    Signed32 { io: IO<'a, i32>, buffer: Vec<i32> },
}

impl<'a> AlsaPlaybackWriter<'a> {
    fn new(
        pcm: &'a PCM,
        sample_format: AlsaPlaybackSampleFormat,
        capacity_samples: usize,
    ) -> Result<Self> {
        match sample_format {
            AlsaPlaybackSampleFormat::Float32 => pcm
                .io_f32()
                .map(Self::Float32)
                .context("failed to create F32 ALSA IO"),
            AlsaPlaybackSampleFormat::Signed32 => Ok(Self::Signed32 {
                io: pcm.io_i32().context("failed to create S32 ALSA IO")?,
                buffer: vec![0; capacity_samples],
            }),
        }
    }

    fn write_interleaved(&mut self, samples: &[f32]) -> std::result::Result<usize, alsa::Error> {
        match self {
            Self::Float32(io) => io.writei(samples),
            Self::Signed32 { io, buffer } => {
                if buffer.len() < samples.len() {
                    buffer.resize(samples.len(), 0);
                }
                convert_f32_samples_to_s32(samples, &mut buffer[..samples.len()]);
                io.writei(&buffer[..samples.len()])
            }
        }
    }
}

pub fn convert_f32_samples_to_s32(samples: &[f32], output: &mut [i32]) {
    debug_assert!(output.len() >= samples.len());
    for (sample, target) in samples.iter().zip(output.iter_mut()) {
        *target = f32_sample_to_s32(*sample);
    }
}

pub fn f32_sample_to_s32(sample: f32) -> i32 {
    if !sample.is_finite() {
        return 0;
    }

    let sample = sample.clamp(-1.0, 1.0);
    if sample >= 1.0 {
        i32::MAX
    } else if sample <= -1.0 {
        i32::MIN
    } else if sample >= 0.0 {
        (sample * i32::MAX as f32).round() as i32
    } else {
        (sample * 2_147_483_648.0).round() as i32
    }
}

pub fn recover_alsa_playback_error(
    pcm: &PCM,
    transport_metrics: &TransportMetrics,
    error: alsa::Error,
    audio_selector: &str,
    audio_device_name: &str,
) -> Result<()> {
    match pcm.try_recover(error, true) {
        Ok(()) => {
            transport_metrics.record_xrun_recovery();
            Ok(())
        }
        Err(error) => {
            eprintln!(
                "audio stream error: ALSA playback failed on {} ({}): {error}",
                audio_selector, audio_device_name
            );
            Err(anyhow!(error))
        }
    }
}

pub struct EngineThreadState {
    pub engine: Engine,
    pub rx: mpsc::Receiver<EngineCommand>,
    pub producer: Producer<StereoFrame>,
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
    pub fn new(
        engine: Engine,
        rx: mpsc::Receiver<EngineCommand>,
        producer: Producer<StereoFrame>,
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
        self.record_output_block();
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

pub fn spawn_engine_thread(
    engine: Engine,
    rx: mpsc::Receiver<EngineCommand>,
    producer: Producer<StereoFrame>,
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
            midi_input_queue,
            priority_actions,
            transport_metrics,
            input_metrics,
            recording_metrics,
        )
        .run()
    })
}

pub fn run_output_recording_writer(
    mut writer: FloatStereoWavWriter,
    mut consumer: Consumer<StereoFrame>,
    metrics: Arc<RecordingMetrics>,
    stop: Arc<AtomicBool>,
) {
    loop {
        let available_frames = consumer.slots();
        if available_frames == 0 {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            thread::sleep(Duration::from_millis(2));
            continue;
        }

        let chunk = match consumer.read_chunk(available_frames) {
            Ok(chunk) => chunk,
            Err(error) => {
                metrics.record_error(format!("recording queue read failed: {error:?}"));
                return;
            }
        };
        let (first, second) = chunk.as_slices();
        let mut write_error = None;
        for frame in first.iter().chain(second.iter()) {
            if let Err(error) = writer.write_frame(*frame) {
                write_error = Some(error);
                break;
            }
        }
        chunk.commit_all();
        metrics.set_frames_written(writer.frames_written());
        if let Some(error) = write_error {
            metrics.record_error(format!("recording write failed: {error}"));
            return;
        }
    }

    match writer.finalize() {
        Ok(frames_written) => {
            metrics.set_frames_written(frames_written);
            metrics.finish_if_active();
        }
        Err(error) => metrics.record_error(format!("recording finalize failed: {error}")),
    }
}

pub fn write_float_stereo_wav_header<W: Write>(
    writer: &mut W,
    data_bytes: u32,
    sample_rate_hz: u32,
) -> io::Result<()> {
    writer.write_all(b"RIFF")?;
    write_u32_le(writer, 36 + data_bytes)?;
    writer.write_all(b"WAVE")?;
    writer.write_all(b"fmt ")?;
    write_u32_le(writer, 16)?;
    write_u16_le(writer, 3)?;
    write_u16_le(writer, ALSA_PLAYBACK_CHANNELS as u16)?;
    write_u32_le(writer, sample_rate_hz)?;
    write_u32_le(
        writer,
        sample_rate_hz * ALSA_PLAYBACK_CHANNELS as u32 * std::mem::size_of::<f32>() as u32,
    )?;
    write_u16_le(
        writer,
        (ALSA_PLAYBACK_CHANNELS * std::mem::size_of::<f32>()) as u16,
    )?;
    write_u16_le(writer, 32)?;
    writer.write_all(b"data")?;
    write_u32_le(writer, data_bytes)
}

pub fn write_u16_le<W: Write>(writer: &mut W, value: u16) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

pub fn write_u32_le<W: Write>(writer: &mut W, value: u32) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

pub fn drain_queue_into_output(
    consumer: &mut Consumer<StereoFrame>,
    transport_metrics: &TransportMetrics,
    output: &mut [f32],
    channels: usize,
) {
    let channel_count = channels.max(1);
    let total_frames = output.len() / channel_count;
    transport_metrics.record_write_request(total_frames);
    let mut missing_frames = 0_usize;
    let mut frame_offset = 0_usize;

    while frame_offset < total_frames {
        let available_frames = consumer.slots().min(total_frames - frame_offset);
        if available_frames == 0 {
            missing_frames += total_frames - frame_offset;
            break;
        }

        let chunk = consumer
            .read_chunk(available_frames)
            .expect("read_chunk should succeed after slots check");
        let (first, second) = chunk.as_slices();

        frame_offset += write_frames_into_output(
            &mut output[frame_offset * channel_count..],
            channel_count,
            first,
        );
        frame_offset += write_frames_into_output(
            &mut output[frame_offset * channel_count..],
            channel_count,
            second,
        );
        chunk.commit_all();
    }

    transport_metrics.set_queued_frames(consumer.slots());
    zero_fill_remaining_output(output, channel_count, frame_offset, total_frames);
    zero_fill_partial_output_tail(output, channel_count);
    transport_metrics.record_underrun(missing_frames);
}

pub fn push_frames_into_queue(
    producer: &mut Producer<StereoFrame>,
    transport_metrics: &TransportMetrics,
    frames: &[StereoFrame],
) {
    let (_, remainder) = producer.push_partial_slice(frames);
    let queued_frames = producer
        .buffer()
        .capacity()
        .saturating_sub(producer.slots());
    transport_metrics.set_queued_frames(queued_frames);
    if !remainder.is_empty() {
        transport_metrics.record_overflow(remainder.len());
    }
}

pub fn push_rendered_channels_into_queue(
    producer: &mut Producer<StereoFrame>,
    transport_metrics: &TransportMetrics,
    left: &[f32],
    right: &[f32],
) {
    let frame_count = left.len().min(right.len());
    let writable_frames = frame_count.min(producer.slots());
    if writable_frames == 0 {
        transport_metrics.set_queued_frames(
            producer
                .buffer()
                .capacity()
                .saturating_sub(producer.slots()),
        );
        transport_metrics.record_overflow(frame_count);
        return;
    }

    let mut chunk = producer
        .write_chunk(writable_frames)
        .expect("write_chunk should succeed after slots check");
    let (first, second) = chunk.as_mut_slices();

    let mut frame_offset = 0_usize;
    frame_offset +=
        write_channels_into_stereo_frames(first, &left[frame_offset..], &right[frame_offset..]);
    frame_offset +=
        write_channels_into_stereo_frames(second, &left[frame_offset..], &right[frame_offset..]);
    chunk.commit_all();

    let queued_frames = producer
        .buffer()
        .capacity()
        .saturating_sub(producer.slots());
    transport_metrics.set_queued_frames(queued_frames);
    transport_metrics.record_overflow(frame_count.saturating_sub(frame_offset));
}

pub fn push_recording_channels_into_queue(
    producer: &mut Producer<StereoFrame>,
    left: &[f32],
    right: &[f32],
    requested_frames: usize,
) -> usize {
    let frame_count = requested_frames.min(left.len()).min(right.len());
    let writable_frames = frame_count.min(producer.slots());
    if writable_frames == 0 {
        return 0;
    }

    let mut chunk = match producer.write_chunk(writable_frames) {
        Ok(chunk) => chunk,
        Err(_) => return 0,
    };
    let (first, second) = chunk.as_mut_slices();

    let mut frame_offset = 0_usize;
    frame_offset +=
        write_channels_into_stereo_frames(first, &left[frame_offset..], &right[frame_offset..]);
    frame_offset +=
        write_channels_into_stereo_frames(second, &left[frame_offset..], &right[frame_offset..]);
    chunk.commit_all();
    frame_offset
}

pub fn write_channels_into_stereo_frames(
    output: &mut [StereoFrame],
    left: &[f32],
    right: &[f32],
) -> usize {
    let frame_count = output.len().min(left.len()).min(right.len());
    for frame_index in 0..frame_count {
        output[frame_index] = [left[frame_index], right[frame_index]];
    }
    frame_count
}

pub fn write_output_frame(frame: &mut [f32], left: f32, right: f32) {
    let mono = (left + right) * 0.5;

    frame[0] = left;
    if frame.len() > 1 {
        frame[1] = right;
    }
    for sample in frame.iter_mut().skip(2) {
        *sample = mono;
    }
}

pub fn write_frames_into_output(
    output: &mut [f32],
    channel_count: usize,
    frames: &[StereoFrame],
) -> usize {
    for (frame_index, stereo) in frames.iter().enumerate() {
        let start = frame_index * channel_count;
        let end = start + channel_count;
        write_output_frame(&mut output[start..end], stereo[0], stereo[1]);
    }
    frames.len()
}

pub fn zero_fill_remaining_output(
    output: &mut [f32],
    channel_count: usize,
    written_frames: usize,
    total_frames: usize,
) {
    for frame in output[written_frames * channel_count..total_frames * channel_count]
        .chunks_mut(channel_count)
    {
        write_output_frame(frame, 0.0, 0.0);
    }
}

pub fn zero_fill_partial_output_tail(output: &mut [f32], channel_count: usize) {
    let full_sample_count = (output.len() / channel_count) * channel_count;
    for sample in &mut output[full_sample_count..] {
        *sample = 0.0;
    }
}

pub struct OpenedMidiConnection {
    pub port_name: String,
    pub _connection: MidiInputConnection<()>,
}

pub struct FloatStereoWavWriter {
    pub writer: BufWriter<File>,
    pub frames_written: u64,
}

impl FloatStereoWavWriter {
    pub fn create(path: &Path, sample_rate_hz: u32) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        write_float_stereo_wav_header(&mut writer, 0, sample_rate_hz)?;
        Ok(Self {
            writer,
            frames_written: 0,
        })
    }

    pub fn write_frame(&mut self, frame: StereoFrame) -> io::Result<()> {
        self.writer.write_all(&frame[0].to_le_bytes())?;
        self.writer.write_all(&frame[1].to_le_bytes())?;
        self.frames_written += 1;
        Ok(())
    }

    pub fn frames_written(&self) -> u64 {
        self.frames_written
    }

    pub fn finalize(mut self) -> io::Result<u64> {
        let frames_written = self.frames_written;
        let data_bytes = frames_written
            .checked_mul(ALSA_PLAYBACK_CHANNELS as u64)
            .and_then(|samples| samples.checked_mul(std::mem::size_of::<f32>() as u64))
            .ok_or_else(|| io::Error::other("recording is too large for WAV"))?;
        if data_bytes > u32::MAX as u64 - 36 {
            return Err(io::Error::other("recording exceeds 32-bit WAV size"));
        }
        self.writer.seek(SeekFrom::Start(4))?;
        write_u32_le(&mut self.writer, 36 + data_bytes as u32)?;
        self.writer.seek(SeekFrom::Start(40))?;
        write_u32_le(&mut self.writer, data_bytes as u32)?;
        self.writer.flush()?;
        Ok(frames_written)
    }
}

pub struct OutputRecordingWorker {
    pub stop: Arc<AtomicBool>,
    pub join_handle: Option<JoinHandle<()>>,
}

impl OutputRecordingWorker {
    pub fn spawn(
        writer: FloatStereoWavWriter,
        consumer: Consumer<StereoFrame>,
        metrics: Arc<RecordingMetrics>,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let join_handle = thread::spawn(move || {
            run_output_recording_writer(writer, consumer, metrics, thread_stop)
        });
        Self {
            stop,
            join_handle: Some(join_handle),
        }
    }

    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    pub fn is_finished(&self) -> bool {
        match &self.join_handle {
            Some(join_handle) => join_handle.is_finished(),
            None => true,
        }
    }

    pub fn join(mut self) {
        self.request_stop();
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

pub struct OutputRecorder {
    pub path: PathBuf,
    pub producer: Producer<StereoFrame>,
    pub worker: OutputRecordingWorker,
    pub target_frames: Option<usize>,
    pub submitted_frames: usize,
    pub metrics: Arc<RecordingMetrics>,
}

impl OutputRecorder {
    pub fn start(
        request: OutputRecordingRequest,
        metrics: Arc<RecordingMetrics>,
    ) -> std::result::Result<Self, String> {
        let writer = FloatStereoWavWriter::create(&request.path, request.sample_rate_hz).map_err(
            |error| {
                format!(
                    "failed to create output recording {}: {error}",
                    request.path.display()
                )
            },
        )?;
        let (producer, consumer) = RingBuffer::<StereoFrame>::new(RECORDING_QUEUE_CAPACITY_FRAMES);
        metrics.start(request.path.clone(), request.max_frames);
        let worker = OutputRecordingWorker::spawn(writer, consumer, Arc::clone(&metrics));
        Ok(Self {
            path: request.path,
            producer,
            worker,
            target_frames: request.max_frames,
            submitted_frames: 0,
            metrics,
        })
    }

    pub fn record_block(&mut self, left: &[f32], right: &[f32]) -> bool {
        let frame_count = left.len().min(right.len());
        let wanted_frames = match self.target_frames {
            Some(target_frames) => {
                frame_count.min(target_frames.saturating_sub(self.submitted_frames))
            }
            None => frame_count,
        };
        if wanted_frames == 0 {
            return true;
        }

        let written_frames =
            push_recording_channels_into_queue(&mut self.producer, left, right, wanted_frames);
        self.metrics
            .record_dropped(wanted_frames.saturating_sub(written_frames));
        self.submitted_frames += wanted_frames;
        self.target_frames
            .is_some_and(|target_frames| self.submitted_frames >= target_frames)
    }

    pub fn stop_async(self) -> OutputRecordingWorker {
        self.worker.request_stop();
        self.worker
    }
}

pub fn parse_midi_message(
    message: &[u8],
    bend_range: f32,
    midi_channel: Option<u8>,
    controller_profile: Option<&ControllerProfile>,
) -> Option<ParsedMidiMessage> {
    let raw_status = *message.first()?;
    let status = raw_status & 0xF0;
    let channel = (raw_status & 0x0F) + 1;
    if status < 0xF0 && midi_channel.is_some_and(|expected| channel != expected) {
        return None;
    }
    match status {
        0x80 if message.len() >= 2 => Some(ParsedMidiMessage::Realtime(RealtimeMidiMessage::Note(
            NoteEvent::NoteOff { note: message[1] },
        ))),
        0x90 if message.len() >= 3 => {
            if message[2] == 0 {
                Some(ParsedMidiMessage::Realtime(RealtimeMidiMessage::Note(
                    NoteEvent::NoteOff { note: message[1] },
                )))
            } else {
                Some(ParsedMidiMessage::Realtime(RealtimeMidiMessage::Note(
                    NoteEvent::NoteOn {
                        note: message[1],
                        velocity: message[2] as f32 / 127.0,
                    },
                )))
            }
        }
        0xB0 if message.len() >= 3 => {
            let cc = message[1];
            let value = message[2] as f32 / 127.0;
            if let Some(binding) = controller_profile.and_then(|profile| profile.binding_for_cc(cc))
            {
                return parse_profile_cc_binding(binding, value);
            }
            let event = match cc {
                1 => ControllerEvent::ModWheel { amount: value },
                64 => ControllerEvent::Sustain { down: value >= 0.5 },
                16 => ControllerEvent::Macro {
                    id: MacroId::Gravitacija,
                    value,
                },
                17 => ControllerEvent::Macro {
                    id: MacroId::Bloom,
                    value,
                },
                18 => ControllerEvent::Macro {
                    id: MacroId::Heat,
                    value,
                },
                19 => ControllerEvent::Macro {
                    id: MacroId::Ruin,
                    value,
                },
                20 => ControllerEvent::Macro {
                    id: MacroId::Swarm,
                    value,
                },
                _ => return None,
            };
            Some(ParsedMidiMessage::Realtime(
                RealtimeMidiMessage::Controller(event),
            ))
        }
        0xC0 if message.len() >= 2 => Some(ParsedMidiMessage::Runtime(
            RuntimeControlMessage::ProgramChange(message[1]),
        )),
        0xD0 if message.len() >= 2 => Some(ParsedMidiMessage::Realtime(
            RealtimeMidiMessage::Controller(ControllerEvent::ChannelAftertouch {
                pressure: message[1] as f32 / 127.0,
            }),
        )),
        0xE0 if message.len() >= 3 => {
            let value = u16::from(message[1]) | (u16::from(message[2]) << 7);
            let normalized = (value as f32 - 8_192.0) / 8_192.0;
            Some(ParsedMidiMessage::Realtime(
                RealtimeMidiMessage::Controller(ControllerEvent::PitchBend {
                    semitones: normalized.clamp(-1.0, 1.0) * bend_range,
                }),
            ))
        }
        _ => None,
    }
}

pub fn parse_profile_cc_binding(
    binding: &ControllerBinding,
    value: f32,
) -> Option<ParsedMidiMessage> {
    match binding.action {
        ControllerBindingAction::Macro(id) => Some(ParsedMidiMessage::Realtime(
            RealtimeMidiMessage::Controller(ControllerEvent::Macro { id, value }),
        )),
        ControllerBindingAction::DirectParam { id, scale } => Some(ParsedMidiMessage::Realtime(
            RealtimeMidiMessage::Controller(ControllerEvent::DirectParam {
                id,
                value: scale_controller_value(id, value, scale),
            }),
        )),
        ControllerBindingAction::GfmLayerAmount => Some(ParsedMidiMessage::Realtime(
            RealtimeMidiMessage::Controller(ControllerEvent::GfmLayerAmount { amount: value }),
        )),
        ControllerBindingAction::BcsLayerAmount => Some(ParsedMidiMessage::Realtime(
            RealtimeMidiMessage::Controller(ControllerEvent::BcsLayerAmount { amount: value }),
        )),
        ControllerBindingAction::BcsLayerEnabled => Some(ParsedMidiMessage::Realtime(
            RealtimeMidiMessage::Controller(ControllerEvent::BcsLayerEnabled {
                enabled: value >= 0.5,
            }),
        )),
        ControllerBindingAction::Runtime(message) => {
            (value >= 0.5).then_some(ParsedMidiMessage::Runtime(message))
        }
        ControllerBindingAction::ToggleParam(id) => (value >= 0.5).then_some(
            ParsedMidiMessage::Runtime(RuntimeControlMessage::ToggleParam(id)),
        ),
        ControllerBindingAction::Reserved => Some(ParsedMidiMessage::Reserved),
    }
}

pub fn sound_lab_midi_overlay_events(
    message: &[u8],
    controller_profile: Option<&ControllerProfile>,
    page: Option<SoundLabPage>,
) -> Option<SoundLabOverlayEvents> {
    let page = page?;
    let raw_status = *message.first()?;
    if raw_status & 0xF0 != 0xB0 || message.len() < 3 {
        return None;
    }

    let cc = message[1];
    let normalized = message[2] as f32 / 127.0;
    let source = sound_lab_overlay_source_for_cc(cc, controller_profile)?;
    match source {
        SoundLabMidiSource::ModWheel => sound_lab_mod_wheel_events(page, normalized),
        SoundLabMidiSource::Knob(_) | SoundLabMidiSource::Slider(_) => {
            let binding = sound_lab_page_binding_for_source(page, source)?;
            Some(SoundLabOverlayEvents::single(
                page,
                source,
                direct_param_overlay_event(binding.id, normalized),
            ))
        }
    }
}

fn sound_lab_overlay_source_for_cc(
    cc: u8,
    controller_profile: Option<&ControllerProfile>,
) -> Option<SoundLabMidiSource> {
    if cc == 1 {
        return Some(SoundLabMidiSource::ModWheel);
    }
    if cc == 64 {
        return None;
    }

    let binding = controller_profile.and_then(|profile| profile.binding_for_cc(cc))?;
    if matches!(
        binding.action,
        ControllerBindingAction::GfmLayerAmount
            | ControllerBindingAction::BcsLayerAmount
            | ControllerBindingAction::BcsLayerEnabled
    ) {
        return None;
    }

    match (binding.section, binding.index) {
        (ControllerBindingSection::Knob, Some(index)) => Some(SoundLabMidiSource::Knob(index)),
        (ControllerBindingSection::Slider, Some(index)) => Some(SoundLabMidiSource::Slider(index)),
        _ => None,
    }
}

fn sound_lab_mod_wheel_events(page: SoundLabPage, value: f32) -> Option<SoundLabOverlayEvents> {
    let params = sound_lab_page_mod_wheel_params(page);
    if params.is_empty() {
        return None;
    }

    let mut events = SoundLabOverlayEvents::empty(page, SoundLabMidiSource::ModWheel);
    for (slot, id) in params
        .iter()
        .copied()
        .take(SOUND_LAB_OVERLAY_EVENT_CAPACITY)
        .enumerate()
    {
        events.events[slot] = Some(direct_param_overlay_event(id, value));
    }
    Some(events)
}

fn direct_param_overlay_event(id: ParamId, normalized: f32) -> ControllerEvent {
    ControllerEvent::DirectParam {
        id,
        value: sound_lab_direct_param_value(id, normalized),
    }
}

pub fn sound_lab_direct_param_value(id: ParamId, normalized: f32) -> f32 {
    let spec = param_spec(id);
    let value = scale_controller_value(id, normalized, default_scale_for_param(id));
    match spec.unit {
        ParamUnit::Boolean => {
            if value >= 0.5 {
                1.0
            } else {
                0.0
            }
        }
        ParamUnit::Indexed | ParamUnit::Semitones => value.round().clamp(spec.min, spec.max),
        _ => value.clamp(spec.min, spec.max),
    }
}

pub fn run_demo_performance(tx: mpsc::Sender<EngineCommand>, thread_stop: Arc<AtomicBool>) {
    let notes = [36_u8, 43, 48, 55, 60, 67];
    let macro_cycle = [
        (MacroId::Bloom, 0.65),
        (MacroId::Heat, 0.58),
        (MacroId::Gravitacija, 0.52),
        (MacroId::Ruin, 0.38),
        (MacroId::Swarm, 0.44),
        (MacroId::Gravitacija, 0.74),
        (MacroId::Ruin, 0.62),
    ];

    let mut step = 0_usize;
    loop {
        if thread_stop.load(Ordering::Relaxed) {
            break;
        }

        let note = notes[step % notes.len()];
        let (macro_id, macro_value) = macro_cycle[step % macro_cycle.len()];
        let velocity = 0.62 + (step % 3) as f32 * 0.10;

        if tx
            .send(EngineCommand::Controller(ControllerEvent::Macro {
                id: macro_id,
                value: macro_value,
            }))
            .is_err()
        {
            break;
        }
        if tx
            .send(EngineCommand::Controller(ControllerEvent::ModWheel {
                amount: ((step % 5) as f32) / 5.0,
            }))
            .is_err()
        {
            break;
        }
        if tx
            .send(EngineCommand::Note(NoteEvent::NoteOn { note, velocity }))
            .is_err()
        {
            break;
        }
        if sleep_interruptibly(&thread_stop, Duration::from_millis(420)) {
            break;
        }
        if tx
            .send(EngineCommand::Controller(
                ControllerEvent::ChannelAftertouch {
                    pressure: 0.20 + ((step % 4) as f32) * 0.12,
                },
            ))
            .is_err()
        {
            break;
        }
        if sleep_interruptibly(&thread_stop, Duration::from_millis(360)) {
            break;
        }
        if tx
            .send(EngineCommand::Note(NoteEvent::NoteOff { note }))
            .is_err()
        {
            break;
        }
        if sleep_interruptibly(&thread_stop, Duration::from_millis(140)) {
            break;
        }
        step = step.wrapping_add(1);
    }
}

pub fn sleep_interruptibly(stop: &AtomicBool, duration: Duration) -> bool {
    let mut remaining = duration;
    while remaining > Duration::ZERO {
        if stop.load(Ordering::Relaxed) {
            return true;
        }
        let slice = remaining.min(Duration::from_millis(50));
        thread::sleep(slice);
        remaining = remaining.saturating_sub(slice);
    }
    stop.load(Ordering::Relaxed)
}
