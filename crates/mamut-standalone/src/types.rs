use super::*;

pub(crate) const ENGINE_RENDER_BLOCK_FRAMES: usize = 256;
pub(crate) const AUDIO_QUEUE_CAPACITY_BLOCKS: usize = 4;
pub(crate) const AUDIO_QUEUE_TARGET_BLOCKS: usize = 2;
pub(crate) const AUDIO_QUEUE_CAPACITY_FRAMES: usize =
    ENGINE_RENDER_BLOCK_FRAMES * AUDIO_QUEUE_CAPACITY_BLOCKS;
pub(crate) const AUDIO_QUEUE_TARGET_FRAMES: usize =
    ENGINE_RENDER_BLOCK_FRAMES * AUDIO_QUEUE_TARGET_BLOCKS;
pub(crate) const ENGINE_IDLE_SLEEP: Duration = Duration::from_millis(1);
pub(crate) const ALSA_WAIT_TIMEOUT_MS: u32 = 100;
pub(crate) const ALSA_PLAYBACK_CHANNELS: usize = 2;
pub(crate) const ALSA_PLAYBACK_SAMPLE_RATE_HZ: u32 = 44_100;
pub(crate) const ALSA_PERIOD_FRAMES_DEFAULT: usize = 256;
pub(crate) const ALSA_BUFFER_FRAMES_DEFAULT: usize = 1_024;
pub(crate) const ALSA_START_THRESHOLD_FRAMES_DEFAULT: usize = ALSA_BUFFER_FRAMES_DEFAULT;
pub(crate) const PERFORMANCE_UI_REFRESH: Duration = Duration::from_millis(75);
pub(crate) const MIDI_ACTIVITY_FLASH: Duration = Duration::from_millis(700);
pub(crate) const MIDI_STARTUP_GUARD: Duration = MIDI_ACTIVITY_FLASH;
pub(crate) const MIDI_INPUT_QUEUE_CAPACITY: usize = 512;
pub(crate) const RUNTIME_CONTROL_QUEUE_CAPACITY: usize = 64;
pub(crate) const RECORDING_QUEUE_CAPACITY_FRAMES: usize = 44_100 * 4;
pub(crate) const RECORDING_REPLY_TIMEOUT: Duration = Duration::from_millis(500);
pub(crate) const DEFAULT_LIVE_TAKE_SECONDS: u64 = 30;
pub(crate) const DEFAULT_GFM_UI_SEED: u64 = DEFAULT_GFM_LAYER_SEED;
pub(crate) const DEFAULT_LIVE_TAKE_TAG: &str = "gravitacija";
pub(crate) const PC4_KNOB_START_ANGLE: f32 = 2.0 * std::f32::consts::PI / 3.0;
pub(crate) const PC4_KNOB_SWEEP_ANGLE: f32 = 5.0 * std::f32::consts::PI / 3.0;
pub(crate) const CAPTURE_DIR_ENV: &str = "MAMUT_CAPTURE_DIR";
pub(crate) const LIVE_SET_STEMS: [&str; 8] = [
    "molten-horizon",
    "cathedral-bloom",
    "ember-vault",
    "razor-thaw",
    "gravity-wake",
    "furnace-choir",
    "granite-plain",
    "glass-tide",
];

pub(crate) type StereoFrame = [f32; 2];

#[derive(Debug, Clone)]
pub(crate) struct OutputRecordingRequest {
    pub(crate) path: PathBuf,
    pub(crate) max_frames: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AlsaPlaybackSampleFormat {
    Float32,
    Signed32,
}

impl AlsaPlaybackSampleFormat {
    pub(crate) fn alsa_format(self) -> Format {
        match self {
            Self::Float32 => Format::float(),
            Self::Signed32 => Format::s32(),
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Float32 => "F32",
            Self::Signed32 => "S32_LE",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum RealtimeMidiMessage {
    Note(NoteEvent),
    Controller(ControllerEvent),
}

#[derive(Debug, Clone)]
pub(crate) struct FactoryPatchEntry {
    pub(crate) stem: String,
    pub(crate) slug: String,
    pub(crate) path: PathBuf,
    pub(crate) patch_name: String,
    pub(crate) description: Option<String>,
    pub(crate) favorite: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PlayOptions {
    pub(crate) patch_path: PathBuf,
    pub(crate) force_demo: bool,
    pub(crate) audio_selector: Option<String>,
    pub(crate) alsa_period_frames: Option<usize>,
    pub(crate) alsa_buffer_frames: Option<usize>,
    pub(crate) alsa_start_threshold_frames: Option<usize>,
    pub(crate) midi_selector: Option<String>,
    pub(crate) midi_channel: Option<u8>,
    pub(crate) controller_profile_path: Option<PathBuf>,
    pub(crate) trace_midi: bool,
    pub(crate) headless: bool,
    pub(crate) gfm_layer_seed: Option<u64>,
    pub(crate) bcs_layer_scenario: Option<BcsScenario>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DryRunOptions {
    pub(crate) patch_path: PathBuf,
    pub(crate) gfm_layer_seed: Option<u64>,
    pub(crate) bcs_layer_scenario: Option<BcsScenario>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AlsaOutputDevice {
    pub(crate) selector: String,
    pub(crate) card_index: i32,
    pub(crate) device_index: i32,
    pub(crate) card_name: String,
    pub(crate) pcm_name: String,
}

impl AlsaOutputDevice {
    pub(crate) fn display_name(&self) -> String {
        format!("{} ({}, {})", self.selector, self.card_name, self.pcm_name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AlsaPlaybackTuning {
    pub(crate) period_frames: usize,
    pub(crate) buffer_frames: usize,
    pub(crate) start_threshold_frames: usize,
}

impl Default for AlsaPlaybackTuning {
    fn default() -> Self {
        Self {
            period_frames: ALSA_PERIOD_FRAMES_DEFAULT,
            buffer_frames: ALSA_BUFFER_FRAMES_DEFAULT,
            start_threshold_frames: ALSA_START_THRESHOLD_FRAMES_DEFAULT,
        }
    }
}

#[derive(Clone)]
pub(crate) struct NamedMidiPort {
    pub(crate) port: MidiInputPort,
    pub(crate) name: String,
}

#[derive(Debug)]
pub(crate) enum EngineCommand {
    Note(NoteEvent),
    Controller(ControllerEvent),
    RequestSnapshot(mpsc::Sender<EngineSnapshot>),
    SetGfmLayerMode(
        GfmLayerMode,
        mpsc::Sender<std::result::Result<GfmVoiceProgramSelection, String>>,
    ),
    SetBcsLayerMode(
        BcsLayerMode,
        mpsc::Sender<std::result::Result<BcsLayerSnapshot, String>>,
    ),
    StartOutputRecording(
        OutputRecordingRequest,
        mpsc::Sender<std::result::Result<PathBuf, String>>,
    ),
    StopOutputRecording(mpsc::Sender<std::result::Result<Option<PathBuf>, String>>),
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeControlMessage {
    ProgramChange(u8),
    Panic,
    ResetControllers,
    NextFavorite,
    PrevFavorite,
    FavoriteSlot(usize),
    ToggleParam(ParamId),
}

#[derive(Debug, Clone)]
pub(crate) struct ControllerProfile {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
    pub(crate) bindings_by_cc: HashMap<u8, ControllerBinding>,
}

impl ControllerProfile {
    pub(crate) fn binding_for_cc(&self, cc: u8) -> Option<&ControllerBinding> {
        self.bindings_by_cc.get(&cc)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ControllerBinding {
    pub(crate) cc: u8,
    pub(crate) control: String,
    pub(crate) section: ControllerBindingSection,
    pub(crate) index: Option<u8>,
    pub(crate) action: ControllerBindingAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ControllerBindingSection {
    Knob,
    Slider,
    Switch,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ControllerBindingAction {
    Macro(MacroId),
    DirectParam {
        id: ParamId,
        scale: ControllerValueScale,
    },
    GfmLayerAmount,
    BcsLayerAmount,
    BcsLayerEnabled,
    Runtime(RuntimeControlMessage),
    ToggleParam(ParamId),
    Reserved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ControllerValueScale {
    Linear,
    Log,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ControllerProfileFile {
    pub(crate) name: Option<String>,
    #[serde(default)]
    pub(crate) binding: Vec<ControllerBindingFile>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ControllerBindingFile {
    pub(crate) control: String,
    pub(crate) cc: u8,
    pub(crate) section: Option<ControllerBindingSection>,
    pub(crate) index: Option<u8>,
    pub(crate) kind: ControllerBindingKind,
    pub(crate) target: Option<String>,
    pub(crate) action: Option<String>,
    pub(crate) slot: Option<usize>,
    pub(crate) scale: Option<ControllerValueScale>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ControllerBindingKind {
    Macro,
    DirectParam,
    GfmLayerAmount,
    BcsLayerAmount,
    BcsLayerEnabled,
    RuntimeAction,
    ToggleParam,
    Reserved,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RuntimeUiCommand {
    Noop,
    Help,
    Status,
    Patches,
    Favorites,
    Favorite(usize),
    Patch(String),
    NextFavorite,
    PrevFavorite,
    DemoPatch,
    Macro(MacroId, f32),
    Panic,
    ResetControllers,
    BcsLayer(Option<BcsScenario>),
    Record { seconds: u64, path: Option<PathBuf> },
    RecordStop,
    AudioList,
    AudioSelect(String),
    MidiList,
    MidiSelect(String),
    Demo,
    Quit,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct TransportMetricsSnapshot {
    pub(crate) queued_frames: usize,
    pub(crate) queue_target_frames: usize,
    pub(crate) write_frames_hint: usize,
    pub(crate) underrun_batches: u64,
    pub(crate) underrun_frames: u64,
    pub(crate) xrun_recoveries: u64,
    pub(crate) overflow_batches: u64,
    pub(crate) overflow_frames: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecordingState {
    Idle,
    Active,
    Finished,
    Error,
}

impl RecordingState {
    pub(crate) fn from_usize(value: usize) -> Self {
        match value {
            1 => Self::Active,
            2 => Self::Finished,
            3 => Self::Error,
            _ => Self::Idle,
        }
    }

    pub(crate) fn as_usize(self) -> usize {
        match self {
            Self::Idle => 0,
            Self::Active => 1,
            Self::Finished => 2,
            Self::Error => 3,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Active => "recording",
            Self::Finished => "done",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordingMetricsSnapshot {
    pub(crate) state: RecordingState,
    pub(crate) path: Option<PathBuf>,
    pub(crate) target_frames: Option<u64>,
    pub(crate) frames_written: u64,
    pub(crate) frames_dropped: u64,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct InputMetricsSnapshot {
    pub(crate) midi_messages: u64,
    pub(crate) last_control: Option<LastControlEvent>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LastControlEvent {
    pub(crate) kind: LastControlKind,
    pub(crate) label: String,
    pub(crate) action: String,
    pub(crate) raw_status: u8,
    pub(crate) channel: Option<u8>,
    pub(crate) raw_value: Option<f32>,
    pub(crate) program: Option<u8>,
    pub(crate) verdict: LastControlVerdict,
    pub(crate) received_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LastControlKind {
    ProfileCc(u8),
    LegacyCc(u8),
    ProgramChange(u8),
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LastControlVerdict {
    Accepted,
    Filtered,
    StartupSuppressed,
    Reserved,
    ReleaseIgnored,
    Ignored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PriorityAction {
    Panic,
    ResetControllers,
}

#[derive(Debug, Default)]
pub(crate) struct TransportMetrics {
    pub(crate) queued_frames: AtomicUsize,
    pub(crate) queue_target_frames: AtomicUsize,
    pub(crate) write_frames_hint: AtomicUsize,
    pub(crate) underrun_batches: AtomicU64,
    pub(crate) underrun_frames: AtomicU64,
    pub(crate) xrun_recoveries: AtomicU64,
    pub(crate) overflow_batches: AtomicU64,
    pub(crate) overflow_frames: AtomicU64,
}

impl TransportMetrics {
    pub(crate) fn snapshot(&self) -> TransportMetricsSnapshot {
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

    pub(crate) fn set_queued_frames(&self, queued_frames: usize) {
        self.queued_frames.store(queued_frames, Ordering::Relaxed);
    }

    pub(crate) fn record_write_request(&self, write_frames: usize) {
        self.write_frames_hint
            .store(write_frames, Ordering::Relaxed);
    }

    pub(crate) fn queue_target_frames(&self, capacity_frames: usize) -> usize {
        let write_frames = round_up_to_render_block(self.write_frames_hint.load(Ordering::Relaxed));
        let target_frames = write_frames
            .max(AUDIO_QUEUE_TARGET_FRAMES)
            .min(capacity_frames);
        self.queue_target_frames
            .store(target_frames, Ordering::Relaxed);
        target_frames
    }

    pub(crate) fn record_underrun(&self, missing_frames: usize) {
        if missing_frames == 0 {
            return;
        }

        self.underrun_batches.fetch_add(1, Ordering::Relaxed);
        self.underrun_frames
            .fetch_add(missing_frames as u64, Ordering::Relaxed);
    }

    pub(crate) fn record_xrun_recovery(&self) {
        self.xrun_recoveries.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_overflow(&self, dropped_frames: usize) {
        if dropped_frames == 0 {
            return;
        }

        self.overflow_batches.fetch_add(1, Ordering::Relaxed);
        self.overflow_frames
            .fetch_add(dropped_frames as u64, Ordering::Relaxed);
    }
}

#[derive(Debug, Default)]
pub(crate) struct RecordingMetrics {
    pub(crate) state: AtomicUsize,
    pub(crate) target_frames: AtomicU64,
    pub(crate) frames_written: AtomicU64,
    pub(crate) frames_dropped: AtomicU64,
    pub(crate) path: Mutex<Option<PathBuf>>,
    pub(crate) error: Mutex<Option<String>>,
}

impl RecordingMetrics {
    pub(crate) fn snapshot(&self) -> RecordingMetricsSnapshot {
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

    pub(crate) fn start(&self, path: PathBuf, target_frames: Option<usize>) {
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

    pub(crate) fn set_frames_written(&self, frames_written: u64) {
        self.frames_written.store(frames_written, Ordering::Relaxed);
    }

    pub(crate) fn record_dropped(&self, dropped_frames: usize) {
        if dropped_frames == 0 {
            return;
        }
        self.frames_dropped
            .fetch_add(dropped_frames as u64, Ordering::Relaxed);
    }

    pub(crate) fn finish_if_active(&self) {
        let _ = self.state.compare_exchange(
            RecordingState::Active.as_usize(),
            RecordingState::Finished.as_usize(),
            Ordering::Relaxed,
            Ordering::Relaxed,
        );
    }

    pub(crate) fn record_error(&self, error_message: String) {
        if let Ok(mut error) = self.error.lock() {
            *error = Some(error_message);
        }
        self.state
            .store(RecordingState::Error.as_usize(), Ordering::Relaxed);
    }
}

#[derive(Debug, Default)]
pub(crate) struct InputMetrics {
    pub(crate) midi_messages: AtomicU64,
    pub(crate) last_control: Mutex<Option<LastControlEvent>>,
}

impl InputMetrics {
    pub(crate) fn snapshot(&self) -> InputMetricsSnapshot {
        InputMetricsSnapshot {
            midi_messages: self.midi_messages.load(Ordering::Relaxed),
            last_control: self
                .last_control
                .lock()
                .ok()
                .and_then(|last_control| last_control.clone()),
        }
    }

    pub(crate) fn record_midi_message(&self) {
        self.midi_messages.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_last_control(&self, event: LastControlEvent) {
        if let Ok(mut last_control) = self.last_control.lock() {
            *last_control = Some(event);
        }
    }
}

impl AlsaPlaybackTuning {
    pub(crate) fn from_play_options(options: &PlayOptions) -> Result<Self> {
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

    pub(crate) fn validate(self) -> Result<Self> {
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
pub(crate) struct PriorityActions {
    pub(crate) panic_requested: AtomicBool,
    pub(crate) reset_controllers_requested: AtomicBool,
}

impl PriorityActions {
    pub(crate) fn request_panic(&self) {
        self.panic_requested.store(true, Ordering::Relaxed);
    }

    pub(crate) fn request_reset_controllers(&self) {
        self.reset_controllers_requested
            .store(true, Ordering::Relaxed);
    }

    pub(crate) fn take_action(&self) -> Option<PriorityAction> {
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

pub(crate) struct AudioRuntime {
    pub(crate) tx: mpsc::Sender<EngineCommand>,
    pub(crate) worker: EngineWorker,
    pub(crate) stream: AlsaPlaybackStream,
    pub(crate) audio_selector: String,
    pub(crate) audio_device_name: String,
    pub(crate) sample_rate_hz: u32,
    pub(crate) channels: usize,
    pub(crate) alsa_tuning: AlsaPlaybackTuning,
    pub(crate) bend_range: f32,
    pub(crate) patch_name: String,
    pub(crate) midi_input_queue: Arc<ArrayQueue<RealtimeMidiMessage>>,
    pub(crate) priority_actions: Arc<PriorityActions>,
    pub(crate) transport_metrics: Arc<TransportMetrics>,
}

pub(crate) struct PreparedAudioRuntime {
    pub(crate) tx: mpsc::Sender<EngineCommand>,
    pub(crate) worker: EngineWorker,
    pub(crate) consumer: Consumer<StereoFrame>,
    pub(crate) selected_device: AlsaOutputDevice,
    pub(crate) audio_selector: String,
    pub(crate) audio_device_name: String,
    pub(crate) sample_rate_hz: u32,
    pub(crate) channels: usize,
    pub(crate) alsa_tuning: AlsaPlaybackTuning,
    pub(crate) bend_range: f32,
    pub(crate) patch_name: String,
    pub(crate) midi_input_queue: Arc<ArrayQueue<RealtimeMidiMessage>>,
    pub(crate) priority_actions: Arc<PriorityActions>,
    pub(crate) transport_metrics: Arc<TransportMetrics>,
}

pub(crate) struct DemoPerformer {
    pub(crate) stop: Arc<AtomicBool>,
    pub(crate) join_handle: Option<JoinHandle<()>>,
}

impl DemoPerformer {
    pub(crate) fn spawn(tx: mpsc::Sender<EngineCommand>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let join_handle = thread::spawn(move || run_demo_performance(tx, thread_stop));
        Self {
            stop,
            join_handle: Some(join_handle),
        }
    }

    pub(crate) fn shutdown(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

pub(crate) enum PerformanceDriver {
    Demo(DemoPerformer),
    Midi(OpenedMidiConnection),
    Idle,
}

impl PerformanceDriver {
    pub(crate) fn is_demo(&self) -> bool {
        matches!(self, Self::Demo(_))
    }

    pub(crate) fn detail(&self) -> String {
        match self {
            Self::Demo(_) => "demo performer active".to_string(),
            Self::Midi(connection) => format!("connected ({})", connection.port_name),
            Self::Idle => "no active input".to_string(),
        }
    }

    pub(crate) fn stop(&mut self) {
        let previous = std::mem::replace(self, Self::Idle);
        previous.shutdown();
    }

    pub(crate) fn shutdown(self) {
        match self {
            Self::Demo(demo) => demo.shutdown(),
            Self::Midi(_) | Self::Idle => {}
        }
    }
}

pub(crate) struct RuntimeSession {
    pub(crate) patch_path: PathBuf,
    pub(crate) patch_name: String,
    pub(crate) audio_selector: Option<String>,
    pub(crate) alsa_tuning: AlsaPlaybackTuning,
    pub(crate) midi_selector: Option<String>,
    pub(crate) midi_channel: Option<u8>,
    pub(crate) controller_profile: Option<Arc<ControllerProfile>>,
    pub(crate) trace_midi: bool,
    pub(crate) gfm_layer_seed: Option<u64>,
    pub(crate) bcs_layer_scenario: Option<BcsScenario>,
    pub(crate) bend_range: f32,
    pub(crate) tx: mpsc::Sender<EngineCommand>,
    pub(crate) midi_input_queue: Arc<ArrayQueue<RealtimeMidiMessage>>,
    pub(crate) runtime_control_queue: Arc<ArrayQueue<RuntimeControlMessage>>,
    pub(crate) priority_actions: Arc<PriorityActions>,
    pub(crate) worker: EngineWorker,
    pub(crate) stream: Option<AlsaPlaybackStream>,
    pub(crate) driver: PerformanceDriver,
    pub(crate) audio_device_name: String,
    pub(crate) sample_rate_hz: u32,
    pub(crate) channels: usize,
    pub(crate) transport_metrics: Arc<TransportMetrics>,
    pub(crate) input_metrics: Arc<InputMetrics>,
    pub(crate) recording_metrics: Arc<RecordingMetrics>,
}

pub(crate) struct EngineWorker {
    pub(crate) join_handle: Option<JoinHandle<()>>,
}

impl EngineWorker {
    pub(crate) fn new(join_handle: JoinHandle<()>) -> Self {
        Self {
            join_handle: Some(join_handle),
        }
    }

    pub(crate) fn shutdown(mut self, tx: mpsc::Sender<EngineCommand>) {
        let _ = tx.send(EngineCommand::Shutdown);
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

pub(crate) struct OpenedAlsaPlayback {
    pub(crate) pcm: PCM,
    pub(crate) audio_selector: String,
    pub(crate) audio_device_name: String,
    pub(crate) sample_rate_hz: u32,
    pub(crate) channels: usize,
    pub(crate) sample_format: AlsaPlaybackSampleFormat,
    pub(crate) tuning: AlsaPlaybackTuning,
}

pub(crate) struct AlsaPlaybackStream {
    pub(crate) stop: Arc<AtomicBool>,
    pub(crate) join_handle: Option<JoinHandle<()>>,
}

impl AlsaPlaybackStream {
    pub(crate) fn spawn(
        opened: OpenedAlsaPlayback,
        consumer: Consumer<StereoFrame>,
        transport_metrics: Arc<TransportMetrics>,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let join_handle = thread::spawn(move || {
            run_alsa_playback_loop(opened, consumer, transport_metrics, thread_stop)
        });
        Self {
            stop,
            join_handle: Some(join_handle),
        }
    }

    pub(crate) fn shutdown(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}
