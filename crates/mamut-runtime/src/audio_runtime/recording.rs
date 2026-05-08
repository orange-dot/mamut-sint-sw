use super::*;

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

        let chunk = match consumer.read_chunk(available_frames) {
            Ok(chunk) => chunk,
            Err(_) => {
                missing_frames += total_frames - frame_offset;
                break;
            }
        };
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

    let queued_frames_before = producer
        .buffer()
        .capacity()
        .saturating_sub(producer.slots());
    let mut chunk = match producer.write_chunk(writable_frames) {
        Ok(chunk) => chunk,
        Err(_) => {
            transport_metrics.set_queued_frames(queued_frames_before);
            transport_metrics.record_overflow(frame_count);
            return;
        }
    };
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
