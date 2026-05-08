use super::*;

#[derive(Clone)]
pub struct MidiTracePublisher {
    queue: Arc<ArrayQueue<RawMidiTraceRecord>>,
    input_metrics: Arc<InputMetrics>,
}

impl MidiTracePublisher {
    pub fn publish(&self, record: RawMidiTraceRecord) {
        if self.queue.push(record).is_err() {
            self.input_metrics.record_trace_record_dropped();
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RawMidiTraceRecord {
    pub received_at: Instant,
    pub timing: MidiTraceTiming,
    pub raw: [u8; MIDI_TRACE_RAW_BYTES],
    pub stored_len: u8,
    pub original_len: u8,
    pub parsed: Option<ParsedMidiMessage>,
    pub overlay: Option<SoundLabOverlayEvents>,
    pub startup_suppressed: bool,
}

impl RawMidiTraceRecord {
    pub fn new(
        message: &[u8],
        received_at: Instant,
        timing: MidiTraceTiming,
        parsed: Option<ParsedMidiMessage>,
        overlay: Option<SoundLabOverlayEvents>,
        startup_suppressed: bool,
    ) -> Self {
        let mut raw = [0; MIDI_TRACE_RAW_BYTES];
        let stored_len = message.len().min(MIDI_TRACE_RAW_BYTES);
        raw[..stored_len].copy_from_slice(&message[..stored_len]);
        Self {
            received_at,
            timing,
            raw,
            stored_len: stored_len as u8,
            original_len: message.len().min(u8::MAX as usize) as u8,
            parsed,
            overlay,
            startup_suppressed,
        }
    }

    pub fn raw_message(&self) -> &[u8] {
        &self.raw[..usize::from(self.stored_len)]
    }

    pub fn routed(&self) -> Option<ParsedMidiMessage> {
        if self.startup_suppressed {
            None
        } else {
            self.parsed
        }
    }
}

pub struct MidiTraceWorker {
    publisher: MidiTracePublisher,
    stop: Arc<AtomicBool>,
    terminal_trace: Arc<AtomicBool>,
    join_handle: Option<JoinHandle<()>>,
}

impl MidiTraceWorker {
    pub fn spawn(
        trace_midi: bool,
        midi_channel: Option<u8>,
        controller_profile: Option<Arc<ControllerProfile>>,
        input_metrics: Arc<InputMetrics>,
        midi_trace_log: Arc<MidiTraceLog>,
    ) -> Self {
        let queue = Arc::new(ArrayQueue::new(MIDI_TRACE_QUEUE_CAPACITY));
        let stop = Arc::new(AtomicBool::new(false));
        let terminal_trace = Arc::new(AtomicBool::new(trace_midi));
        let thread_queue = Arc::clone(&queue);
        let thread_stop = Arc::clone(&stop);
        let thread_terminal_trace = Arc::clone(&terminal_trace);
        let thread_metrics = Arc::clone(&input_metrics);
        let join_handle = thread::spawn(move || {
            run_midi_trace_worker(
                thread_terminal_trace,
                midi_channel,
                controller_profile,
                thread_metrics,
                midi_trace_log,
                thread_queue,
                thread_stop,
            )
        });
        Self {
            publisher: MidiTracePublisher {
                queue,
                input_metrics,
            },
            stop,
            terminal_trace,
            join_handle: Some(join_handle),
        }
    }

    pub fn publisher(&self) -> MidiTracePublisher {
        self.publisher.clone()
    }

    pub fn set_terminal_trace_enabled(&self, enabled: bool) {
        self.terminal_trace.store(enabled, Ordering::Relaxed);
    }

    pub fn shutdown(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

impl Drop for MidiTraceWorker {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub fn run_midi_trace_worker(
    terminal_trace: Arc<AtomicBool>,
    midi_channel: Option<u8>,
    controller_profile: Option<Arc<ControllerProfile>>,
    input_metrics: Arc<InputMetrics>,
    midi_trace_log: Arc<MidiTraceLog>,
    queue: Arc<ArrayQueue<RawMidiTraceRecord>>,
    stop: Arc<AtomicBool>,
) {
    loop {
        let Some(record) = queue.pop() else {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            thread::sleep(Duration::from_millis(1));
            continue;
        };
        handle_midi_trace_record(
            record,
            terminal_trace.load(Ordering::Relaxed),
            midi_channel,
            controller_profile.as_deref(),
            &input_metrics,
            &midi_trace_log,
        );
    }

    while let Some(record) = queue.pop() {
        handle_midi_trace_record(
            record,
            terminal_trace.load(Ordering::Relaxed),
            midi_channel,
            controller_profile.as_deref(),
            &input_metrics,
            &midi_trace_log,
        );
    }
    midi_trace_log.finish_with_trace_drops(
        "midi trace worker shutdown",
        input_metrics.snapshot().trace_records_dropped,
    );
}

pub fn handle_midi_trace_record(
    record: RawMidiTraceRecord,
    trace_midi: bool,
    midi_channel: Option<u8>,
    controller_profile: Option<&ControllerProfile>,
    input_metrics: &InputMetrics,
    midi_trace_log: &MidiTraceLog,
) {
    if let Some(event) = last_control_event(
        record.raw_message(),
        midi_channel,
        controller_profile,
        record.routed(),
        record.received_at,
        record.startup_suppressed,
    ) {
        input_metrics.record_last_control(event);
    }

    let trace_to_sidecar = midi_trace_log.is_active();
    if !trace_midi && !trace_to_sidecar {
        return;
    }

    let mut line = if record.startup_suppressed {
        format_midi_trace_startup_suppressed(
            record.raw_message(),
            midi_channel,
            controller_profile,
            record.parsed,
            record.timing,
        )
    } else {
        format_midi_trace_message(
            record.raw_message(),
            midi_channel,
            controller_profile,
            record.parsed,
            record.timing,
        )
    };
    if record.original_len > record.stored_len {
        line.push_str(" raw_truncated");
    }
    if let Some(overlay) = record.overlay {
        line.push_str(" -> ");
        line.push_str(&overlay.trace_summary());
    }
    if trace_midi {
        eprintln!("{line}");
    }
    if trace_to_sidecar {
        midi_trace_log.write_line(record.received_at, &line);
    }
}
