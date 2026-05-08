use super::*;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TransportMetricsSnapshot {
    pub queued_frames: usize,
    pub queue_target_frames: usize,
    pub write_frames_hint: usize,
    pub underrun_batches: u64,
    pub underrun_frames: u64,
    pub xrun_recoveries: u64,
    pub overflow_batches: u64,
    pub overflow_frames: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingState {
    Idle,
    Active,
    Finished,
    Error,
}

impl RecordingState {
    pub fn from_usize(value: usize) -> Self {
        match value {
            1 => Self::Active,
            2 => Self::Finished,
            3 => Self::Error,
            _ => Self::Idle,
        }
    }

    pub fn as_usize(self) -> usize {
        match self {
            Self::Idle => 0,
            Self::Active => 1,
            Self::Finished => 2,
            Self::Error => 3,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Active => "recording",
            Self::Finished => "done",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordingMetricsSnapshot {
    pub state: RecordingState,
    pub path: Option<PathBuf>,
    pub target_frames: Option<u64>,
    pub frames_written: u64,
    pub frames_dropped: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct InputMetricsSnapshot {
    pub midi_messages: u64,
    pub midi_messages_accepted: u64,
    pub midi_messages_dropped: u64,
    pub runtime_controls_dropped: u64,
    pub trace_records_dropped: u64,
    pub controllers_coalesced: u64,
    pub last_control: Option<LastControlEvent>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LastControlEvent {
    pub kind: LastControlKind,
    pub label: String,
    pub action: String,
    pub raw_status: u8,
    pub channel: Option<u8>,
    pub raw_value: Option<f32>,
    pub program: Option<u8>,
    pub verdict: LastControlVerdict,
    pub received_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LastControlKind {
    ProfileCc(u8),
    LegacyCc(u8),
    ProgramChange(u8),
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LastControlVerdict {
    Accepted,
    Filtered,
    StartupSuppressed,
    Reserved,
    ReleaseIgnored,
    Ignored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorityAction {
    Panic,
    ResetControllers,
}

#[derive(Debug, Default)]
pub struct TransportMetrics {
    pub queued_frames: AtomicUsize,
    pub queue_target_frames: AtomicUsize,
    pub write_frames_hint: AtomicUsize,
    pub underrun_batches: AtomicU64,
    pub underrun_frames: AtomicU64,
    pub xrun_recoveries: AtomicU64,
    pub overflow_batches: AtomicU64,
    pub overflow_frames: AtomicU64,
}

impl TransportMetrics {
    pub fn snapshot(&self) -> TransportMetricsSnapshot {
        TransportMetricsSnapshot {
            queued_frames: self.queued_frames.load(Ordering::Relaxed),
            queue_target_frames: self.queue_target_frames.load(Ordering::Relaxed),
            write_frames_hint: self.write_frames_hint.load(Ordering::Relaxed),
            underrun_batches: self.underrun_batches.load(Ordering::Relaxed),
            underrun_frames: self.underrun_frames.load(Ordering::Relaxed),
            xrun_recoveries: self.xrun_recoveries.load(Ordering::Relaxed),
            overflow_batches: self.overflow_batches.load(Ordering::Relaxed),
            overflow_frames: self.overflow_frames.load(Ordering::Relaxed),
        }
    }

    pub fn set_queued_frames(&self, queued_frames: usize) {
        self.queued_frames.store(queued_frames, Ordering::Relaxed);
    }

    pub fn record_write_request(&self, write_frames: usize) {
        self.write_frames_hint
            .store(write_frames, Ordering::Relaxed);
    }

    pub fn queue_target_frames(&self, capacity_frames: usize) -> usize {
        let write_frames = round_up_to_render_block(self.write_frames_hint.load(Ordering::Relaxed));
        let target_frames = write_frames
            .max(AUDIO_QUEUE_TARGET_FRAMES)
            .min(capacity_frames);
        self.queue_target_frames
            .store(target_frames, Ordering::Relaxed);
        target_frames
    }

    pub fn record_underrun(&self, missing_frames: usize) {
        if missing_frames == 0 {
            return;
        }

        self.underrun_batches.fetch_add(1, Ordering::Relaxed);
        self.underrun_frames
            .fetch_add(missing_frames as u64, Ordering::Relaxed);
    }

    pub fn record_xrun_recovery(&self) {
        self.xrun_recoveries.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_overflow(&self, dropped_frames: usize) {
        if dropped_frames == 0 {
            return;
        }

        self.overflow_batches.fetch_add(1, Ordering::Relaxed);
        self.overflow_frames
            .fetch_add(dropped_frames as u64, Ordering::Relaxed);
    }
}

#[derive(Debug, Default)]
pub struct RecordingMetrics {
    pub state: AtomicUsize,
    pub target_frames: AtomicU64,
    pub frames_written: AtomicU64,
    pub frames_dropped: AtomicU64,
    pub path: Mutex<Option<PathBuf>>,
    pub error: Mutex<Option<String>>,
}

impl RecordingMetrics {
    pub fn snapshot(&self) -> RecordingMetricsSnapshot {
        let target_frames = self.target_frames.load(Ordering::Relaxed);
        RecordingMetricsSnapshot {
            state: RecordingState::from_usize(self.state.load(Ordering::Relaxed)),
            path: self.path.lock().ok().and_then(|path| path.clone()),
            target_frames: (target_frames > 0).then_some(target_frames),
            frames_written: self.frames_written.load(Ordering::Relaxed),
            frames_dropped: self.frames_dropped.load(Ordering::Relaxed),
            error: self.error.lock().ok().and_then(|error| error.clone()),
        }
    }

    pub fn start(&self, path: PathBuf, target_frames: Option<usize>) {
        if let Ok(mut current_path) = self.path.lock() {
            *current_path = Some(path);
        }
        if let Ok(mut error) = self.error.lock() {
            *error = None;
        }
        self.target_frames
            .store(target_frames.unwrap_or(0) as u64, Ordering::Relaxed);
        self.frames_written.store(0, Ordering::Relaxed);
        self.frames_dropped.store(0, Ordering::Relaxed);
        self.state
            .store(RecordingState::Active.as_usize(), Ordering::Relaxed);
    }

    pub fn set_frames_written(&self, frames_written: u64) {
        self.frames_written.store(frames_written, Ordering::Relaxed);
    }

    pub fn record_dropped(&self, dropped_frames: usize) {
        if dropped_frames == 0 {
            return;
        }
        self.frames_dropped
            .fetch_add(dropped_frames as u64, Ordering::Relaxed);
    }

    pub fn finish_if_active(&self) {
        let _ = self.state.compare_exchange(
            RecordingState::Active.as_usize(),
            RecordingState::Finished.as_usize(),
            Ordering::Relaxed,
            Ordering::Relaxed,
        );
    }

    pub fn record_error(&self, error_message: String) {
        if let Ok(mut error) = self.error.lock() {
            *error = Some(error_message);
        }
        self.state
            .store(RecordingState::Error.as_usize(), Ordering::Relaxed);
    }
}

#[derive(Debug, Default)]
pub struct InputMetrics {
    pub midi_messages: AtomicU64,
    pub midi_messages_accepted: AtomicU64,
    pub midi_messages_dropped: AtomicU64,
    pub runtime_controls_dropped: AtomicU64,
    pub trace_records_dropped: AtomicU64,
    pub controllers_coalesced: AtomicU64,
    pub last_control: Mutex<Option<LastControlEvent>>,
}

impl InputMetrics {
    pub fn snapshot(&self) -> InputMetricsSnapshot {
        InputMetricsSnapshot {
            midi_messages: self.midi_messages.load(Ordering::Relaxed),
            midi_messages_accepted: self.midi_messages_accepted.load(Ordering::Relaxed),
            midi_messages_dropped: self.midi_messages_dropped.load(Ordering::Relaxed),
            runtime_controls_dropped: self.runtime_controls_dropped.load(Ordering::Relaxed),
            trace_records_dropped: self.trace_records_dropped.load(Ordering::Relaxed),
            controllers_coalesced: self.controllers_coalesced.load(Ordering::Relaxed),
            last_control: self
                .last_control
                .lock()
                .ok()
                .and_then(|last_control| last_control.clone()),
        }
    }

    pub fn record_midi_message(&self) {
        self.midi_messages.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_midi_message_accepted(&self) {
        self.midi_messages_accepted.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_midi_message_dropped(&self) {
        self.midi_messages_dropped.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_runtime_control_dropped(&self) {
        self.runtime_controls_dropped
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_trace_record_dropped(&self) {
        self.trace_records_dropped.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_controllers_coalesced(&self, count: usize) {
        if count > 0 {
            self.controllers_coalesced
                .fetch_add(count as u64, Ordering::Relaxed);
        }
    }

    pub fn record_last_control(&self, event: LastControlEvent) {
        if let Ok(mut last_control) = self.last_control.lock() {
            *last_control = Some(event);
        }
    }
}

impl AlsaPlaybackTuning {
    pub fn from_play_options(options: &PlayOptions) -> Result<Self> {
        let tuning = Self {
            period_frames: options
                .alsa_period_frames
                .unwrap_or(ALSA_PERIOD_FRAMES_DEFAULT),
            buffer_frames: options
                .alsa_buffer_frames
                .unwrap_or(ALSA_BUFFER_FRAMES_DEFAULT),
            start_threshold_frames: options
                .alsa_start_threshold_frames
                .unwrap_or(ALSA_START_THRESHOLD_FRAMES_DEFAULT),
        };
        tuning.validate()?;
        Ok(tuning)
    }

    pub fn validate(self) -> Result<Self> {
        if self.buffer_frames < self.period_frames {
            return Err(anyhow!(
                "ALSA buffer size {} must be greater than or equal to period size {}",
                self.buffer_frames,
                self.period_frames
            ));
        }
        if self.start_threshold_frames > self.buffer_frames {
            return Err(anyhow!(
                "ALSA start threshold {} cannot exceed buffer size {}",
                self.start_threshold_frames,
                self.buffer_frames
            ));
        }
        if self.period_frames > AUDIO_QUEUE_CAPACITY_FRAMES {
            return Err(anyhow!(
                "ALSA period size {} exceeds internal queue capacity {}; lower --alsa-period-frames or raise queue capacity in code",
                self.period_frames,
                AUDIO_QUEUE_CAPACITY_FRAMES
            ));
        }
        Ok(self)
    }
}

#[derive(Debug, Default)]
pub struct PriorityActions {
    pub panic_requested: AtomicBool,
    pub reset_controllers_requested: AtomicBool,
}

impl PriorityActions {
    pub fn request_panic(&self) {
        self.panic_requested.store(true, Ordering::Relaxed);
    }

    pub fn request_reset_controllers(&self) {
        self.reset_controllers_requested
            .store(true, Ordering::Relaxed);
    }

    pub fn take_action(&self) -> Option<PriorityAction> {
        if self.panic_requested.swap(false, Ordering::Relaxed) {
            self.reset_controllers_requested
                .store(false, Ordering::Relaxed);
            return Some(PriorityAction::Panic);
        }

        self.reset_controllers_requested
            .swap(false, Ordering::Relaxed)
            .then_some(PriorityAction::ResetControllers)
    }
}
