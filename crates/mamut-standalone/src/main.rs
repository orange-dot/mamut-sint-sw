use std::{
    collections::HashMap,
    env, fs,
    io::{self, BufRead, IsTerminal, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        mpsc::{self, RecvTimeoutError, TryRecvError},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use alsa::{
    Direction, ValueOr,
    pcm::{Access, Format, Frames, HwParams, IO, PCM},
};
use anyhow::{Context, Result, anyhow};
use crossbeam_queue::ArrayQueue;
use eframe::{NativeOptions, egui};
use mamut_dsp::StereoBlockMut;
use mamut_engine::{
    ControllerEvent, Engine, EngineConfig, EngineSnapshot, NoteEvent, ProcessBlock, Scheduled,
};
use mamut_params::{MacroId, ParamId, ParamUnit, param_by_key, param_spec};
use mamut_patch::{PatchFileV1, load_patch_toml, validate_patch_v1};
use midir::{Ignore, MidiInput, MidiInputConnection, MidiInputPort};
use rtrb::{Consumer, Producer, RingBuffer};
use serde::Deserialize;

const ENGINE_RENDER_BLOCK_FRAMES: usize = 256;
const AUDIO_QUEUE_CAPACITY_BLOCKS: usize = 4;
const AUDIO_QUEUE_TARGET_BLOCKS: usize = 2;
const AUDIO_QUEUE_CAPACITY_FRAMES: usize = ENGINE_RENDER_BLOCK_FRAMES * AUDIO_QUEUE_CAPACITY_BLOCKS;
const AUDIO_QUEUE_TARGET_FRAMES: usize = ENGINE_RENDER_BLOCK_FRAMES * AUDIO_QUEUE_TARGET_BLOCKS;
const ENGINE_IDLE_SLEEP: Duration = Duration::from_millis(1);
const ALSA_WAIT_TIMEOUT_MS: u32 = 100;
const ALSA_PLAYBACK_CHANNELS: usize = 2;
const ALSA_PLAYBACK_SAMPLE_RATE_HZ: u32 = 44_100;
const ALSA_PERIOD_FRAMES_DEFAULT: usize = 256;
const ALSA_BUFFER_FRAMES_DEFAULT: usize = 1_024;
const ALSA_START_THRESHOLD_FRAMES_DEFAULT: usize = ALSA_BUFFER_FRAMES_DEFAULT;
const PERFORMANCE_UI_REFRESH: Duration = Duration::from_millis(75);
const MIDI_ACTIVITY_FLASH: Duration = Duration::from_millis(700);
const MIDI_INPUT_QUEUE_CAPACITY: usize = 512;
const RUNTIME_CONTROL_QUEUE_CAPACITY: usize = 64;
const LIVE_SET_STEMS: [&str; 8] = [
    "molten-horizon",
    "cathedral-bloom",
    "ember-vault",
    "razor-thaw",
    "gravity-wake",
    "furnace-choir",
    "granite-plain",
    "glass-tide",
];

type StereoFrame = [f32; 2];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AlsaPlaybackSampleFormat {
    Float32,
    Signed32,
}

impl AlsaPlaybackSampleFormat {
    fn alsa_format(self) -> Format {
        match self {
            Self::Float32 => Format::float(),
            Self::Signed32 => Format::s32(),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Float32 => "F32",
            Self::Signed32 => "S32_LE",
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum RealtimeMidiMessage {
    Note(NoteEvent),
    Controller(ControllerEvent),
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        eprintln!();
        print_usage();
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();

    match args.first().map(String::as_str) {
        Some("list-factory") => list_factory_patches(),
        Some("list-audio") => list_audio_devices(),
        Some("list-midi") => list_midi_devices(),
        Some("validate") => {
            let path = resolve_patch_argument(args.get(1).map(String::as_str))?;
            let patch = load_patch_from_path(&path)?;
            validate_patch_v1(&patch).context("patch validation failed")?;
            println!("valid: {}", path.display());
            Ok(())
        }
        Some("dry-run") => {
            let path = resolve_patch_argument(args.get(1).map(String::as_str))?;
            dry_run(&path)
        }
        Some("play") => {
            let options = parse_play_options(&args[1..])?;
            play(&options)
        }
        Some("help") | Some("--help") | Some("-h") => {
            print_usage();
            Ok(())
        }
        None => dry_run(&default_patch_path()),
        Some(other) => Err(anyhow!("unknown command `{other}`")),
    }
}

fn print_usage() {
    eprintln!(
        "usage:
  mamut-standalone
  mamut-standalone list-factory
  mamut-standalone list-audio
  mamut-standalone list-midi
  mamut-standalone validate [factory-name-or-path]
  mamut-standalone dry-run [factory-name-or-path]
  mamut-standalone play [--demo] [--headless] --audio-device <alsa-index-or-hw:card,device> [--alsa-period-frames <n>] [--alsa-buffer-frames <n>] [--alsa-start-threshold-frames <n>] [--midi-device <name-or-index>] [--midi-channel <1..16>] [--controller-profile <path>] [--trace-midi] [factory-name-or-path]

interactive play controls:
  help
  status
  patches
  favorites
  favorite <0..7>
  patch <factory-name-or-path>
  next
  prev
  demo-patch
  macro <gravitacija|bloom|heat|ruin|swarm> <0..1>
  panic
  reset-controllers
  audio [alsa-index-or-hw:card,device]
  midi [name-or-index]
  demo
  quit"
    );
}

#[derive(Debug, Clone)]
struct FactoryPatchEntry {
    stem: String,
    slug: String,
    path: PathBuf,
    patch_name: String,
    description: Option<String>,
    favorite: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct PlayOptions {
    patch_path: PathBuf,
    force_demo: bool,
    audio_selector: Option<String>,
    alsa_period_frames: Option<usize>,
    alsa_buffer_frames: Option<usize>,
    alsa_start_threshold_frames: Option<usize>,
    midi_selector: Option<String>,
    midi_channel: Option<u8>,
    controller_profile_path: Option<PathBuf>,
    trace_midi: bool,
    headless: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AlsaOutputDevice {
    selector: String,
    card_index: i32,
    device_index: i32,
    card_name: String,
    pcm_name: String,
}

impl AlsaOutputDevice {
    fn display_name(&self) -> String {
        format!("{} ({}, {})", self.selector, self.card_name, self.pcm_name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AlsaPlaybackTuning {
    period_frames: usize,
    buffer_frames: usize,
    start_threshold_frames: usize,
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
struct NamedMidiPort {
    port: MidiInputPort,
    name: String,
}

#[derive(Debug)]
enum EngineCommand {
    Note(NoteEvent),
    Controller(ControllerEvent),
    RequestSnapshot(mpsc::Sender<EngineSnapshot>),
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeControlMessage {
    ProgramChange(u8),
    Panic,
    ResetControllers,
    NextFavorite,
    PrevFavorite,
    FavoriteSlot(usize),
    ToggleParam(ParamId),
}

#[derive(Debug, Clone)]
struct ControllerProfile {
    name: String,
    path: PathBuf,
    bindings_by_cc: HashMap<u8, ControllerBinding>,
}

impl ControllerProfile {
    fn binding_for_cc(&self, cc: u8) -> Option<&ControllerBinding> {
        self.bindings_by_cc.get(&cc)
    }
}

#[derive(Debug, Clone)]
struct ControllerBinding {
    cc: u8,
    control: String,
    section: ControllerBindingSection,
    index: Option<u8>,
    action: ControllerBindingAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ControllerBindingSection {
    Knob,
    Slider,
    Switch,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ControllerBindingAction {
    Macro(MacroId),
    DirectParam {
        id: ParamId,
        scale: ControllerValueScale,
    },
    Runtime(RuntimeControlMessage),
    ToggleParam(ParamId),
    Reserved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ControllerValueScale {
    Linear,
    Log,
}

#[derive(Debug, Deserialize)]
struct ControllerProfileFile {
    name: Option<String>,
    #[serde(default)]
    binding: Vec<ControllerBindingFile>,
}

#[derive(Debug, Deserialize)]
struct ControllerBindingFile {
    control: String,
    cc: u8,
    section: Option<ControllerBindingSection>,
    index: Option<u8>,
    kind: ControllerBindingKind,
    target: Option<String>,
    action: Option<String>,
    slot: Option<usize>,
    scale: Option<ControllerValueScale>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ControllerBindingKind {
    Macro,
    DirectParam,
    RuntimeAction,
    ToggleParam,
    Reserved,
}

#[derive(Debug, Clone, PartialEq)]
enum RuntimeUiCommand {
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
    AudioList,
    AudioSelect(String),
    MidiList,
    MidiSelect(String),
    Demo,
    Quit,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct TransportMetricsSnapshot {
    queued_frames: usize,
    queue_target_frames: usize,
    write_frames_hint: usize,
    underrun_batches: u64,
    underrun_frames: u64,
    xrun_recoveries: u64,
    overflow_batches: u64,
    overflow_frames: u64,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct InputMetricsSnapshot {
    midi_messages: u64,
    last_control: Option<LastControlEvent>,
}

#[derive(Debug, Clone, PartialEq)]
struct LastControlEvent {
    kind: LastControlKind,
    label: String,
    action: String,
    raw_status: u8,
    channel: Option<u8>,
    raw_value: Option<f32>,
    program: Option<u8>,
    verdict: LastControlVerdict,
    received_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LastControlKind {
    ProfileCc(u8),
    LegacyCc(u8),
    ProgramChange(u8),
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LastControlVerdict {
    Accepted,
    Filtered,
    Reserved,
    ReleaseIgnored,
    Ignored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PriorityAction {
    Panic,
    ResetControllers,
}

#[derive(Debug, Default)]
struct TransportMetrics {
    queued_frames: AtomicUsize,
    queue_target_frames: AtomicUsize,
    write_frames_hint: AtomicUsize,
    underrun_batches: AtomicU64,
    underrun_frames: AtomicU64,
    xrun_recoveries: AtomicU64,
    overflow_batches: AtomicU64,
    overflow_frames: AtomicU64,
}

impl TransportMetrics {
    fn snapshot(&self) -> TransportMetricsSnapshot {
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

    fn set_queued_frames(&self, queued_frames: usize) {
        self.queued_frames.store(queued_frames, Ordering::Relaxed);
    }

    fn record_write_request(&self, write_frames: usize) {
        self.write_frames_hint
            .store(write_frames, Ordering::Relaxed);
    }

    fn queue_target_frames(&self, capacity_frames: usize) -> usize {
        let write_frames = round_up_to_render_block(self.write_frames_hint.load(Ordering::Relaxed));
        let target_frames = write_frames
            .max(AUDIO_QUEUE_TARGET_FRAMES)
            .min(capacity_frames);
        self.queue_target_frames
            .store(target_frames, Ordering::Relaxed);
        target_frames
    }

    fn record_underrun(&self, missing_frames: usize) {
        if missing_frames == 0 {
            return;
        }

        self.underrun_batches.fetch_add(1, Ordering::Relaxed);
        self.underrun_frames
            .fetch_add(missing_frames as u64, Ordering::Relaxed);
    }

    fn record_xrun_recovery(&self) {
        self.xrun_recoveries.fetch_add(1, Ordering::Relaxed);
    }

    fn record_overflow(&self, dropped_frames: usize) {
        if dropped_frames == 0 {
            return;
        }

        self.overflow_batches.fetch_add(1, Ordering::Relaxed);
        self.overflow_frames
            .fetch_add(dropped_frames as u64, Ordering::Relaxed);
    }
}

#[derive(Debug, Default)]
struct InputMetrics {
    midi_messages: AtomicU64,
    last_control: Mutex<Option<LastControlEvent>>,
}

impl InputMetrics {
    fn snapshot(&self) -> InputMetricsSnapshot {
        InputMetricsSnapshot {
            midi_messages: self.midi_messages.load(Ordering::Relaxed),
            last_control: self
                .last_control
                .lock()
                .ok()
                .and_then(|last_control| last_control.clone()),
        }
    }

    fn record_midi_message(&self) {
        self.midi_messages.fetch_add(1, Ordering::Relaxed);
    }

    fn record_last_control(&self, event: LastControlEvent) {
        if let Ok(mut last_control) = self.last_control.lock() {
            *last_control = Some(event);
        }
    }
}

impl AlsaPlaybackTuning {
    fn from_play_options(options: &PlayOptions) -> Result<Self> {
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

    fn validate(self) -> Result<Self> {
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
struct PriorityActions {
    panic_requested: AtomicBool,
    reset_controllers_requested: AtomicBool,
}

impl PriorityActions {
    fn request_panic(&self) {
        self.panic_requested.store(true, Ordering::Relaxed);
    }

    fn request_reset_controllers(&self) {
        self.reset_controllers_requested
            .store(true, Ordering::Relaxed);
    }

    fn take_action(&self) -> Option<PriorityAction> {
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

struct AudioRuntime {
    tx: mpsc::Sender<EngineCommand>,
    worker: EngineWorker,
    stream: AlsaPlaybackStream,
    audio_selector: String,
    audio_device_name: String,
    sample_rate_hz: u32,
    channels: usize,
    alsa_tuning: AlsaPlaybackTuning,
    bend_range: f32,
    patch_name: String,
    midi_input_queue: Arc<ArrayQueue<RealtimeMidiMessage>>,
    priority_actions: Arc<PriorityActions>,
    transport_metrics: Arc<TransportMetrics>,
}

struct PreparedAudioRuntime {
    tx: mpsc::Sender<EngineCommand>,
    worker: EngineWorker,
    consumer: Consumer<StereoFrame>,
    selected_device: AlsaOutputDevice,
    audio_selector: String,
    audio_device_name: String,
    sample_rate_hz: u32,
    channels: usize,
    alsa_tuning: AlsaPlaybackTuning,
    bend_range: f32,
    patch_name: String,
    midi_input_queue: Arc<ArrayQueue<RealtimeMidiMessage>>,
    priority_actions: Arc<PriorityActions>,
    transport_metrics: Arc<TransportMetrics>,
}

struct DemoPerformer {
    stop: Arc<AtomicBool>,
    join_handle: Option<JoinHandle<()>>,
}

impl DemoPerformer {
    fn spawn(tx: mpsc::Sender<EngineCommand>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let join_handle = thread::spawn(move || run_demo_performance(tx, thread_stop));
        Self {
            stop,
            join_handle: Some(join_handle),
        }
    }

    fn shutdown(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

enum PerformanceDriver {
    Demo(DemoPerformer),
    Midi(OpenedMidiConnection),
    Idle,
}

impl PerformanceDriver {
    fn is_demo(&self) -> bool {
        matches!(self, Self::Demo(_))
    }

    fn detail(&self) -> String {
        match self {
            Self::Demo(_) => "demo performer active".to_string(),
            Self::Midi(connection) => format!("connected ({})", connection.port_name),
            Self::Idle => "no active input".to_string(),
        }
    }

    fn stop(&mut self) {
        let previous = std::mem::replace(self, Self::Idle);
        previous.shutdown();
    }

    fn shutdown(self) {
        match self {
            Self::Demo(demo) => demo.shutdown(),
            Self::Midi(_) | Self::Idle => {}
        }
    }
}

struct RuntimeSession {
    patch_path: PathBuf,
    patch_name: String,
    audio_selector: Option<String>,
    alsa_tuning: AlsaPlaybackTuning,
    midi_selector: Option<String>,
    midi_channel: Option<u8>,
    controller_profile: Option<Arc<ControllerProfile>>,
    trace_midi: bool,
    bend_range: f32,
    tx: mpsc::Sender<EngineCommand>,
    midi_input_queue: Arc<ArrayQueue<RealtimeMidiMessage>>,
    runtime_control_queue: Arc<ArrayQueue<RuntimeControlMessage>>,
    priority_actions: Arc<PriorityActions>,
    worker: EngineWorker,
    stream: Option<AlsaPlaybackStream>,
    driver: PerformanceDriver,
    audio_device_name: String,
    sample_rate_hz: u32,
    channels: usize,
    transport_metrics: Arc<TransportMetrics>,
    input_metrics: Arc<InputMetrics>,
}

struct EngineWorker {
    join_handle: Option<JoinHandle<()>>,
}

impl EngineWorker {
    fn new(join_handle: JoinHandle<()>) -> Self {
        Self {
            join_handle: Some(join_handle),
        }
    }

    fn shutdown(mut self, tx: mpsc::Sender<EngineCommand>) {
        let _ = tx.send(EngineCommand::Shutdown);
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

struct OpenedAlsaPlayback {
    pcm: PCM,
    audio_selector: String,
    audio_device_name: String,
    sample_rate_hz: u32,
    channels: usize,
    sample_format: AlsaPlaybackSampleFormat,
    tuning: AlsaPlaybackTuning,
}

struct AlsaPlaybackStream {
    stop: Arc<AtomicBool>,
    join_handle: Option<JoinHandle<()>>,
}

impl AlsaPlaybackStream {
    fn spawn(
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

    fn shutdown(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

impl RuntimeSession {
    fn new(options: &PlayOptions) -> Result<Self> {
        let alsa_tuning = AlsaPlaybackTuning::from_play_options(options)?;
        let controller_profile = options
            .controller_profile_path
            .as_deref()
            .map(load_controller_profile)
            .transpose()?
            .map(Arc::new);
        let prepared_runtime = build_audio_runtime(
            &options.patch_path,
            options.audio_selector.as_deref(),
            alsa_tuning,
        )?;
        let runtime = start_prepared_audio_runtime(prepared_runtime)?;
        let input_metrics = Arc::new(InputMetrics::default());
        let runtime_control_queue = Arc::new(ArrayQueue::new(RUNTIME_CONTROL_QUEUE_CAPACITY));
        let mut session = Self {
            patch_path: options.patch_path.clone(),
            patch_name: runtime.patch_name,
            audio_selector: Some(runtime.audio_selector.clone()),
            alsa_tuning: runtime.alsa_tuning,
            midi_selector: options.midi_selector.clone(),
            midi_channel: options.midi_channel,
            controller_profile,
            trace_midi: options.trace_midi,
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
        };
        session.driver = session.open_driver_for_tx(
            session.tx.clone(),
            Arc::clone(&session.midi_input_queue),
            session.bend_range,
            Arc::clone(&session.runtime_control_queue),
            Arc::clone(&session.input_metrics),
            options.force_demo,
        )?;
        Ok(session)
    }

    fn open_driver_for_tx(
        &self,
        tx: mpsc::Sender<EngineCommand>,
        midi_input_queue: Arc<ArrayQueue<RealtimeMidiMessage>>,
        bend_range: f32,
        runtime_control_queue: Arc<ArrayQueue<RuntimeControlMessage>>,
        input_metrics: Arc<InputMetrics>,
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
            input_metrics,
            force_demo,
        )
    }

    fn restore_demo_driver(&mut self) {
        self.driver = PerformanceDriver::Demo(DemoPerformer::spawn(self.tx.clone()));
    }

    fn print_startup_summary(&self) {
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
        println!("interactive controls active; type `help` for commands");
    }

    fn command_loop(&mut self) -> Result<()> {
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

    fn block_forever(&mut self) -> Result<()> {
        loop {
            self.print_runtime_control_messages()?;
            thread::sleep(Duration::from_millis(50));
        }
    }

    fn switch_patch(&mut self, path: PathBuf) -> Result<()> {
        let keep_demo = self.driver.is_demo();
        let runtime = build_audio_runtime(&path, self.audio_selector.as_deref(), self.alsa_tuning)?;
        self.install_runtime(path.clone(), runtime, keep_demo)?;
        println!("loaded patch: {} ({})", self.patch_name, path.display());
        Ok(())
    }

    fn set_macro(&self, id: MacroId, value: f32) -> Result<()> {
        self.tx
            .send(EngineCommand::Controller(ControllerEvent::Macro {
                id,
                value: value.clamp(0.0, 1.0),
            }))
            .map_err(|_| anyhow!("audio runtime is no longer available"))
    }

    fn panic(&self) -> Result<()> {
        self.priority_actions.request_panic();
        Ok(())
    }

    fn reset_controllers(&self) -> Result<()> {
        self.priority_actions.request_reset_controllers();
        Ok(())
    }

    fn toggle_param(&self, id: ParamId) -> Result<f32> {
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

    fn print_status(&self) -> Result<()> {
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
        println!("midi activity: messages={}", input.midi_messages);
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

    fn request_snapshot(&self) -> Result<EngineSnapshot> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(EngineCommand::RequestSnapshot(reply_tx))
            .map_err(|_| anyhow!("audio runtime is no longer available"))?;
        reply_rx
            .recv_timeout(Duration::from_millis(500))
            .map_err(|_| anyhow!("timed out waiting for engine snapshot"))
    }

    fn switch_audio(&mut self, selector: String) -> Result<()> {
        let keep_demo = self.driver.is_demo();
        let runtime =
            build_audio_runtime(&self.patch_path, Some(selector.as_str()), self.alsa_tuning)?;
        self.install_runtime(self.patch_path.clone(), runtime, keep_demo)?;
        println!(
            "audio switched to `{}`",
            self.audio_selector.as_deref().unwrap_or(selector.as_str())
        );
        Ok(())
    }

    fn switch_midi(&mut self, selector: String) -> Result<()> {
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
            Arc::clone(&self.input_metrics),
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
                    Arc::clone(&self.input_metrics),
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
                    Arc::clone(&self.input_metrics),
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

    fn enable_demo(&mut self) {
        let mut old_driver = std::mem::replace(&mut self.driver, PerformanceDriver::Idle);
        old_driver.stop();
        self.restore_demo_driver();
        println!("demo performer enabled");
    }

    fn switch_favorite(&mut self, direction: isize) -> Result<()> {
        let path = adjacent_live_patch(&self.patch_path, direction)?;
        self.switch_patch(path)
    }

    fn load_favorite_slot(&mut self, slot: usize) -> Result<()> {
        let path = live_patch_path(slot)?;
        self.switch_patch(path)
    }

    fn install_runtime(
        &mut self,
        patch_path: PathBuf,
        prepared_runtime: PreparedAudioRuntime,
        keep_demo: bool,
    ) -> Result<()> {
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
                        Arc::clone(&self.input_metrics),
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
            Arc::clone(&self.input_metrics),
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
                    Arc::clone(&self.input_metrics),
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

    fn rebuild_current_runtime_after_stream_release(
        &mut self,
        old_mode_was_demo: bool,
    ) -> Result<()> {
        let prepared = build_audio_runtime(
            &self.patch_path,
            self.audio_selector.as_deref(),
            self.alsa_tuning,
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
            Arc::clone(&self.input_metrics),
            old_mode_was_demo,
        );
        Ok(())
    }

    fn current_live_slot(&self) -> Option<usize> {
        live_slot_for_path(&self.patch_path)
    }

    fn input_metrics_snapshot(&self) -> InputMetricsSnapshot {
        self.input_metrics.snapshot()
    }

    fn transport_metrics_snapshot(&self) -> TransportMetricsSnapshot {
        self.transport_metrics.snapshot()
    }

    fn print_runtime_control_messages(&mut self) -> Result<()> {
        for message in self.poll_runtime_control_messages()? {
            println!("{message}");
        }
        Ok(())
    }

    fn poll_runtime_control_messages(&mut self) -> Result<Vec<String>> {
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

fn open_driver_for_selector(
    tx: mpsc::Sender<EngineCommand>,
    midi_input_queue: Arc<ArrayQueue<RealtimeMidiMessage>>,
    bend_range: f32,
    midi_selector: Option<&str>,
    midi_channel: Option<u8>,
    controller_profile: Option<Arc<ControllerProfile>>,
    trace_midi: bool,
    runtime_control_queue: Arc<ArrayQueue<RuntimeControlMessage>>,
    input_metrics: Arc<InputMetrics>,
    force_demo: bool,
) -> Result<PerformanceDriver> {
    if force_demo {
        return Ok(PerformanceDriver::Demo(DemoPerformer::spawn(tx)));
    }

    if let Some(connection) = open_midi_input(
        midi_input_queue,
        bend_range,
        midi_selector,
        midi_channel,
        controller_profile,
        trace_midi,
        runtime_control_queue,
        input_metrics,
    )? {
        Ok(PerformanceDriver::Midi(connection))
    } else {
        Ok(PerformanceDriver::Demo(DemoPerformer::spawn(tx)))
    }
}

fn restore_driver_state(
    tx: mpsc::Sender<EngineCommand>,
    midi_input_queue: Arc<ArrayQueue<RealtimeMidiMessage>>,
    bend_range: f32,
    midi_selector: Option<&str>,
    midi_channel: Option<u8>,
    controller_profile: Option<Arc<ControllerProfile>>,
    trace_midi: bool,
    runtime_control_queue: Arc<ArrayQueue<RuntimeControlMessage>>,
    input_metrics: Arc<InputMetrics>,
    old_mode_was_demo: bool,
) -> PerformanceDriver {
    if old_mode_was_demo {
        return PerformanceDriver::Demo(DemoPerformer::spawn(tx));
    }

    match open_midi_input(
        midi_input_queue,
        bend_range,
        midi_selector,
        midi_channel,
        controller_profile,
        trace_midi,
        runtime_control_queue,
        input_metrics,
    ) {
        Ok(Some(connection)) => PerformanceDriver::Midi(connection),
        Ok(None) | Err(_) => PerformanceDriver::Idle,
    }
}

fn list_factory_patches() -> Result<()> {
    for entry in factory_patch_entries()? {
        let favorite = if entry.favorite { "*" } else { " " };
        let description = entry
            .description
            .as_deref()
            .unwrap_or("no description")
            .trim();
        println!(
            "{favorite} {:<18} {:<20} {}",
            entry.stem, entry.patch_name, description
        );
    }
    Ok(())
}

fn list_favorite_patches() -> Result<()> {
    for (slot, entry) in favorite_patch_entries()?.iter().enumerate() {
        let description = entry
            .description
            .as_deref()
            .unwrap_or("no description")
            .trim();
        println!(
            "{slot}: {:<18} {:<20} {}",
            entry.stem, entry.patch_name, description
        );
    }
    Ok(())
}

fn list_audio_devices() -> Result<()> {
    let devices = collect_alsa_output_devices()?;
    if devices.is_empty() {
        println!("no ALSA hw playback devices found");
        return Ok(());
    }

    for (index, entry) in devices.iter().enumerate() {
        println!(
            "{index}: {} - {} - {}",
            entry.selector, entry.card_name, entry.pcm_name
        );
    }
    Ok(())
}

fn collect_alsa_output_devices() -> Result<Vec<AlsaOutputDevice>> {
    let card_names = read_alsa_card_names()?;
    let pcm_contents = fs::read_to_string("/proc/asound/pcm")
        .context("failed to read /proc/asound/pcm for ALSA playback enumeration")?;
    let mut devices = Vec::new();

    for line in pcm_contents.lines() {
        let Some((card_index, device_index, pcm_name, has_playback)) = parse_alsa_pcm_line(line)
        else {
            continue;
        };
        if !has_playback {
            continue;
        }

        devices.push(AlsaOutputDevice {
            selector: format!("hw:{card_index},{device_index}"),
            card_index,
            device_index,
            card_name: card_names
                .get(&card_index)
                .cloned()
                .unwrap_or_else(|| format!("card {card_index}")),
            pcm_name,
        });
    }

    devices.sort_by(|left, right| {
        (
            left.card_index,
            left.device_index,
            left.card_name.as_str(),
            left.pcm_name.as_str(),
        )
            .cmp(&(
                right.card_index,
                right.device_index,
                right.card_name.as_str(),
                right.pcm_name.as_str(),
            ))
    });
    Ok(devices)
}

fn read_alsa_card_names() -> Result<HashMap<i32, String>> {
    let contents = fs::read_to_string("/proc/asound/cards")
        .context("failed to read /proc/asound/cards for ALSA card enumeration")?;
    let mut cards = HashMap::new();

    for line in contents.lines() {
        let Some((card_index, card_name)) = parse_alsa_card_line(line) else {
            continue;
        };
        cards.insert(card_index, card_name);
    }

    Ok(cards)
}

fn parse_alsa_card_line(line: &str) -> Option<(i32, String)> {
    let trimmed = line.trim_start();
    let index_end = trimmed.find(char::is_whitespace)?;
    let card_index = trimmed[..index_end].parse::<i32>().ok()?;
    let bracket_start = trimmed.find('[')?;
    let bracket_end = trimmed[bracket_start + 1..].find(']')? + bracket_start + 1;
    let short_name = trimmed[bracket_start + 1..bracket_end].trim();
    let description = trimmed[bracket_end + 1..].strip_prefix(':')?.trim();
    let card_name = description
        .rsplit_once(" - ")
        .map(|(_, display)| display.trim())
        .filter(|display| !display.is_empty())
        .unwrap_or(short_name);
    Some((card_index, card_name.to_string()))
}

fn parse_alsa_pcm_line(line: &str) -> Option<(i32, i32, String, bool)> {
    let (prefix, rest) = line.split_once(':')?;
    let (card, device) = prefix.split_once('-')?;
    let card_index = card.trim().parse::<i32>().ok()?;
    let device_index = device.trim().parse::<i32>().ok()?;
    let segments = rest.split(" : ").map(str::trim).collect::<Vec<_>>();
    if segments.len() < 3 {
        return None;
    }

    let pcm_name = segments[1].to_string();
    let has_playback = segments
        .iter()
        .skip(2)
        .any(|segment| segment.contains("playback"));
    Some((card_index, device_index, pcm_name, has_playback))
}

fn select_alsa_output_device(selector: Option<&str>) -> Result<AlsaOutputDevice> {
    let devices = collect_alsa_output_devices()?;
    if devices.is_empty() {
        return Err(anyhow!("no ALSA hw playback devices available"));
    }

    let selector = selector.context(
        "audio output device is required; run `mamut-standalone list-audio` and choose an ALSA `hw:<card>,<device>` selector",
    )?;

    if let Ok(index) = selector.parse::<usize>() {
        return devices
            .into_iter()
            .nth(index)
            .ok_or_else(|| {
                anyhow!(
                    "audio output device index {index} is out of range; run `mamut-standalone list-audio`"
                )
            });
    }

    let normalized = normalize_alsa_hw_selector(selector)?;
    devices
        .into_iter()
        .find(|device| device.selector == normalized)
        .ok_or_else(|| anyhow!("no ALSA hw playback device matched `{normalized}`"))
}

fn normalize_alsa_hw_selector(selector: &str) -> Result<String> {
    let trimmed = selector.trim();
    let Some(rest) = trimmed.strip_prefix("hw:") else {
        return Err(anyhow!(
            "invalid audio selector `{selector}`; expected a list index or ALSA `hw:<card>,<device>`"
        ));
    };

    let (card, device) = rest
        .split_once(',')
        .context("ALSA hw selector must look like `hw:<card>,<device>`")?;
    if device.contains(',') {
        return Err(anyhow!(
            "invalid ALSA hw selector `{selector}`; expected exactly one card/device pair"
        ));
    }

    let card_index = card
        .parse::<i32>()
        .with_context(|| format!("invalid ALSA card index `{card}`"))?;
    let device_index = device
        .parse::<i32>()
        .with_context(|| format!("invalid ALSA device index `{device}`"))?;

    if card_index < 0 || device_index < 0 {
        return Err(anyhow!(
            "invalid ALSA hw selector `{selector}`; card and device indexes must be non-negative"
        ));
    }

    Ok(format!("hw:{card_index},{device_index}"))
}

fn same_hw_selector(current: Option<&str>, replacement: &str) -> bool {
    let Some(current) = current else {
        return false;
    };
    normalize_alsa_hw_selector(current).ok() == normalize_alsa_hw_selector(replacement).ok()
}

fn open_alsa_playback_device(
    selected_device: &AlsaOutputDevice,
    tuning: AlsaPlaybackTuning,
) -> Result<OpenedAlsaPlayback> {
    let pcm =
        PCM::new(&selected_device.selector, Direction::Playback, false).with_context(|| {
            format!(
                "failed to open ALSA playback device {}",
                selected_device.selector
            )
        })?;

    let sample_format;
    {
        let hwp = HwParams::any(&pcm).context("failed to allocate ALSA hw params")?;
        hwp.set_rate_resample(false)
            .context("failed to disable ALSA resampling")?;
        hwp.set_channels(ALSA_PLAYBACK_CHANNELS as u32)
            .context("failed to set ALSA channel count")?;
        hwp.set_rate(ALSA_PLAYBACK_SAMPLE_RATE_HZ, ValueOr::Nearest)
            .context("failed to set ALSA sample rate")?;
        sample_format =
            select_alsa_playback_sample_format(&hwp, selected_device.selector.as_str())?;
        hwp.set_access(Access::RWInterleaved)
            .context("failed to set ALSA access mode")?;
        hwp.set_period_size(tuning.period_frames as Frames, ValueOr::Nearest)
            .context("failed to set ALSA period size")?;
        hwp.set_buffer_size(tuning.buffer_frames as Frames)
            .context("failed to set ALSA buffer size")?;
        pcm.hw_params(&hwp)
            .context("failed to apply ALSA hw params")?;
    }

    let (
        applied_rate,
        applied_channels,
        applied_format,
        applied_period_frames,
        applied_buffer_frames,
    ) = {
        let applied_hwp = pcm
            .hw_params_current()
            .context("failed to read applied ALSA hw params")?;
        let applied_rate = applied_hwp
            .get_rate()
            .context("failed to read ALSA sample rate")?;
        let applied_channels = applied_hwp
            .get_channels()
            .context("failed to read ALSA channel count")? as usize;
        let applied_format = applied_hwp
            .get_format()
            .context("failed to read ALSA sample format")?;
        let applied_period_frames = applied_hwp
            .get_period_size()
            .context("failed to read ALSA period size")?
            as usize;
        let applied_buffer_frames = applied_hwp
            .get_buffer_size()
            .context("failed to read ALSA buffer size")?
            as usize;
        (
            applied_rate,
            applied_channels,
            applied_format,
            applied_period_frames,
            applied_buffer_frames,
        )
    };

    if applied_rate != ALSA_PLAYBACK_SAMPLE_RATE_HZ {
        return Err(anyhow!(
            "ALSA device {} applied {} Hz instead of requested {} Hz",
            selected_device.selector,
            applied_rate,
            ALSA_PLAYBACK_SAMPLE_RATE_HZ
        ));
    }
    if applied_channels != ALSA_PLAYBACK_CHANNELS {
        return Err(anyhow!(
            "ALSA device {} applied {} channels instead of requested {}",
            selected_device.selector,
            applied_channels,
            ALSA_PLAYBACK_CHANNELS
        ));
    }
    if applied_format != sample_format.alsa_format() {
        return Err(anyhow!(
            "ALSA device {} applied {:?} instead of requested {:?}",
            selected_device.selector,
            applied_format,
            sample_format.alsa_format()
        ));
    }
    if applied_period_frames != tuning.period_frames {
        return Err(anyhow!(
            "ALSA device {} applied period {} instead of requested {}",
            selected_device.selector,
            applied_period_frames,
            tuning.period_frames
        ));
    }
    if applied_buffer_frames != tuning.buffer_frames {
        return Err(anyhow!(
            "ALSA device {} applied buffer {} instead of requested {}",
            selected_device.selector,
            applied_buffer_frames,
            tuning.buffer_frames
        ));
    }

    {
        let swp = pcm
            .sw_params_current()
            .context("failed to read ALSA sw params")?;
        swp.set_avail_min(tuning.period_frames as Frames)
            .context("failed to set ALSA avail_min")?;
        swp.set_start_threshold(tuning.start_threshold_frames as Frames)
            .context("failed to set ALSA start threshold")?;
        pcm.sw_params(&swp)
            .context("failed to apply ALSA sw params")?;
    }
    pcm.prepare().context("failed to prepare ALSA PCM")?;

    Ok(OpenedAlsaPlayback {
        pcm,
        audio_selector: selected_device.selector.clone(),
        audio_device_name: selected_device.display_name(),
        sample_rate_hz: ALSA_PLAYBACK_SAMPLE_RATE_HZ,
        channels: ALSA_PLAYBACK_CHANNELS,
        sample_format,
        tuning,
    })
}

fn select_alsa_playback_sample_format(
    hwp: &HwParams<'_>,
    selector: &str,
) -> Result<AlsaPlaybackSampleFormat> {
    for candidate in [
        AlsaPlaybackSampleFormat::Float32,
        AlsaPlaybackSampleFormat::Signed32,
    ] {
        if hwp.test_format(candidate.alsa_format()).is_ok() {
            hwp.set_format(candidate.alsa_format()).with_context(|| {
                format!(
                    "failed to set ALSA sample format {} on {selector}",
                    candidate.label()
                )
            })?;
            return Ok(candidate);
        }
    }

    Err(anyhow!(
        "ALSA device {selector} supports neither F32 nor S32_LE playback sample format"
    ))
}

fn list_midi_devices() -> Result<()> {
    let midi_input = match MidiInput::new("mamut-standalone") {
        Ok(midi_input) => midi_input,
        Err(error) => {
            println!("MIDI support unavailable: {error}");
            return Ok(());
        }
    };
    let ports = collect_midi_ports(&midi_input)?;
    if ports.is_empty() {
        println!("no MIDI input devices found");
        return Ok(());
    }

    for (index, port) in ports.iter().enumerate() {
        println!("{index}: {}", port.name);
    }
    Ok(())
}

fn dry_run(path: &Path) -> Result<()> {
    let patch = load_patch_from_path(path)?;
    validate_patch_v1(&patch).context("patch validation failed")?;
    let mut engine = Engine::new(EngineConfig::default(), patch)?;
    let mut left = vec![0.0_f32; 256];
    let mut right = vec![0.0_f32; 256];

    let note_events = [
        Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: 60,
                velocity: 0.88,
            },
        },
        Scheduled {
            frame_offset: 200,
            event: NoteEvent::NoteOff { note: 60 },
        },
    ];
    let controller_events = [
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::ModWheel { amount: 0.45 },
        },
        Scheduled {
            frame_offset: 64,
            event: ControllerEvent::ChannelAftertouch { pressure: 0.30 },
        },
        Scheduled {
            frame_offset: 96,
            event: ControllerEvent::Macro {
                id: MacroId::Gravitacija,
                value: 0.64,
            },
        },
        Scheduled {
            frame_offset: 160,
            event: ControllerEvent::Macro {
                id: MacroId::Ruin,
                value: 0.42,
            },
        },
    ];

    engine.process_block(ProcessBlock {
        frame_count: 256,
        note_events: &note_events,
        controller_events: &controller_events,
        macro_state: None,
        output: Some(StereoBlockMut::new(&mut left, &mut right)),
    });

    let snapshot = engine.snapshot();
    println!("patch: {} ({})", snapshot.patch_name, path.display());
    println!(
        "description: {}",
        snapshot.patch_description.unwrap_or_default()
    );
    println!("active voices: {}", snapshot.active_voice_count);
    println!("held notes: {:?}", snapshot.held_notes);
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
        "derived: mass={:.3} strain={:.3} headroom={:.3} rupture_threshold={:.3}",
        snapshot.derived.mass,
        snapshot.derived.strain,
        snapshot.derived.headroom,
        snapshot.derived.rupture_threshold
    );
    println!(
        "direct: cutoff_hz={:.2} sync={:.3} crossmod={:.3} body_drive={:.3} peak={:.3}",
        snapshot.direct.cutoff_hz,
        snapshot.direct.sync_amount,
        snapshot.direct.crossmod_amount,
        snapshot.direct.body_drive,
        snapshot.peak_output
    );

    Ok(())
}

fn play(options: &PlayOptions) -> Result<()> {
    let mut session = RuntimeSession::new(options)?;
    if !options.headless && display_available() {
        run_performance_window(session)
    } else if io::stdin().is_terminal() {
        session.command_loop()
    } else {
        session.print_startup_summary();
        println!("stdin is not a terminal; running without interactive controls");
        session.block_forever()
    }
}

fn display_available() -> bool {
    env::var_os("DISPLAY").is_some() || env::var_os("WAYLAND_DISPLAY").is_some()
}

struct PerformanceApp {
    session: RuntimeSession,
    snapshot: Option<EngineSnapshot>,
    selected_tab: PerformanceTab,
    last_snapshot_error: Option<String>,
    last_status_message: Option<String>,
    last_snapshot_refresh: Instant,
    last_midi_message_count: u64,
    midi_hot_until: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PerformanceTab {
    Live,
    Pc4,
    Debug,
}

impl PerformanceTab {
    const ALL: [Self; 3] = [Self::Live, Self::Pc4, Self::Debug];

    fn label(self) -> &'static str {
        match self {
            Self::Live => "Live",
            Self::Pc4 => "PC4",
            Self::Debug => "Debug",
        }
    }
}

impl PerformanceApp {
    fn new(session: RuntimeSession) -> Self {
        let snapshot = session.request_snapshot().ok();
        let last_midi_message_count = session.input_metrics_snapshot().midi_messages;
        Self {
            session,
            snapshot,
            selected_tab: PerformanceTab::Live,
            last_snapshot_error: None,
            last_status_message: None,
            last_snapshot_refresh: Instant::now()
                .checked_sub(PERFORMANCE_UI_REFRESH)
                .unwrap_or_else(Instant::now),
            last_midi_message_count,
            midi_hot_until: Instant::now(),
        }
    }

    fn poll_runtime(&mut self) {
        match self.session.poll_runtime_control_messages() {
            Ok(messages) => {
                if let Some(message) = messages.last() {
                    self.last_status_message = Some(message.clone());
                }
            }
            Err(error) => {
                self.last_status_message = Some(format!("runtime control error: {error}"));
            }
        }

        let input = self.session.input_metrics_snapshot();
        if input.midi_messages != self.last_midi_message_count {
            self.last_midi_message_count = input.midi_messages;
            self.midi_hot_until = Instant::now() + MIDI_ACTIVITY_FLASH;
        }

        if self.last_snapshot_refresh.elapsed() >= PERFORMANCE_UI_REFRESH {
            match self.session.request_snapshot() {
                Ok(snapshot) => {
                    self.snapshot = Some(snapshot);
                    self.last_snapshot_error = None;
                }
                Err(error) => {
                    self.last_snapshot_error = Some(error.to_string());
                }
            }
            self.last_snapshot_refresh = Instant::now();
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.input(|input| {
            if input.key_pressed(egui::Key::ArrowLeft) {
                self.run_action(|session| session.switch_favorite(-1), "previous live slot");
            }
            if input.key_pressed(egui::Key::ArrowRight) {
                self.run_action(|session| session.switch_favorite(1), "next live slot");
            }
            if input.key_pressed(egui::Key::P) {
                self.run_action(|session| session.panic(), "panic");
            }
            if input.key_pressed(egui::Key::R) {
                self.run_action(|session| session.reset_controllers(), "reset controllers");
            }
            for (key, slot) in [
                (egui::Key::Num0, 0_usize),
                (egui::Key::Num1, 1),
                (egui::Key::Num2, 2),
                (egui::Key::Num3, 3),
                (egui::Key::Num4, 4),
                (egui::Key::Num5, 5),
                (egui::Key::Num6, 6),
                (egui::Key::Num7, 7),
            ] {
                if input.key_pressed(key) {
                    self.run_action(
                        |session| session.load_favorite_slot(slot),
                        &format!("live slot {slot}"),
                    );
                }
            }
        });
    }

    fn run_action<F>(&mut self, action: F, label: &str) -> bool
    where
        F: FnOnce(&mut RuntimeSession) -> Result<()>,
    {
        match action(&mut self.session) {
            Ok(()) => {
                self.last_status_message = Some(format!("{label} ok"));
                true
            }
            Err(error) => {
                self.last_status_message = Some(format!("{label} failed: {error}"));
                false
            }
        }
    }

    fn render_header(&mut self, ui: &mut egui::Ui) {
        if let Some(snapshot) = &self.snapshot {
            ui.heading(&snapshot.patch_name);
            if let Some(description) = &snapshot.patch_description {
                ui.label(description);
            }
        } else {
            ui.heading("EPM1 Performance");
        }

        let current_slot = self
            .session
            .current_live_slot()
            .map(|slot| slot.to_string())
            .unwrap_or_else(|| "-".to_string());
        ui.label(format!(
            "Live slot: {current_slot} / {}",
            LIVE_SET_STEMS.len().saturating_sub(1)
        ));
        ui.label(format!(
            "Audio: {} @ {} Hz / {} ch",
            self.session.audio_device_name, self.session.sample_rate_hz, self.session.channels
        ));
        ui.label(format!("MIDI: {}", self.session.driver.detail()));

        let midi_color = if Instant::now() <= self.midi_hot_until {
            egui::Color32::from_rgb(64, 220, 120)
        } else {
            egui::Color32::from_rgb(90, 90, 90)
        };
        ui.colored_label(
            midi_color,
            format!(
                "MIDI activity {}",
                self.session.input_metrics_snapshot().midi_messages
            ),
        );

        let transport = self.session.transport_metrics_snapshot();
        ui.label(format!(
            "Queued {} / Target {} / Write {}  Underruns {}  XRuns {}  Overflows {}",
            transport.queued_frames,
            transport.queue_target_frames,
            transport.write_frames_hint,
            transport.underrun_batches,
            transport.xrun_recoveries,
            transport.overflow_batches
        ));

        if let Some(message) = &self.last_status_message {
            ui.separator();
            ui.label(message);
        }
        if let Some(error) = &self.last_snapshot_error {
            ui.colored_label(egui::Color32::from_rgb(220, 90, 90), error);
        }
    }

    fn render_slot_buttons(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (slot, stem) in LIVE_SET_STEMS.iter().enumerate() {
                let selected = self.session.current_live_slot() == Some(slot);
                let label = format!("{slot}:{stem}");
                if ui.selectable_label(selected, label).clicked() {
                    self.run_action(
                        |session| session.load_favorite_slot(slot),
                        &format!("live slot {slot}"),
                    );
                }
            }
        });
    }

    fn render_macro_controls(&mut self, ui: &mut egui::Ui) {
        let Some(snapshot) = self.snapshot.as_ref() else {
            ui.label("waiting for engine snapshot...");
            return;
        };

        let macro_values = [
            (
                MacroId::Gravitacija,
                "Gravitacija",
                snapshot.live_macros.gravitacija,
                snapshot.effective_macros.gravitacija,
            ),
            (
                MacroId::Bloom,
                "Bloom",
                snapshot.live_macros.bloom,
                snapshot.effective_macros.bloom,
            ),
            (
                MacroId::Heat,
                "Heat",
                snapshot.live_macros.heat,
                snapshot.effective_macros.heat,
            ),
            (
                MacroId::Ruin,
                "Ruin",
                snapshot.live_macros.ruin,
                snapshot.effective_macros.ruin,
            ),
            (
                MacroId::Swarm,
                "Swarm",
                snapshot.live_macros.swarm,
                snapshot.effective_macros.swarm,
            ),
        ];

        ui.horizontal(|ui| {
            for (_id, label, live_value, effective_value) in macro_values {
                ui.vertical(|ui| {
                    ui.strong(label);
                    let mut value = live_value;
                    ui.add_enabled_ui(false, |ui| {
                        ui.add_sized(
                            [84.0, 220.0],
                            egui::Slider::new(&mut value, 0.0..=1.0)
                                .vertical()
                                .show_value(false),
                        );
                    });
                    ui.label(format!("live {:.2}", live_value));
                    ui.small(format!("eff {:.2}", effective_value));
                });
            }
        });
    }

    fn render_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            for tab in PerformanceTab::ALL {
                if ui
                    .selectable_label(self.selected_tab == tab, tab.label())
                    .clicked()
                {
                    self.selected_tab = tab;
                }
            }
        });
    }

    fn render_live_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        self.render_header(ui);
        ui.separator();
        self.render_slot_buttons(ui);
        ui.separator();
        self.render_macro_controls(ui);
        ui.separator();
        self.render_footer(ui, ctx);
    }

    fn render_pc4_tab(&mut self, ui: &mut egui::Ui) {
        let input = self.session.input_metrics_snapshot();
        if let Some(event) = &input.last_control {
            ui.label(format!(
                "Latest: {} -> {} ({})",
                event.label,
                event.action,
                verdict_label(event.verdict)
            ));
        } else {
            ui.label("Latest: none");
        }
        ui.separator();

        let Some(snapshot) = self.snapshot.as_ref() else {
            ui.label("waiting for engine snapshot...");
            return;
        };

        if let Some(profile) = self.session.controller_profile.clone() {
            ui.label(format!(
                "Profile: {} ({})",
                profile.name,
                profile.path.display()
            ));
            ui.separator();
            self.render_controller_section(
                ui,
                "Knobs",
                &profile,
                ControllerBindingSection::Knob,
                snapshot,
                input.last_control.as_ref(),
            );
            ui.separator();
            self.render_controller_section(
                ui,
                "Sliders",
                &profile,
                ControllerBindingSection::Slider,
                snapshot,
                input.last_control.as_ref(),
            );
            ui.separator();
            self.render_controller_section(
                ui,
                "Switches",
                &profile,
                ControllerBindingSection::Switch,
                snapshot,
                input.last_control.as_ref(),
            );
            let other = sorted_bindings_for_section(&profile, ControllerBindingSection::Other);
            if !other.is_empty() {
                ui.separator();
                self.render_binding_grid(
                    ui,
                    "Other",
                    &other,
                    snapshot,
                    input.last_control.as_ref(),
                );
            }
        } else {
            self.render_legacy_pc4_tab(ui, snapshot, input.last_control.as_ref());
        }

        ui.separator();
        self.render_program_change_map(ui, input.last_control.as_ref());
    }

    fn render_controller_section(
        &self,
        ui: &mut egui::Ui,
        title: &str,
        profile: &ControllerProfile,
        section: ControllerBindingSection,
        snapshot: &EngineSnapshot,
        last_control: Option<&LastControlEvent>,
    ) {
        let bindings = sorted_bindings_for_section(profile, section);
        self.render_binding_grid(ui, title, &bindings, snapshot, last_control);
    }

    fn render_binding_grid(
        &self,
        ui: &mut egui::Ui,
        title: &str,
        bindings: &[&ControllerBinding],
        snapshot: &EngineSnapshot,
        last_control: Option<&LastControlEvent>,
    ) {
        ui.strong(title);
        egui::Grid::new(format!("pc4-{title}"))
            .num_columns(3)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                for (index, binding) in bindings.iter().enumerate() {
                    render_binding_tile(ui, binding, snapshot, last_control);
                    if index % 3 == 2 {
                        ui.end_row();
                    }
                }
            });
    }

    fn render_legacy_pc4_tab(
        &self,
        ui: &mut egui::Ui,
        snapshot: &EngineSnapshot,
        last_control: Option<&LastControlEvent>,
    ) {
        ui.colored_label(
            egui::Color32::from_rgb(230, 190, 80),
            "Full PC4 profile not loaded; showing legacy live map.",
        );
        let legacy = [
            (
                16,
                "Legacy CC16",
                "macro Gravitacija",
                snapshot.live_macros.gravitacija,
            ),
            (17, "Legacy CC17", "macro Bloom", snapshot.live_macros.bloom),
            (18, "Legacy CC18", "macro Heat", snapshot.live_macros.heat),
            (19, "Legacy CC19", "macro Ruin", snapshot.live_macros.ruin),
            (20, "Legacy CC20", "macro Swarm", snapshot.live_macros.swarm),
        ];
        egui::Grid::new("pc4-legacy-macros")
            .num_columns(5)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                for (cc, label, action, value) in legacy {
                    let highlighted =
                        last_control.is_some_and(|event| event_matches_legacy_cc(event, cc));
                    render_small_status_tile(
                        ui,
                        highlighted,
                        label,
                        &format!("CC{cc}"),
                        action,
                        &format!("{value:.2}"),
                    );
                }
            });
        ui.separator();
        ui.label("Also active: CC1 mod wheel, CC64 sustain, channel aftertouch, pitch bend.");
    }

    fn render_program_change_map(
        &self,
        ui: &mut egui::Ui,
        last_control: Option<&LastControlEvent>,
    ) {
        ui.strong("Program Change");
        egui::Grid::new("pc4-program-change")
            .num_columns(4)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                for (slot, stem) in LIVE_SET_STEMS.iter().enumerate() {
                    let highlighted = last_control
                        .is_some_and(|event| event_matches_program_change(event, slot as u8));
                    render_small_status_tile(
                        ui,
                        highlighted,
                        &format!("PC {slot}"),
                        "program",
                        stem,
                        if self.session.current_live_slot() == Some(slot) {
                            "current"
                        } else {
                            ""
                        },
                    );
                    if slot % 4 == 3 {
                        ui.end_row();
                    }
                }
            });
    }

    fn render_debug_tab(&mut self, ui: &mut egui::Ui) {
        let input = self.session.input_metrics_snapshot();
        ui.label(format!("Driver: {}", self.session.driver.detail()));
        match self.session.midi_channel {
            Some(channel) => ui.label(format!("MIDI channel filter: channel {channel}")),
            None => ui.label("MIDI channel filter: all channels"),
        };
        if let Some(profile) = &self.session.controller_profile {
            ui.label(format!(
                "Profile: {} ({})",
                profile.name,
                profile.path.display()
            ));
        } else {
            ui.label("Profile: none (legacy live map)");
        }
        ui.label(format!("MIDI messages: {}", input.midi_messages));
        if let Some(event) = &input.last_control {
            ui.separator();
            ui.label(format!("Latest event: {} -> {}", event.label, event.action));
            ui.label(format!(
                "Raw status: 0x{:02X}  Channel: {}  Verdict: {}",
                event.raw_status,
                event
                    .channel
                    .map(|channel| channel.to_string())
                    .unwrap_or_else(|| "system".to_string()),
                verdict_label(event.verdict)
            ));
            if let Some(value) = event.raw_value {
                ui.label(format!("Raw value: {:.3}", value));
            }
            if let Some(program) = event.program {
                ui.label(format!("Program: {program}"));
            }
        }

        ui.separator();
        let transport = self.session.transport_metrics_snapshot();
        ui.label(format!(
            "ALSA queue: queued={} target={} write_hint={}",
            transport.queued_frames, transport.queue_target_frames, transport.write_frames_hint
        ));
        ui.label(format!(
            "Underruns: batches={} frames={}  XRuns={}  Overflows: batches={} frames={}",
            transport.underrun_batches,
            transport.underrun_frames,
            transport.xrun_recoveries,
            transport.overflow_batches,
            transport.overflow_frames
        ));
        if let Some(message) = &self.last_status_message {
            ui.separator();
            ui.label(format!("Runtime status: {message}"));
        }
        if let Some(error) = &self.last_snapshot_error {
            ui.colored_label(egui::Color32::from_rgb(220, 90, 90), error);
        }
    }

    fn render_footer(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            if ui.button("Prev").clicked() {
                self.run_action(|session| session.switch_favorite(-1), "previous live slot");
            }
            if ui.button("Next").clicked() {
                self.run_action(|session| session.switch_favorite(1), "next live slot");
            }
            if ui
                .add(
                    egui::Button::new("PANIC")
                        .fill(egui::Color32::from_rgb(180, 40, 40))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::WHITE)),
                )
                .clicked()
            {
                self.run_action(|session| session.panic(), "panic");
            }
            if ui.button("Reset Ctrls").clicked() {
                self.run_action(|session| session.reset_controllers(), "reset controllers");
            }
            if ui.button("Quit").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });

        if let Some(snapshot) = &self.snapshot {
            ui.separator();
            ui.label(format!(
                "Voices {}  Sustain {}  Held {:?}",
                snapshot.active_voice_count, snapshot.sustain_down, snapshot.held_notes
            ));
            let peak_color = if snapshot.clip_detected {
                egui::Color32::from_rgb(220, 80, 80)
            } else {
                egui::Color32::from_rgb(110, 190, 255)
            };
            ui.colored_label(
                peak_color,
                format!(
                    "Peak {:.3}  Clip {}",
                    snapshot.peak_output, snapshot.clip_detected
                ),
            );
        }
    }
}

impl eframe::App for PerformanceApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_shortcuts(ctx);
        self.poll_runtime();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("EPM1 PC4 Performance Rig");
            ui.separator();
            self.render_tabs(ui);
            ui.separator();
            match self.selected_tab {
                PerformanceTab::Live => self.render_live_tab(ui, ctx),
                PerformanceTab::Pc4 => self.render_pc4_tab(ui),
                PerformanceTab::Debug => self.render_debug_tab(ui),
            }
        });

        ctx.request_repaint_after(Duration::from_millis(16));
    }
}

fn sorted_bindings_for_section(
    profile: &ControllerProfile,
    section: ControllerBindingSection,
) -> Vec<&ControllerBinding> {
    let mut bindings = profile
        .bindings_by_cc
        .values()
        .filter(|binding| binding.section == section)
        .collect::<Vec<_>>();
    bindings.sort_by_key(|binding| {
        (
            binding.index.unwrap_or(u8::MAX),
            binding.control.clone(),
            binding.cc,
        )
    });
    bindings
}

fn render_binding_tile(
    ui: &mut egui::Ui,
    binding: &ControllerBinding,
    snapshot: &EngineSnapshot,
    last_control: Option<&LastControlEvent>,
) {
    let highlighted = last_control.is_some_and(|event| event_matches_binding(event, binding));
    let (value, secondary) = binding_display_value(snapshot, binding);
    let value = secondary
        .map(|secondary| format!("{value}  {secondary}"))
        .unwrap_or(value);
    render_small_status_tile(
        ui,
        highlighted,
        &binding.control,
        &format!("CC{}", binding.cc),
        &describe_binding_action(binding.action),
        &value,
    );
}

fn render_small_status_tile(
    ui: &mut egui::Ui,
    highlighted: bool,
    title: &str,
    subtitle: &str,
    action: &str,
    value: &str,
) {
    let fill = if highlighted {
        egui::Color32::from_rgb(56, 84, 48)
    } else {
        egui::Color32::from_rgb(32, 32, 32)
    };
    egui::Frame::group(ui.style()).fill(fill).show(ui, |ui| {
        ui.set_min_width(150.0);
        ui.strong(title);
        ui.small(subtitle);
        ui.label(action);
        if !value.is_empty() {
            ui.label(value);
        }
    });
}

fn binding_display_value(
    snapshot: &EngineSnapshot,
    binding: &ControllerBinding,
) -> (String, Option<String>) {
    match binding.action {
        ControllerBindingAction::Macro(id) => {
            let live = macro_snapshot_value(snapshot, id, false);
            let effective = macro_snapshot_value(snapshot, id, true);
            (format!("{live:.2}"), Some(format!("eff {effective:.2}")))
        }
        ControllerBindingAction::DirectParam { id, .. } => (
            direct_param_display_value(snapshot, id).unwrap_or_else(|| "not exposed".to_string()),
            None,
        ),
        ControllerBindingAction::Runtime(_) => ("trigger".to_string(), None),
        ControllerBindingAction::ToggleParam(id) => {
            (toggle_param_display_value(snapshot, id), None)
        }
        ControllerBindingAction::Reserved => ("Reserved".to_string(), None),
    }
}

fn macro_snapshot_value(snapshot: &EngineSnapshot, id: MacroId, effective: bool) -> f32 {
    let macros = if effective {
        snapshot.effective_macros
    } else {
        snapshot.live_macros
    };
    match id {
        MacroId::Gravitacija => macros.gravitacija,
        MacroId::Bloom => macros.bloom,
        MacroId::Heat => macros.heat,
        MacroId::Ruin => macros.ruin,
        MacroId::Swarm => macros.swarm,
    }
}

fn toggle_param_display_value(snapshot: &EngineSnapshot, id: ParamId) -> String {
    match id {
        ParamId::ChorusEnabled => on_off(snapshot.direct.chorus_enabled),
        ParamId::ReverbEnabled => on_off(snapshot.direct.reverb_enabled),
        _ => "unsupported".to_string(),
    }
}

fn on_off(value: bool) -> String {
    if value { "on" } else { "off" }.to_string()
}

fn direct_param_display_value(snapshot: &EngineSnapshot, id: ParamId) -> Option<String> {
    let value = match id {
        ParamId::Osc1SawLevel => snapshot.direct.osc1_wave_mix[0],
        ParamId::Osc1PulseLevel => snapshot.direct.osc1_wave_mix[1],
        ParamId::Osc1TriangleLevel => snapshot.direct.osc1_wave_mix[2],
        ParamId::Osc1NoiseLevel => snapshot.direct.osc1_wave_mix[3],
        ParamId::Osc2SawLevel => snapshot.direct.osc2_wave_mix[0],
        ParamId::Osc2PulseLevel => snapshot.direct.osc2_wave_mix[1],
        ParamId::Osc2TriangleLevel => snapshot.direct.osc2_wave_mix[2],
        ParamId::SubLevel => snapshot.direct.sub_level,
        ParamId::MixerPreFilterDrive => snapshot.direct.mixer_pre_filter_drive,
        ParamId::MixerBodyMix => snapshot.direct.mixer_body_mix,
        ParamId::FilterCutoffHz => snapshot.direct.cutoff_hz,
        ParamId::FilterResonance => snapshot.direct.resonance,
        ParamId::FilterDrive => snapshot.direct.filter_drive,
        ParamId::FilterKeytrack => snapshot.direct.filter_tracking,
        ParamId::AmpEnvAttackMs => snapshot.direct.amp_env.attack_ms,
        ParamId::AmpEnvDecayMs => snapshot.direct.amp_env.decay_ms,
        ParamId::AmpEnvSustain => snapshot.direct.amp_env.sustain,
        ParamId::AmpEnvReleaseMs => snapshot.direct.amp_env.release_ms,
        ParamId::FilterEnvAttackMs => snapshot.direct.filter_env.attack_ms,
        ParamId::FilterEnvDecayMs => snapshot.direct.filter_env.decay_ms,
        ParamId::FilterEnvSustain => snapshot.direct.filter_env.sustain,
        ParamId::FilterEnvReleaseMs => snapshot.direct.filter_env.release_ms,
        ParamId::FilterEnvDepth => snapshot.direct.filter_env_depth,
        ParamId::VoiceStereoWidth => snapshot.direct.stereo_width,
        ParamId::VoiceDetuneSpreadCents => snapshot.direct.detune_spread_cents,
        ParamId::FinalStageBodyDrive => snapshot.direct.body_drive,
        ParamId::FinalStageAsymmetry => snapshot.direct.final_asymmetry,
        ParamId::FinalStageLowMidEmphasis => snapshot.direct.low_mid_emphasis,
        ParamId::FinalStageOutputTrimDb => snapshot.direct.output_trim_db,
        ParamId::ChorusMix => snapshot.direct.chorus_mix,
        ParamId::ChorusDepth => snapshot.direct.chorus_depth,
        ParamId::ChorusRateHz => snapshot.direct.chorus_rate_hz,
        ParamId::ReverbMix => snapshot.direct.reverb_mix,
        ParamId::ReverbSize => snapshot.direct.reverb_size,
        ParamId::ReverbDamping => snapshot.direct.reverb_damping,
        ParamId::Osc1FineTuneCents
        | ParamId::Osc2IntervalSemitones
        | ParamId::Osc2FineTuneCents
        | ParamId::Osc2SyncAmount
        | ParamId::Osc2CrossmodAmount
        | ParamId::SubOctaveOffset
        | ParamId::VoiceVelocityToLevel
        | ParamId::VoiceVelocityToFilter
        | ParamId::GravitacijaMacro
        | ParamId::BloomMacro
        | ParamId::HeatMacro
        | ParamId::RuinMacro
        | ParamId::SwarmMacro
        | ParamId::ChorusEnabled
        | ParamId::ReverbEnabled => return None,
    };
    Some(format_param_value(id, value))
}

fn format_param_value(id: ParamId, value: f32) -> String {
    match param_spec(id).unit {
        ParamUnit::Hertz => format!("{value:.1} Hz"),
        ParamUnit::Milliseconds => format!("{value:.1} ms"),
        ParamUnit::Decibels => format!("{value:.1} dB"),
        ParamUnit::Cents => format!("{value:.1} cents"),
        ParamUnit::Semitones => format!("{value:.1} st"),
        ParamUnit::Boolean => {
            if value >= 0.5 {
                "on".to_string()
            } else {
                "off".to_string()
            }
        }
        ParamUnit::Normalized => format!("{value:.2}"),
    }
}

fn event_matches_binding(event: &LastControlEvent, binding: &ControllerBinding) -> bool {
    event_is_recent(event)
        && matches!(
            event.kind,
            LastControlKind::ProfileCc(cc) | LastControlKind::LegacyCc(cc) if cc == binding.cc
        )
}

fn event_matches_legacy_cc(event: &LastControlEvent, cc: u8) -> bool {
    event_is_recent(event)
        && matches!(event.kind, LastControlKind::LegacyCc(event_cc) if event_cc == cc)
}

fn event_matches_program_change(event: &LastControlEvent, slot: u8) -> bool {
    event_is_recent(event)
        && matches!(event.kind, LastControlKind::ProgramChange(program) if program == slot)
}

fn event_is_recent(event: &LastControlEvent) -> bool {
    Instant::now() <= event.received_at + MIDI_ACTIVITY_FLASH
}

fn verdict_label(verdict: LastControlVerdict) -> &'static str {
    match verdict {
        LastControlVerdict::Accepted => "accepted",
        LastControlVerdict::Filtered => "filtered",
        LastControlVerdict::Reserved => "reserved",
        LastControlVerdict::ReleaseIgnored => "release ignored",
        LastControlVerdict::Ignored => "ignored",
    }
}

fn run_performance_window(session: RuntimeSession) -> Result<()> {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("EPM1 Performance Rig")
            .with_inner_size([1140.0, 720.0])
            .with_min_inner_size([960.0, 640.0]),
        ..Default::default()
    };

    eframe::run_native(
        "EPM1 Performance Rig",
        options,
        Box::new(move |_cc| Ok(Box::new(PerformanceApp::new(session)))),
    )
    .map_err(|error| anyhow!("failed to launch performance window: {error}"))
}

fn build_audio_runtime(
    patch_path: &Path,
    audio_selector: Option<&str>,
    alsa_tuning: AlsaPlaybackTuning,
) -> Result<PreparedAudioRuntime> {
    let patch = load_patch_from_path(patch_path)?;
    validate_patch_v1(&patch).context("patch validation failed")?;
    let selected_device = select_alsa_output_device(audio_selector)?;
    let sample_rate_hz = ALSA_PLAYBACK_SAMPLE_RATE_HZ as f32;
    let channels = ALSA_PLAYBACK_CHANNELS;
    let bend_range = patch.performance_response.bend_range_semitones as f32;
    let patch_name = patch.meta.patch_name.clone();

    let (tx, rx) = mpsc::channel::<EngineCommand>();
    let transport_metrics = Arc::new(TransportMetrics::default());
    transport_metrics.record_write_request(alsa_tuning.period_frames);
    let midi_input_queue = Arc::new(ArrayQueue::new(MIDI_INPUT_QUEUE_CAPACITY));
    let priority_actions = Arc::new(PriorityActions::default());
    let engine = Engine::new(
        EngineConfig {
            sample_rate_hz,
            max_block_frames: 2_048,
            voice_count: 6,
        },
        patch,
    )?;
    let (producer, consumer) = RingBuffer::<StereoFrame>::new(AUDIO_QUEUE_CAPACITY_FRAMES);
    let worker = EngineWorker::new(spawn_engine_thread(
        engine,
        rx,
        producer,
        Arc::clone(&midi_input_queue),
        Arc::clone(&priority_actions),
        Arc::clone(&transport_metrics),
    ));
    Ok(PreparedAudioRuntime {
        tx,
        worker,
        consumer,
        selected_device: selected_device.clone(),
        audio_selector: selected_device.selector.clone(),
        audio_device_name: selected_device.display_name(),
        sample_rate_hz: ALSA_PLAYBACK_SAMPLE_RATE_HZ,
        channels,
        alsa_tuning,
        bend_range,
        patch_name,
        midi_input_queue,
        priority_actions,
        transport_metrics,
    })
}

fn start_prepared_audio_runtime(prepared: PreparedAudioRuntime) -> Result<AudioRuntime> {
    let PreparedAudioRuntime {
        tx,
        worker,
        consumer,
        selected_device,
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
    } = prepared;

    let opened_playback = match open_alsa_playback_device(&selected_device, alsa_tuning) {
        Ok(opened_playback) => opened_playback,
        Err(error) => {
            worker.shutdown(tx);
            return Err(error);
        }
    };
    transport_metrics.record_write_request(opened_playback.tuning.period_frames);
    let stream =
        AlsaPlaybackStream::spawn(opened_playback, consumer, Arc::clone(&transport_metrics));

    Ok(AudioRuntime {
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
    })
}

fn print_runtime_help() {
    println!("runtime commands:");
    println!("  help                     show this command list");
    println!("  status                   show current patch, mode, macros, and activity");
    println!("  patches                  list factory patches");
    println!("  favorites                list the live-ready setlist slots");
    println!("  favorite <0..7>          load a setlist slot directly");
    println!("  patch <name-or-path>     load a factory patch or explicit TOML path");
    println!("  next                     load the next favorite patch");
    println!("  prev                     load the previous favorite patch");
    println!("  demo-patch               jump to the default opener patch");
    println!("  macro <name> <0..1>      set one public macro");
    println!("  panic                    clear all notes and live controller state");
    println!("  reset-controllers        clear bend/aftertouch/mod/sustain and macros");
    println!("  audio                    list ALSA hw playback outputs");
    println!(
        "  audio <index-or-hw:card,device> switch ALSA hw output (resets live performance state)"
    );
    println!("  --audio-device <selector> required ALSA selector at launch");
    println!("  --alsa-period-frames <n> ALSA period size in frames");
    println!("  --alsa-buffer-frames <n> ALSA buffer size in frames");
    println!("  --alsa-start-threshold-frames <n> ALSA start threshold in frames");
    println!("  midi                     list available MIDI inputs");
    println!("  midi <name-or-index>     switch to a MIDI input");
    println!("  --midi-channel <1..16>   accept MIDI only from one channel at launch");
    println!("  --controller-profile <path> load TOML controller bindings");
    println!("  --trace-midi             log incoming MIDI messages to stderr");
    println!("  demo                     switch the session to the demo performer");
    println!("  quit                     stop playback and exit");
}

fn parse_play_options(args: &[String]) -> Result<PlayOptions> {
    let mut patch_arg: Option<String> = None;
    let mut force_demo = false;
    let mut audio_selector = None;
    let mut alsa_period_frames = None;
    let mut alsa_buffer_frames = None;
    let mut alsa_start_threshold_frames = None;
    let mut midi_selector = None;
    let mut midi_channel = None;
    let mut controller_profile_path = None;
    let mut trace_midi = false;
    let mut headless = false;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--demo" => {
                force_demo = true;
                index += 1;
            }
            "--headless" => {
                headless = true;
                index += 1;
            }
            "--audio-device" => {
                let value = args
                    .get(index + 1)
                    .context("missing value after --audio-device")?;
                audio_selector = Some(value.clone());
                index += 2;
            }
            "--alsa-period-frames" => {
                let value = args
                    .get(index + 1)
                    .context("missing value after --alsa-period-frames")?;
                alsa_period_frames = Some(parse_frame_count(value, "--alsa-period-frames")?);
                index += 2;
            }
            "--alsa-buffer-frames" => {
                let value = args
                    .get(index + 1)
                    .context("missing value after --alsa-buffer-frames")?;
                alsa_buffer_frames = Some(parse_frame_count(value, "--alsa-buffer-frames")?);
                index += 2;
            }
            "--alsa-start-threshold-frames" => {
                let value = args
                    .get(index + 1)
                    .context("missing value after --alsa-start-threshold-frames")?;
                alsa_start_threshold_frames =
                    Some(parse_frame_count(value, "--alsa-start-threshold-frames")?);
                index += 2;
            }
            "--midi-device" => {
                let value = args
                    .get(index + 1)
                    .context("missing value after --midi-device")?;
                midi_selector = Some(value.clone());
                index += 2;
            }
            "--midi-channel" => {
                let value = args
                    .get(index + 1)
                    .context("missing value after --midi-channel")?;
                midi_channel = Some(parse_midi_channel(value)?);
                index += 2;
            }
            "--controller-profile" => {
                let value = args
                    .get(index + 1)
                    .context("missing value after --controller-profile")?;
                controller_profile_path = Some(resolve_controller_profile_argument(value)?);
                index += 2;
            }
            "--trace-midi" => {
                trace_midi = true;
                index += 1;
            }
            option if option.starts_with("--") => {
                return Err(anyhow!("unknown play option `{option}`"));
            }
            patch if patch_arg.is_none() => {
                patch_arg = Some(patch.to_string());
                index += 1;
            }
            extra => {
                return Err(anyhow!(
                    "unexpected extra argument `{extra}`; pass at most one patch name or path"
                ));
            }
        }
    }

    Ok(PlayOptions {
        patch_path: resolve_patch_argument(patch_arg.as_deref())?,
        force_demo,
        audio_selector,
        alsa_period_frames,
        alsa_buffer_frames,
        alsa_start_threshold_frames,
        midi_selector,
        midi_channel,
        controller_profile_path,
        trace_midi,
        headless,
    })
}

fn parse_frame_count(value: &str, flag: &str) -> Result<usize> {
    let frames = value
        .parse::<usize>()
        .with_context(|| format!("invalid frame count `{value}` for {flag}"))?;
    if frames == 0 {
        Err(anyhow!("{flag} expects a value greater than zero"))
    } else {
        Ok(frames)
    }
}

fn parse_midi_channel(value: &str) -> Result<u8> {
    let channel = value
        .parse::<u8>()
        .with_context(|| format!("invalid MIDI channel `{value}`"))?;
    if (1..=16).contains(&channel) {
        Ok(channel)
    } else {
        Err(anyhow!(
            "invalid MIDI channel `{value}`; expected a value in the range 1..16"
        ))
    }
}

fn parse_runtime_ui_command(input: &str) -> Result<RuntimeUiCommand> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(RuntimeUiCommand::Noop);
    }

    let mut parts = trimmed.split_whitespace();
    let command = parts.next().unwrap_or_default();

    match command {
        "help" | "h" => Ok(RuntimeUiCommand::Help),
        "status" | "s" => Ok(RuntimeUiCommand::Status),
        "patches" | "list-patches" => Ok(RuntimeUiCommand::Patches),
        "favorites" | "favs" => Ok(RuntimeUiCommand::Favorites),
        "favorite" | "fav" => {
            let slot = parts
                .next()
                .context("favorite command requires a numeric slot")?;
            if parts.next().is_some() {
                return Err(anyhow!("favorite command accepts exactly one slot"));
            }
            let slot = slot
                .parse::<usize>()
                .with_context(|| format!("invalid favorite slot `{slot}`"))?;
            Ok(RuntimeUiCommand::Favorite(slot))
        }
        "patch" => {
            let argument = parts.collect::<Vec<_>>().join(" ");
            if argument.trim().is_empty() {
                Err(anyhow!("patch command requires a factory name or path"))
            } else {
                Ok(RuntimeUiCommand::Patch(argument))
            }
        }
        "next" | "next-favorite" => Ok(RuntimeUiCommand::NextFavorite),
        "prev" | "previous" | "prev-favorite" => Ok(RuntimeUiCommand::PrevFavorite),
        "demo-patch" => Ok(RuntimeUiCommand::DemoPatch),
        "macro" => {
            let macro_name = parts
                .next()
                .context("macro command requires a macro name")?;
            let value = parts.next().context("macro command requires a value")?;
            if parts.next().is_some() {
                return Err(anyhow!("macro command accepts exactly two arguments"));
            }
            let macro_id = parse_macro_id(macro_name)?;
            let value = value
                .parse::<f32>()
                .with_context(|| format!("invalid macro value `{value}`"))?;
            Ok(RuntimeUiCommand::Macro(macro_id, value.clamp(0.0, 1.0)))
        }
        "panic" => Ok(RuntimeUiCommand::Panic),
        "reset-controllers" | "reset" => Ok(RuntimeUiCommand::ResetControllers),
        "audio" => {
            let selector = parts.collect::<Vec<_>>().join(" ");
            if selector.trim().is_empty() {
                Ok(RuntimeUiCommand::AudioList)
            } else {
                Ok(RuntimeUiCommand::AudioSelect(selector))
            }
        }
        "midi" => {
            let selector = parts.collect::<Vec<_>>().join(" ");
            if selector.trim().is_empty() {
                Ok(RuntimeUiCommand::MidiList)
            } else {
                Ok(RuntimeUiCommand::MidiSelect(selector))
            }
        }
        "demo" => Ok(RuntimeUiCommand::Demo),
        "quit" | "exit" => Ok(RuntimeUiCommand::Quit),
        other => Err(anyhow!("unknown runtime command `{other}`")),
    }
}

fn parse_macro_id(value: &str) -> Result<MacroId> {
    match value.trim().to_ascii_lowercase().as_str() {
        "gravitacija" => Ok(MacroId::Gravitacija),
        "bloom" => Ok(MacroId::Bloom),
        "heat" => Ok(MacroId::Heat),
        "ruin" => Ok(MacroId::Ruin),
        "swarm" => Ok(MacroId::Swarm),
        _ => Err(anyhow!(
            "unknown macro `{value}`; expected gravitacija, bloom, heat, ruin, or swarm"
        )),
    }
}

fn macro_display_name(id: MacroId) -> &'static str {
    match id {
        MacroId::Gravitacija => "Gravitacija",
        MacroId::Bloom => "Bloom",
        MacroId::Heat => "Heat",
        MacroId::Ruin => "Ruin",
        MacroId::Swarm => "Swarm",
    }
}

fn load_controller_profile(path: &Path) -> Result<ControllerProfile> {
    let input = fs::read_to_string(path)
        .with_context(|| format!("failed to read controller profile {}", path.display()))?;
    controller_profile_from_toml(&input, path)
}

fn controller_profile_from_toml(input: &str, path: &Path) -> Result<ControllerProfile> {
    let file: ControllerProfileFile = toml::from_str(input)
        .with_context(|| format!("failed to parse controller profile {}", path.display()))?;
    if file.binding.is_empty() {
        return Err(anyhow!(
            "controller profile {} has no bindings",
            path.display()
        ));
    }

    let mut bindings_by_cc = HashMap::new();
    for binding in file.binding {
        let cc = binding.cc;
        let action = controller_binding_action(&binding)
            .with_context(|| format!("invalid binding `{}` on cc {cc}", binding.control))?;
        let (section, index) = controller_binding_section(&binding)?;
        let previous = bindings_by_cc.insert(
            cc,
            ControllerBinding {
                cc,
                control: binding.control,
                section,
                index,
                action,
            },
        );
        if let Some(previous) = previous {
            return Err(anyhow!(
                "controller profile {} maps cc {cc} more than once (`{}` and another binding)",
                path.display(),
                previous.control
            ));
        }
    }

    Ok(ControllerProfile {
        name: file.name.unwrap_or_else(|| {
            path.file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("controller-profile")
                .to_string()
        }),
        path: path.to_path_buf(),
        bindings_by_cc,
    })
}

fn controller_binding_section(
    binding: &ControllerBindingFile,
) -> Result<(ControllerBindingSection, Option<u8>)> {
    let inferred = infer_controller_binding_section(&binding.control);
    let section = binding.section.unwrap_or(inferred.0);
    let index = binding.index.or(inferred.1);
    if let Some(index) = index {
        if index == 0 {
            return Err(anyhow!(
                "controller binding index must be greater than zero"
            ));
        }
        if matches!(
            section,
            ControllerBindingSection::Knob
                | ControllerBindingSection::Slider
                | ControllerBindingSection::Switch
        ) && index > 9
        {
            return Err(anyhow!(
                "controller binding index {index} is out of range for section {section:?}"
            ));
        }
    }
    Ok((section, index))
}

fn infer_controller_binding_section(control: &str) -> (ControllerBindingSection, Option<u8>) {
    let trimmed = control.trim();
    if let Some(rest) = trimmed.strip_prefix("SW") {
        return (
            ControllerBindingSection::Switch,
            parse_control_index_prefix(rest),
        );
    }
    if let Some(rest) = trimmed.strip_prefix('K') {
        return (
            ControllerBindingSection::Knob,
            parse_control_index_prefix(rest),
        );
    }
    if let Some(rest) = trimmed.strip_prefix('S') {
        return (
            ControllerBindingSection::Slider,
            parse_control_index_prefix(rest),
        );
    }
    (ControllerBindingSection::Other, None)
}

fn parse_control_index_prefix(value: &str) -> Option<u8> {
    let digits = value
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    digits.parse::<u8>().ok()
}

fn controller_binding_action(binding: &ControllerBindingFile) -> Result<ControllerBindingAction> {
    match binding.kind {
        ControllerBindingKind::Macro => {
            let target = binding
                .target
                .as_deref()
                .ok_or_else(|| anyhow!("macro binding requires target"))?;
            Ok(ControllerBindingAction::Macro(parse_macro_id(target)?))
        }
        ControllerBindingKind::DirectParam => {
            let target = binding
                .target
                .as_deref()
                .ok_or_else(|| anyhow!("direct_param binding requires target"))?;
            let id = param_by_key(target)
                .ok_or_else(|| anyhow!("unknown direct_param target `{target}`"))?;
            let spec = param_spec(id);
            if spec.unit == ParamUnit::Boolean {
                return Err(anyhow!(
                    "direct_param target `{target}` is boolean; use toggle_param"
                ));
            }
            Ok(ControllerBindingAction::DirectParam {
                id,
                scale: binding.scale.unwrap_or_else(|| default_scale_for_param(id)),
            })
        }
        ControllerBindingKind::RuntimeAction => {
            let action = binding
                .action
                .as_deref()
                .ok_or_else(|| anyhow!("runtime_action binding requires action"))?;
            Ok(ControllerBindingAction::Runtime(
                parse_profile_runtime_action(action, binding.slot)?,
            ))
        }
        ControllerBindingKind::ToggleParam => {
            let target = binding
                .target
                .as_deref()
                .ok_or_else(|| anyhow!("toggle_param binding requires target"))?;
            let id =
                param_by_key(target).ok_or_else(|| anyhow!("unknown toggle target `{target}`"))?;
            let spec = param_spec(id);
            if spec.unit != ParamUnit::Boolean {
                return Err(anyhow!(
                    "toggle_param target `{target}` is not a boolean parameter"
                ));
            }
            Ok(ControllerBindingAction::ToggleParam(id))
        }
        ControllerBindingKind::Reserved => Ok(ControllerBindingAction::Reserved),
    }
}

fn parse_profile_runtime_action(
    action: &str,
    slot: Option<usize>,
) -> Result<RuntimeControlMessage> {
    match action.trim().to_ascii_lowercase().as_str() {
        "panic" => Ok(RuntimeControlMessage::Panic),
        "reset_controllers" | "reset-controllers" | "reset" => {
            Ok(RuntimeControlMessage::ResetControllers)
        }
        "next_favorite" | "next" => Ok(RuntimeControlMessage::NextFavorite),
        "prev_favorite" | "previous_favorite" | "prev" | "previous" => {
            Ok(RuntimeControlMessage::PrevFavorite)
        }
        "favorite_slot" | "favorite" => {
            let slot = slot.ok_or_else(|| anyhow!("favorite_slot action requires slot"))?;
            if slot < LIVE_SET_STEMS.len() {
                Ok(RuntimeControlMessage::FavoriteSlot(slot))
            } else {
                Err(anyhow!("favorite slot {slot} is out of range"))
            }
        }
        _ => Err(anyhow!("unknown runtime action `{action}`")),
    }
}

fn default_scale_for_param(id: ParamId) -> ControllerValueScale {
    match param_spec(id).unit {
        ParamUnit::Hertz => ControllerValueScale::Log,
        _ => ControllerValueScale::Linear,
    }
}

fn scale_controller_value(id: ParamId, value: f32, scale: ControllerValueScale) -> f32 {
    let spec = param_spec(id);
    let normalized = value.clamp(0.0, 1.0);
    match scale {
        ControllerValueScale::Linear => spec.min + normalized * (spec.max - spec.min),
        ControllerValueScale::Log if spec.min > 0.0 && spec.max > spec.min => {
            let min = spec.min.ln();
            let max = spec.max.ln();
            (min + normalized * (max - min)).exp()
        }
        ControllerValueScale::Log => spec.min + normalized * (spec.max - spec.min),
    }
}

fn load_patch_from_path(path: &Path) -> Result<PatchFileV1> {
    let input = fs::read_to_string(path)
        .with_context(|| format!("failed to read patch file {}", path.display()))?;
    load_patch_toml(&input)
        .with_context(|| format!("failed to parse patch file {}", path.display()))
}

fn resolve_patch_argument(argument: Option<&str>) -> Result<PathBuf> {
    let Some(argument) = argument else {
        return Ok(default_patch_path());
    };

    let direct_path = PathBuf::from(argument);
    if direct_path.exists() {
        return Ok(direct_path);
    }

    let factory_dir = workspace_root().join("patches/factory");
    let factory_candidate = if argument.ends_with(".toml") {
        factory_dir.join(argument)
    } else {
        factory_dir.join(format!("{argument}.toml"))
    };
    if factory_candidate.exists() {
        return Ok(factory_candidate);
    }

    let argument_slug = slugify(argument);
    for entry in factory_patch_entries()? {
        if entry.stem.eq_ignore_ascii_case(argument) || entry.slug == argument_slug {
            return Ok(entry.path);
        }
    }

    Err(anyhow!(
        "unknown patch `{argument}`; use a path or run `list-factory` for available factory names"
    ))
}

fn resolve_controller_profile_argument(argument: &str) -> Result<PathBuf> {
    let direct_path = PathBuf::from(argument);
    if direct_path.exists() {
        return Ok(direct_path);
    }

    let workspace_path = workspace_root().join(argument);
    if workspace_path.exists() {
        return Ok(workspace_path);
    }

    Err(anyhow!(
        "unknown controller profile `{argument}`; use a path relative to the current directory or repository root"
    ))
}

fn factory_patch_entries() -> Result<Vec<FactoryPatchEntry>> {
    let factory_dir = workspace_root().join("patches/factory");
    let mut entries = Vec::new();

    for entry in fs::read_dir(&factory_dir).with_context(|| {
        format!(
            "failed to read factory patch directory {}",
            factory_dir.display()
        )
    })? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }

        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("toml") {
            continue;
        }

        let patch = load_patch_from_path(&path)?;
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or_else(|| anyhow!("invalid factory patch filename {}", path.display()))?
            .to_string();
        let favorite = patch
            .ui
            .as_ref()
            .and_then(|ui| ui.favorite)
            .unwrap_or(false);
        entries.push(FactoryPatchEntry {
            stem: stem.clone(),
            slug: slugify(&patch.meta.patch_name),
            path,
            patch_name: patch.meta.patch_name,
            description: patch.meta.description,
            favorite,
        });
    }

    entries.sort_by(|left, right| left.stem.cmp(&right.stem));
    Ok(entries)
}

fn favorite_patch_entries() -> Result<Vec<FactoryPatchEntry>> {
    live_patch_entries()
}

fn live_patch_entries() -> Result<Vec<FactoryPatchEntry>> {
    let entries = factory_patch_entries()?;
    let mut ordered = Vec::with_capacity(LIVE_SET_STEMS.len());

    for stem in LIVE_SET_STEMS {
        let Some(entry) = entries.iter().find(|entry| entry.stem == stem) else {
            return Err(anyhow!(
                "live set references missing factory patch `{stem}`"
            ));
        };
        ordered.push(entry.clone());
    }

    Ok(ordered)
}

fn live_patch_path(slot: usize) -> Result<PathBuf> {
    live_patch_entries()?
        .get(slot)
        .map(|entry| entry.path.clone())
        .ok_or_else(|| anyhow!("live slot {slot} is out of range"))
}

fn live_slot_for_path(path: &Path) -> Option<usize> {
    let current_stem = path.file_stem().and_then(|value| value.to_str())?;
    LIVE_SET_STEMS.iter().position(|stem| *stem == current_stem)
}

fn adjacent_live_patch(current_path: &Path, direction: isize) -> Result<PathBuf> {
    let live_set = live_patch_entries()?;
    let current_stem = current_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let current_index = live_set.iter().position(|entry| entry.stem == current_stem);

    let target_index = match current_index {
        Some(index) => wrap_index(index, direction, live_set.len()),
        None if direction >= 0 => 0,
        None => live_set.len().saturating_sub(1),
    };

    live_set
        .get(target_index)
        .map(|entry| entry.path.clone())
        .ok_or_else(|| anyhow!("live patch selection failed"))
}

fn wrap_index(index: usize, direction: isize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let len = len as isize;
    let index = index as isize;
    ((index + direction).rem_euclid(len)) as usize
}

fn collect_midi_ports(midi_input: &MidiInput) -> Result<Vec<NamedMidiPort>> {
    let mut ports = Vec::new();
    for port in midi_input.ports() {
        let name = midi_input
            .port_name(&port)
            .unwrap_or_else(|_| "unknown-midi-port".to_string());
        ports.push(NamedMidiPort { port, name });
    }
    Ok(ports)
}

fn open_midi_input(
    midi_input_queue: Arc<ArrayQueue<RealtimeMidiMessage>>,
    bend_range: f32,
    selector: Option<&str>,
    midi_channel: Option<u8>,
    controller_profile: Option<Arc<ControllerProfile>>,
    trace_midi: bool,
    runtime_control_queue: Arc<ArrayQueue<RuntimeControlMessage>>,
    input_metrics: Arc<InputMetrics>,
) -> Result<Option<OpenedMidiConnection>> {
    let mut midi_input = match MidiInput::new("mamut-standalone") {
        Ok(midi_input) => midi_input,
        Err(error) => {
            return if selector.is_some() {
                Err(anyhow!("failed to create MIDI input: {error}"))
            } else {
                Ok(None)
            };
        }
    };
    midi_input.ignore(Ignore::None);
    let ports = collect_midi_ports(&midi_input)?;

    if ports.is_empty() {
        return if selector.is_some() {
            Err(anyhow!("no MIDI input devices available"))
        } else {
            Ok(None)
        };
    }

    let port_index = if let Some(selector) = selector {
        let names: Vec<String> = ports.iter().map(|entry| entry.name.clone()).collect();
        select_named_index(selector, &names, "MIDI input device")?
    } else {
        0
    };

    let selected = ports
        .into_iter()
        .nth(port_index)
        .ok_or_else(|| anyhow!("MIDI input device index {port_index} is out of range"))?;
    let port_name = selected.name.clone();
    let connection = midi_input
        .connect(
            &selected.port,
            "mamut-midi-in",
            move |_stamp, message, _| {
                let parsed = parse_midi_message(
                    message,
                    bend_range,
                    midi_channel,
                    controller_profile.as_deref(),
                );
                if let Some(event) = last_control_event(
                    message,
                    midi_channel,
                    controller_profile.as_deref(),
                    parsed,
                    Instant::now(),
                ) {
                    input_metrics.record_last_control(event);
                }
                if trace_midi {
                    trace_midi_message(
                        message,
                        midi_channel,
                        controller_profile.as_deref(),
                        parsed,
                    );
                }
                if let Some(parsed) = parsed {
                    input_metrics.record_midi_message();
                    match parsed {
                        ParsedMidiMessage::Realtime(message) => {
                            let _ = midi_input_queue.push(message);
                        }
                        ParsedMidiMessage::Runtime(command) => {
                            let _ = runtime_control_queue.push(command);
                        }
                        ParsedMidiMessage::Reserved => {}
                    }
                }
            },
            (),
        )
        .map_err(|error| anyhow!("failed to open MIDI input connection: {error}"))?;

    Ok(Some(OpenedMidiConnection {
        port_name,
        _connection: connection,
    }))
}

fn trace_midi_message(
    message: &[u8],
    midi_channel: Option<u8>,
    controller_profile: Option<&ControllerProfile>,
    parsed: Option<ParsedMidiMessage>,
) {
    let raw_status = *message.first().unwrap_or(&0);
    let status = raw_status & 0xF0;
    let channel = (raw_status & 0x0F) + 1;
    let raw = message
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ");

    let channel_filtered =
        status < 0xF0 && midi_channel.is_some_and(|expected| channel != expected);
    let verdict = if channel_filtered {
        format!(
            "filtered channel {channel} (expected channel {})",
            midi_channel.unwrap_or(channel)
        )
    } else if let Some(binding) = profile_binding_for_message(message, controller_profile) {
        describe_profile_cc_message(binding, message[2] as f32 / 127.0, parsed)
    } else if let Some(parsed) = parsed {
        describe_parsed_midi_message(parsed)
    } else {
        describe_unparsed_midi_message(message)
    };

    if status < 0xF0 {
        eprintln!("midi trace: ch={channel} raw=[{raw}] {verdict}");
    } else {
        eprintln!("midi trace: system raw=[{raw}] {verdict}");
    }
}

fn last_control_event(
    message: &[u8],
    midi_channel: Option<u8>,
    controller_profile: Option<&ControllerProfile>,
    parsed: Option<ParsedMidiMessage>,
    received_at: Instant,
) -> Option<LastControlEvent> {
    let raw_status = *message.first()?;
    let status = raw_status & 0xF0;
    let channel = (raw_status & 0x0F) + 1;
    if status < 0xF0 && midi_channel.is_some_and(|expected| channel != expected) {
        return Some(LastControlEvent {
            kind: LastControlKind::Other,
            label: format!("channel {channel}"),
            action: "filtered".to_string(),
            raw_status,
            channel: Some(channel),
            raw_value: message.get(2).map(|value| *value as f32 / 127.0),
            program: None,
            verdict: LastControlVerdict::Filtered,
            received_at,
        });
    }

    match status {
        0xB0 if message.len() >= 3 => {
            let cc = message[1];
            let raw_value = message[2] as f32 / 127.0;
            if let Some(binding) = controller_profile.and_then(|profile| profile.binding_for_cc(cc))
            {
                let verdict = match (binding.action, parsed) {
                    (ControllerBindingAction::Reserved, _)
                    | (_, Some(ParsedMidiMessage::Reserved)) => LastControlVerdict::Reserved,
                    (ControllerBindingAction::Runtime(_), None)
                    | (ControllerBindingAction::ToggleParam(_), None) => {
                        LastControlVerdict::ReleaseIgnored
                    }
                    (_, Some(_)) => LastControlVerdict::Accepted,
                    (_, None) => LastControlVerdict::Ignored,
                };
                return Some(LastControlEvent {
                    kind: LastControlKind::ProfileCc(cc),
                    label: binding.control.clone(),
                    action: describe_binding_action(binding.action),
                    raw_status,
                    channel: Some(channel),
                    raw_value: Some(raw_value),
                    program: None,
                    verdict,
                    received_at,
                });
            }

            let (label, action) = match cc {
                1 => ("Mod Wheel".to_string(), "mod wheel".to_string()),
                64 => ("Sustain".to_string(), "sustain".to_string()),
                16 => ("Legacy CC16".to_string(), "macro Gravitacija".to_string()),
                17 => ("Legacy CC17".to_string(), "macro Bloom".to_string()),
                18 => ("Legacy CC18".to_string(), "macro Heat".to_string()),
                19 => ("Legacy CC19".to_string(), "macro Ruin".to_string()),
                20 => ("Legacy CC20".to_string(), "macro Swarm".to_string()),
                _ => return None,
            };
            Some(LastControlEvent {
                kind: LastControlKind::LegacyCc(cc),
                label,
                action,
                raw_status,
                channel: Some(channel),
                raw_value: Some(raw_value),
                program: None,
                verdict: if parsed.is_some() {
                    LastControlVerdict::Accepted
                } else {
                    LastControlVerdict::Ignored
                },
                received_at,
            })
        }
        0xC0 if message.len() >= 2 => Some(LastControlEvent {
            kind: LastControlKind::ProgramChange(message[1]),
            label: "Program Change".to_string(),
            action: format!("live slot {}", message[1]),
            raw_status,
            channel: Some(channel),
            raw_value: None,
            program: Some(message[1]),
            verdict: if parsed.is_some() {
                LastControlVerdict::Accepted
            } else {
                LastControlVerdict::Ignored
            },
            received_at,
        }),
        0xD0 if message.len() >= 2 => Some(LastControlEvent {
            kind: LastControlKind::Other,
            label: "Channel Aftertouch".to_string(),
            action: "aftertouch".to_string(),
            raw_status,
            channel: Some(channel),
            raw_value: Some(message[1] as f32 / 127.0),
            program: None,
            verdict: LastControlVerdict::Accepted,
            received_at,
        }),
        0xE0 if message.len() >= 3 => Some(LastControlEvent {
            kind: LastControlKind::Other,
            label: "Pitch Bend".to_string(),
            action: "pitch bend".to_string(),
            raw_status,
            channel: Some(channel),
            raw_value: None,
            program: None,
            verdict: LastControlVerdict::Accepted,
            received_at,
        }),
        _ => None,
    }
}

fn profile_binding_for_message<'a>(
    message: &[u8],
    controller_profile: Option<&'a ControllerProfile>,
) -> Option<&'a ControllerBinding> {
    let raw_status = *message.first()?;
    let status = raw_status & 0xF0;
    if status == 0xB0 && message.len() >= 3 {
        controller_profile.and_then(|profile| profile.binding_for_cc(message[1]))
    } else {
        None
    }
}

fn describe_profile_cc_message(
    binding: &ControllerBinding,
    value: f32,
    parsed: Option<ParsedMidiMessage>,
) -> String {
    match (binding.action, parsed) {
        (ControllerBindingAction::Reserved, _) | (_, Some(ParsedMidiMessage::Reserved)) => {
            format!("profile {} reserved value={value:.3}", binding.control)
        }
        (ControllerBindingAction::Runtime(_), None)
        | (ControllerBindingAction::ToggleParam(_), None) => {
            format!(
                "profile {} {} release ignored value={value:.3}",
                binding.control,
                describe_binding_action(binding.action)
            )
        }
        (_, Some(parsed)) => format!(
            "profile {} {} value={value:.3} -> {}",
            binding.control,
            describe_binding_action(binding.action),
            describe_parsed_midi_message(parsed)
        ),
        (_, None) => format!(
            "profile {} {} value={value:.3}",
            binding.control,
            describe_binding_action(binding.action)
        ),
    }
}

fn describe_binding_action(action: ControllerBindingAction) -> String {
    match action {
        ControllerBindingAction::Macro(id) => format!("macro {}", macro_display_name(id)),
        ControllerBindingAction::DirectParam { id, .. } => {
            format!("direct param {}", param_spec(id).name)
        }
        ControllerBindingAction::Runtime(message) => describe_runtime_control_message(message),
        ControllerBindingAction::ToggleParam(id) => format!("toggle {}", param_spec(id).name),
        ControllerBindingAction::Reserved => "reserved".to_string(),
    }
}

fn describe_runtime_control_message(message: RuntimeControlMessage) -> String {
    match message {
        RuntimeControlMessage::ProgramChange(slot) => format!("program change slot={slot}"),
        RuntimeControlMessage::Panic => "panic".to_string(),
        RuntimeControlMessage::ResetControllers => "reset controllers".to_string(),
        RuntimeControlMessage::NextFavorite => "next favorite".to_string(),
        RuntimeControlMessage::PrevFavorite => "previous favorite".to_string(),
        RuntimeControlMessage::FavoriteSlot(slot) => format!("favorite slot={slot}"),
        RuntimeControlMessage::ToggleParam(id) => format!("toggle {}", param_spec(id).name),
    }
}

fn describe_parsed_midi_message(parsed: ParsedMidiMessage) -> String {
    match parsed {
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Note(NoteEvent::NoteOn {
            note,
            velocity,
        })) => format!("note on note={note} velocity={velocity:.3}"),
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Note(NoteEvent::NoteOff { note })) => {
            format!("note off note={note}")
        }
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
            ControllerEvent::ModWheel { amount },
        )) => format!("mod wheel amount={amount:.3}"),
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
            ControllerEvent::Sustain { down },
        )) => format!("sustain down={down}"),
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
            ControllerEvent::ChannelAftertouch { pressure },
        )) => format!("aftertouch pressure={pressure:.3}"),
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
            ControllerEvent::PitchBend { semitones },
        )) => format!("pitch bend semitones={semitones:.3}"),
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(ControllerEvent::Macro {
            id,
            value,
        })) => format!("macro {} value={value:.3}", macro_display_name(id)),
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
            ControllerEvent::DirectParam { id, value },
        )) => format!("direct param {} value={value:.3}", param_spec(id).name),
        ParsedMidiMessage::Runtime(message) => describe_runtime_control_message(message),
        ParsedMidiMessage::Reserved => "profile reserved".to_string(),
    }
}

fn describe_unparsed_midi_message(message: &[u8]) -> String {
    let Some(raw_status) = message.first().copied() else {
        return "ignored empty message".to_string();
    };
    let status = raw_status & 0xF0;
    match status {
        0xA0 if message.len() >= 3 => {
            format!(
                "ignored poly aftertouch note={} value={}",
                message[1], message[2]
            )
        }
        0xB0 if message.len() >= 3 => format!("ignored cc={} value={}", message[1], message[2]),
        0xF0 => "ignored system/common message".to_string(),
        _ => "ignored unsupported message".to_string(),
    }
}

fn select_named_index(selector: &str, names: &[String], kind: &str) -> Result<usize> {
    if let Ok(index) = selector.parse::<usize>() {
        return if index < names.len() {
            Ok(index)
        } else {
            Err(anyhow!(
                "{kind} index {index} is out of range; available count is {}",
                names.len()
            ))
        };
    }

    if let Some((index, _)) = names
        .iter()
        .enumerate()
        .find(|(_, name)| name.eq_ignore_ascii_case(selector))
    {
        return Ok(index);
    }

    let selector_lower = selector.to_ascii_lowercase();
    let matches: Vec<usize> = names
        .iter()
        .enumerate()
        .filter_map(|(index, name)| {
            name.to_ascii_lowercase()
                .contains(&selector_lower)
                .then_some(index)
        })
        .collect();

    match matches.as_slice() {
        [index] => Ok(*index),
        [] => Err(anyhow!(
            "no {kind} matched `{selector}`; run the relevant list command to inspect available devices"
        )),
        _ => Err(anyhow!(
            "selector `{selector}` is ambiguous for {kind}; use a numeric index or a more specific name"
        )),
    }
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut last_was_hyphen = false;

    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_was_hyphen = false;
        } else if !last_was_hyphen && !slug.is_empty() {
            slug.push('-');
            last_was_hyphen = true;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    slug
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

fn default_patch_path() -> PathBuf {
    workspace_root().join("patches/factory/molten-horizon.toml")
}

fn round_up_to_render_block(frames: usize) -> usize {
    if frames == 0 {
        return 0;
    }

    let remainder = frames % ENGINE_RENDER_BLOCK_FRAMES;
    if remainder == 0 {
        frames
    } else {
        frames + (ENGINE_RENDER_BLOCK_FRAMES - remainder)
    }
}

fn run_alsa_playback_loop(
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
        transport_metrics.record_write_request(frames_to_write);

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

enum AlsaPlaybackWriter<'a> {
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

fn convert_f32_samples_to_s32(samples: &[f32], output: &mut [i32]) {
    debug_assert!(output.len() >= samples.len());
    for (sample, target) in samples.iter().zip(output.iter_mut()) {
        *target = f32_sample_to_s32(*sample);
    }
}

fn f32_sample_to_s32(sample: f32) -> i32 {
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

fn recover_alsa_playback_error(
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

struct EngineThreadState {
    engine: Engine,
    rx: mpsc::Receiver<EngineCommand>,
    producer: Producer<StereoFrame>,
    midi_input_queue: Arc<ArrayQueue<RealtimeMidiMessage>>,
    priority_actions: Arc<PriorityActions>,
    transport_metrics: Arc<TransportMetrics>,
    left: Vec<f32>,
    right: Vec<f32>,
    note_events: Vec<Scheduled<NoteEvent>>,
    controller_events: Vec<Scheduled<ControllerEvent>>,
    snapshot_requests: Vec<mpsc::Sender<EngineSnapshot>>,
}

impl EngineThreadState {
    fn new(
        engine: Engine,
        rx: mpsc::Receiver<EngineCommand>,
        producer: Producer<StereoFrame>,
        midi_input_queue: Arc<ArrayQueue<RealtimeMidiMessage>>,
        priority_actions: Arc<PriorityActions>,
        transport_metrics: Arc<TransportMetrics>,
    ) -> Self {
        Self {
            engine,
            rx,
            producer,
            midi_input_queue,
            priority_actions,
            transport_metrics,
            left: vec![0.0; ENGINE_RENDER_BLOCK_FRAMES],
            right: vec![0.0; ENGINE_RENDER_BLOCK_FRAMES],
            note_events: Vec::with_capacity(32),
            controller_events: Vec::with_capacity(64),
            snapshot_requests: Vec::with_capacity(4),
        }
    }

    fn run(mut self) {
        loop {
            if !self.apply_priority_actions() {
                break;
            }
            self.drain_realtime_midi_nonblocking();
            if !self.drain_commands_nonblocking() {
                break;
            }

            self.flush_snapshot_requests();

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
    }

    fn drain_commands_nonblocking(&mut self) -> bool {
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

    fn drain_realtime_midi_nonblocking(&mut self) {
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

    fn handle_command(&mut self, command: EngineCommand) -> bool {
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
            EngineCommand::Shutdown => return false,
        }
        true
    }

    fn apply_priority_actions(&mut self) -> bool {
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

    fn flush_snapshot_requests(&mut self) {
        if self.snapshot_requests.is_empty() {
            return;
        }

        let snapshot = self.engine.snapshot();
        for reply in self.snapshot_requests.drain(..) {
            let _ = reply.send(snapshot.clone());
        }
    }

    fn queue_needs_audio(&self) -> bool {
        let capacity_frames = self.producer.buffer().capacity();
        let queued_frames = capacity_frames.saturating_sub(self.producer.slots());
        let target_frames = self.transport_metrics.queue_target_frames(capacity_frames);
        self.transport_metrics.set_queued_frames(queued_frames);
        queued_frames < target_frames
    }

    fn render_audio_block(&mut self) {
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
    }
}

fn spawn_engine_thread(
    engine: Engine,
    rx: mpsc::Receiver<EngineCommand>,
    producer: Producer<StereoFrame>,
    midi_input_queue: Arc<ArrayQueue<RealtimeMidiMessage>>,
    priority_actions: Arc<PriorityActions>,
    transport_metrics: Arc<TransportMetrics>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        EngineThreadState::new(
            engine,
            rx,
            producer,
            midi_input_queue,
            priority_actions,
            transport_metrics,
        )
        .run()
    })
}

fn drain_queue_into_output(
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

#[cfg(test)]
fn push_frames_into_queue(
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

fn push_rendered_channels_into_queue(
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

fn write_channels_into_stereo_frames(
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

fn write_output_frame(frame: &mut [f32], left: f32, right: f32) {
    let mono = (left + right) * 0.5;

    frame[0] = left;
    if frame.len() > 1 {
        frame[1] = right;
    }
    for sample in frame.iter_mut().skip(2) {
        *sample = mono;
    }
}

fn write_frames_into_output(
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

fn zero_fill_remaining_output(
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

fn zero_fill_partial_output_tail(output: &mut [f32], channel_count: usize) {
    let full_sample_count = (output.len() / channel_count) * channel_count;
    for sample in &mut output[full_sample_count..] {
        *sample = 0.0;
    }
}

struct OpenedMidiConnection {
    port_name: String,
    _connection: MidiInputConnection<()>,
}

#[derive(Debug, Clone, Copy)]
enum ParsedMidiMessage {
    Realtime(RealtimeMidiMessage),
    Runtime(RuntimeControlMessage),
    Reserved,
}

fn parse_midi_message(
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

fn parse_profile_cc_binding(binding: &ControllerBinding, value: f32) -> Option<ParsedMidiMessage> {
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
        ControllerBindingAction::Runtime(message) => {
            (value >= 0.5).then_some(ParsedMidiMessage::Runtime(message))
        }
        ControllerBindingAction::ToggleParam(id) => (value >= 0.5).then_some(
            ParsedMidiMessage::Runtime(RuntimeControlMessage::ToggleParam(id)),
        ),
        ControllerBindingAction::Reserved => Some(ParsedMidiMessage::Reserved),
    }
}

fn run_demo_performance(tx: mpsc::Sender<EngineCommand>, thread_stop: Arc<AtomicBool>) {
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

fn sleep_interruptibly(stop: &AtomicBool, duration: Duration) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn pc4_full_profile() -> ControllerProfile {
        controller_profile_from_toml(
            include_str!("../../../profiles/pc4-full.toml"),
            Path::new("profiles/pc4-full.toml"),
        )
        .expect("pc4-full profile parses")
    }

    fn test_engine_thread_state() -> EngineThreadState {
        let patch = load_patch_from_path(&default_patch_path()).expect("default patch loads");
        let engine = Engine::new(
            EngineConfig {
                sample_rate_hz: ALSA_PLAYBACK_SAMPLE_RATE_HZ as f32,
                max_block_frames: 2_048,
                voice_count: 6,
            },
            patch,
        )
        .expect("engine builds");
        let (_tx, rx) = mpsc::channel();
        let (producer, _consumer) = RingBuffer::<StereoFrame>::new(AUDIO_QUEUE_CAPACITY_FRAMES);
        EngineThreadState::new(
            engine,
            rx,
            producer,
            Arc::new(ArrayQueue::new(MIDI_INPUT_QUEUE_CAPACITY)),
            Arc::new(PriorityActions::default()),
            Arc::new(TransportMetrics::default()),
        )
    }

    fn test_snapshot() -> EngineSnapshot {
        let patch = load_patch_from_path(&default_patch_path()).expect("default patch loads");
        Engine::new(
            EngineConfig {
                sample_rate_hz: ALSA_PLAYBACK_SAMPLE_RATE_HZ as f32,
                max_block_frames: 2_048,
                voice_count: 6,
            },
            patch,
        )
        .expect("engine builds")
        .snapshot()
    }

    #[test]
    fn resolve_factory_patch_by_stem() {
        let path = resolve_patch_argument(Some("molten-horizon")).expect("factory patch resolves");
        assert!(path.ends_with("patches/factory/molten-horizon.toml"));
    }

    #[test]
    fn resolve_factory_patch_by_patch_name_slug() {
        let path = resolve_patch_argument(Some("Molten Horizon")).expect("patch name resolves");
        assert!(path.ends_with("patches/factory/molten-horizon.toml"));
    }

    #[test]
    fn parse_play_options_supports_device_selection() {
        let args = vec![
            "--demo".to_string(),
            "--headless".to_string(),
            "--audio-device".to_string(),
            "hw:4,0".to_string(),
            "--alsa-period-frames".to_string(),
            "256".to_string(),
            "--alsa-buffer-frames".to_string(),
            "1024".to_string(),
            "--alsa-start-threshold-frames".to_string(),
            "1024".to_string(),
            "--midi-device".to_string(),
            "Launchkey".to_string(),
            "--midi-channel".to_string(),
            "3".to_string(),
            "--controller-profile".to_string(),
            "profiles/pc4-full.toml".to_string(),
            "--trace-midi".to_string(),
            "razor-thaw".to_string(),
        ];

        let options = parse_play_options(&args).expect("play options parse");
        assert!(options.force_demo);
        assert!(options.headless);
        assert!(options.trace_midi);
        assert_eq!(options.audio_selector.as_deref(), Some("hw:4,0"));
        assert_eq!(options.alsa_period_frames, Some(256));
        assert_eq!(options.alsa_buffer_frames, Some(1024));
        assert_eq!(options.alsa_start_threshold_frames, Some(1024));
        assert_eq!(options.midi_selector.as_deref(), Some("Launchkey"));
        assert_eq!(options.midi_channel, Some(3));
        assert!(
            options
                .controller_profile_path
                .as_ref()
                .expect("profile path set")
                .ends_with("profiles/pc4-full.toml")
        );
        assert!(
            options
                .patch_path
                .ends_with("patches/factory/razor-thaw.toml")
        );
    }

    #[test]
    fn normalize_alsa_hw_selector_rejects_non_hw_paths() {
        assert_eq!(
            normalize_alsa_hw_selector("hw:4,0").expect("hw selector normalizes"),
            "hw:4,0"
        );
        assert!(normalize_alsa_hw_selector("default").is_err());
        assert!(normalize_alsa_hw_selector("pipewire").is_err());
        assert!(normalize_alsa_hw_selector("plughw:4,0").is_err());
    }

    #[test]
    fn same_hw_selector_matches_normalized_hw_pairs() {
        assert!(same_hw_selector(Some("hw:04,0"), "hw:4,0"));
        assert!(!same_hw_selector(Some("hw:4,0"), "hw:5,0"));
        assert!(!same_hw_selector(Some("default"), "hw:4,0"));
    }

    #[test]
    fn alsa_tuning_rejects_invalid_relationships() {
        assert!(
            AlsaPlaybackTuning {
                period_frames: 512,
                buffer_frames: 256,
                start_threshold_frames: 256,
            }
            .validate()
            .is_err()
        );
        assert!(
            AlsaPlaybackTuning {
                period_frames: 256,
                buffer_frames: 1024,
                start_threshold_frames: 2048,
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn parse_alsa_proc_lines_extract_expected_fields() {
        assert_eq!(
            parse_alsa_card_line(" 4 [AG06AG03       ]: USB-Audio - AG06/AG03"),
            Some((4, "AG06/AG03".to_string()))
        );
        assert_eq!(
            parse_alsa_pcm_line("04-00: USB Audio : USB Audio : playback 1 : capture 1"),
            Some((4, 0, "USB Audio".to_string(), true))
        );
    }

    #[test]
    fn parse_runtime_ui_command_supports_patch_and_macro_commands() {
        assert_eq!(
            parse_runtime_ui_command("patch Cathedral Bloom").expect("patch command parses"),
            RuntimeUiCommand::Patch("Cathedral Bloom".to_string())
        );
        assert_eq!(
            parse_runtime_ui_command("favorite 3").expect("favorite command parses"),
            RuntimeUiCommand::Favorite(3)
        );
        assert_eq!(
            parse_runtime_ui_command("macro gravitacija 0.74").expect("macro command parses"),
            RuntimeUiCommand::Macro(MacroId::Gravitacija, 0.74)
        );
        assert_eq!(
            parse_runtime_ui_command("panic").expect("panic parses"),
            RuntimeUiCommand::Panic
        );
        assert_eq!(
            parse_runtime_ui_command("audio Scarlett").expect("audio command parses"),
            RuntimeUiCommand::AudioSelect("Scarlett".to_string())
        );
        assert_eq!(
            parse_runtime_ui_command("midi 1").expect("midi command parses"),
            RuntimeUiCommand::MidiSelect("1".to_string())
        );
        assert_eq!(
            parse_runtime_ui_command("next").expect("next parses"),
            RuntimeUiCommand::NextFavorite
        );
        assert_eq!(
            parse_runtime_ui_command("demo-patch").expect("demo patch parses"),
            RuntimeUiCommand::DemoPatch
        );
    }

    #[test]
    fn factory_bank_has_productized_patch_set() {
        let entries = factory_patch_entries().expect("factory entries load");
        assert!(entries.len() >= 8);
        assert!(entries.iter().any(|entry| entry.favorite));
    }

    #[test]
    fn favorite_navigation_wraps_across_live_set() {
        let next = adjacent_live_patch(Path::new("patches/factory/glass-tide.toml"), 1)
            .expect("favorite next resolves");
        assert!(next.ends_with("patches/factory/molten-horizon.toml"));

        let prev = adjacent_live_patch(Path::new("patches/factory/molten-horizon.toml"), -1)
            .expect("favorite prev resolves");
        assert!(prev.ends_with("patches/factory/glass-tide.toml"));
    }

    #[test]
    fn live_set_slots_follow_locked_pc4_order() {
        let live_set = live_patch_entries().expect("live set loads");
        assert_eq!(live_set.len(), 8);
        assert_eq!(live_set[0].stem, "molten-horizon");
        assert_eq!(live_set[1].stem, "cathedral-bloom");
        assert_eq!(live_set[7].stem, "glass-tide");
    }

    #[test]
    fn snapshot_requests_are_one_shot_and_do_not_drain_events() {
        let mut state = test_engine_thread_state();
        let (reply_tx, reply_rx) = mpsc::channel();
        state.snapshot_requests.push(reply_tx);
        state.note_events.push(Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: 60,
                velocity: 1.0,
            },
        });
        state.controller_events.push(Scheduled {
            frame_offset: 0,
            event: ControllerEvent::Macro {
                id: MacroId::Bloom,
                value: 0.5,
            },
        });

        state.flush_snapshot_requests();

        assert!(reply_rx.recv_timeout(Duration::from_millis(50)).is_ok());
        assert!(reply_rx.try_recv().is_err());
        assert!(state.snapshot_requests.is_empty());
        assert_eq!(state.note_events.len(), 1);
        assert_eq!(state.controller_events.len(), 1);
    }

    #[test]
    fn parse_midi_message_maps_pc4_knobs_and_program_change() {
        let parsed = parse_midi_message(&[0xB0, 16, 64], 2.0, None, None).expect("cc16 parses");
        match parsed {
            ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
                ControllerEvent::Macro { id, value },
            )) => {
                assert_eq!(id, MacroId::Gravitacija);
                assert!((value - (64.0 / 127.0)).abs() < 0.0001);
            }
            other => panic!("unexpected parsed MIDI message: {other:?}"),
        }

        let parsed = parse_midi_message(&[0xB0, 20, 100], 2.0, None, None).expect("cc20 parses");
        match parsed {
            ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
                ControllerEvent::Macro { id, value },
            )) => {
                assert_eq!(id, MacroId::Swarm);
                assert!((value - (100.0 / 127.0)).abs() < 0.0001);
            }
            other => panic!("unexpected parsed MIDI message: {other:?}"),
        }

        let parsed =
            parse_midi_message(&[0xC0, 5], 2.0, None, None).expect("program change parses");
        match parsed {
            ParsedMidiMessage::Runtime(RuntimeControlMessage::ProgramChange(slot)) => {
                assert_eq!(slot, 5);
            }
            other => panic!("unexpected parsed MIDI message: {other:?}"),
        }
    }

    #[test]
    fn parse_midi_message_covers_pc4gen_mamut_smoke_contract() {
        for (cc, expected_id) in [
            (16, MacroId::Gravitacija),
            (17, MacroId::Bloom),
            (18, MacroId::Heat),
            (19, MacroId::Ruin),
            (20, MacroId::Swarm),
        ] {
            let parsed = parse_midi_message(&[0xB0, cc, 127], 2.0, None, None)
                .expect("macro control parses");
            match parsed {
                ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
                    ControllerEvent::Macro { id, value },
                )) => {
                    assert_eq!(id, expected_id);
                    assert!((value - 1.0).abs() < 0.0001);
                }
                other => panic!("unexpected parsed MIDI message: {other:?}"),
            }
        }

        assert!(matches!(
            parse_midi_message(&[0xB0, 1, 127], 2.0, None, None),
            Some(ParsedMidiMessage::Realtime(
                RealtimeMidiMessage::Controller(ControllerEvent::ModWheel { .. })
            ))
        ));
        assert!(matches!(
            parse_midi_message(&[0xB0, 64, 127], 2.0, None, None),
            Some(ParsedMidiMessage::Realtime(
                RealtimeMidiMessage::Controller(ControllerEvent::Sustain { down: true })
            ))
        ));
        assert!(matches!(
            parse_midi_message(&[0xB0, 64, 0], 2.0, None, None),
            Some(ParsedMidiMessage::Realtime(
                RealtimeMidiMessage::Controller(ControllerEvent::Sustain { down: false })
            ))
        ));
        assert!(matches!(
            parse_midi_message(&[0xD0, 100], 2.0, None, None),
            Some(ParsedMidiMessage::Realtime(
                RealtimeMidiMessage::Controller(ControllerEvent::ChannelAftertouch { .. })
            ))
        ));
        assert!(matches!(
            parse_midi_message(&[0xE0, 0x00, 0x40], 2.0, None, None),
            Some(ParsedMidiMessage::Realtime(
                RealtimeMidiMessage::Controller(ControllerEvent::PitchBend { .. })
            ))
        ));

        let slots = (0..8)
            .map(
                |slot| match parse_midi_message(&[0xC0, slot], 2.0, None, None) {
                    Some(ParsedMidiMessage::Runtime(RuntimeControlMessage::ProgramChange(
                        slot,
                    ))) => slot,
                    other => panic!("unexpected parsed MIDI message: {other:?}"),
                },
            )
            .collect::<Vec<_>>();
        assert_eq!(slots, (0..8).collect::<Vec<_>>());
    }

    #[test]
    fn controller_profile_loads_pc4_full_bindings() {
        let profile = pc4_full_profile();
        assert_eq!(profile.name, "pc4-full");
        assert_eq!(profile.bindings_by_cc.len(), 27);
        assert!(matches!(
            profile.binding_for_cc(71).map(|binding| binding.action),
            Some(ControllerBindingAction::Macro(MacroId::Gravitacija))
        ));
        assert!(matches!(
            profile.binding_for_cc(80).map(|binding| binding.action),
            Some(ControllerBindingAction::Runtime(
                RuntimeControlMessage::Panic
            ))
        ));
        assert!(matches!(
            profile.binding_for_cc(90).map(|binding| binding.action),
            Some(ControllerBindingAction::Reserved)
        ));
        let knobs = sorted_bindings_for_section(&profile, ControllerBindingSection::Knob);
        let sliders = sorted_bindings_for_section(&profile, ControllerBindingSection::Slider);
        let switches = sorted_bindings_for_section(&profile, ControllerBindingSection::Switch);
        assert_eq!(knobs.len(), 9);
        assert_eq!(sliders.len(), 9);
        assert_eq!(switches.len(), 9);
        assert_eq!(knobs[0].control, "K1 Filter 1");
        assert_eq!(sliders[6].control, "S7");
        assert_eq!(switches[8].control, "SW9");
    }

    #[test]
    fn controller_profile_groups_unknown_controls_as_other() {
        let input = r#"
[[binding]]
control = "X1"
cc = 44
kind = "reserved"
"#;
        let profile =
            controller_profile_from_toml(input, Path::new("other.toml")).expect("profile parses");
        let binding = profile.binding_for_cc(44).expect("binding exists");
        assert_eq!(binding.section, ControllerBindingSection::Other);
        assert_eq!(binding.index, None);
    }

    #[test]
    fn pc4_display_values_come_from_mamut_snapshot() {
        let profile = pc4_full_profile();
        let snapshot = test_snapshot();
        let cutoff = profile.binding_for_cc(26).expect("s7 binding");
        let attack = profile.binding_for_cc(72).expect("k3 binding");
        let chorus = profile.binding_for_cc(85).expect("sw5 binding");
        let reserved = profile.binding_for_cc(3).expect("k8 binding");

        assert!(
            binding_display_value(&snapshot, cutoff).0.contains("Hz"),
            "cutoff should display in Hz"
        );
        assert!(
            binding_display_value(&snapshot, attack).0.contains("ms"),
            "attack should display in ms"
        );
        assert!(matches!(
            binding_display_value(&snapshot, chorus).0.as_str(),
            "on" | "off"
        ));
        assert_eq!(binding_display_value(&snapshot, reserved).0, "Reserved");
    }

    #[test]
    fn last_control_event_tracks_profile_reserved_and_program_change() {
        let profile = pc4_full_profile();
        let reserved = parse_midi_message(&[0xB0, 3, 64], 2.0, None, Some(&profile));
        let event = last_control_event(
            &[0xB0, 3, 64],
            None,
            Some(&profile),
            reserved,
            Instant::now(),
        )
        .expect("reserved event");
        assert_eq!(event.kind, LastControlKind::ProfileCc(3));
        assert_eq!(event.verdict, LastControlVerdict::Reserved);

        let parsed = parse_midi_message(&[0xC0, 4], 2.0, None, Some(&profile));
        let event = last_control_event(&[0xC0, 4], None, Some(&profile), parsed, Instant::now())
            .expect("program change event");
        assert_eq!(event.kind, LastControlKind::ProgramChange(4));
        assert_eq!(event.program, Some(4));
    }

    #[test]
    fn parse_midi_message_uses_pc4_full_profile() {
        let profile = pc4_full_profile();

        let parsed =
            parse_midi_message(&[0xB0, 71, 127], 2.0, None, Some(&profile)).expect("k1 parses");
        match parsed {
            ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
                ControllerEvent::Macro { id, value },
            )) => {
                assert_eq!(id, MacroId::Gravitacija);
                assert!((value - 1.0).abs() < 0.0001);
            }
            other => panic!("unexpected parsed MIDI message: {other:?}"),
        }

        let parsed =
            parse_midi_message(&[0xB0, 72, 127], 2.0, None, Some(&profile)).expect("k3 parses");
        match parsed {
            ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
                ControllerEvent::DirectParam { id, value },
            )) => {
                assert_eq!(id, ParamId::AmpEnvAttackMs);
                assert!((value - param_spec(ParamId::AmpEnvAttackMs).max).abs() < 0.01);
            }
            other => panic!("unexpected parsed MIDI message: {other:?}"),
        }

        let parsed =
            parse_midi_message(&[0xB0, 26, 127], 2.0, None, Some(&profile)).expect("s7 parses");
        match parsed {
            ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
                ControllerEvent::DirectParam { id, value },
            )) => {
                assert_eq!(id, ParamId::FilterCutoffHz);
                assert!((value - param_spec(ParamId::FilterCutoffHz).max).abs() < 0.01);
            }
            other => panic!("unexpected parsed MIDI message: {other:?}"),
        }

        assert!(matches!(
            parse_midi_message(&[0xB0, 80, 127], 2.0, None, Some(&profile)),
            Some(ParsedMidiMessage::Runtime(RuntimeControlMessage::Panic))
        ));
        assert!(parse_midi_message(&[0xB0, 80, 0], 2.0, None, Some(&profile)).is_none());
        assert!(matches!(
            parse_midi_message(&[0xB0, 3, 64], 2.0, None, Some(&profile)),
            Some(ParsedMidiMessage::Reserved)
        ));
    }

    #[test]
    fn controller_profile_rejects_duplicate_cc() {
        let input = r#"
[[binding]]
control = "A"
cc = 71
kind = "macro"
target = "gravitacija"

[[binding]]
control = "B"
cc = 71
kind = "macro"
target = "bloom"
"#;
        assert!(controller_profile_from_toml(input, Path::new("dup.toml")).is_err());
    }

    #[test]
    fn controller_profile_rejects_unknown_param() {
        let input = r#"
[[binding]]
control = "S1"
cc = 12
kind = "direct_param"
target = "not_a_param"
"#;
        assert!(controller_profile_from_toml(input, Path::new("unknown.toml")).is_err());
    }

    #[test]
    fn controller_profile_rejects_boolean_direct_param() {
        let input = r#"
[[binding]]
control = "SW5"
cc = 85
kind = "direct_param"
target = "chorus_enabled"
"#;
        assert!(controller_profile_from_toml(input, Path::new("boolean.toml")).is_err());
    }

    #[test]
    fn parse_midi_message_respects_channel_filter() {
        assert!(parse_midi_message(&[0x90, 60, 100], 2.0, Some(2), None).is_none());

        let parsed = parse_midi_message(&[0x91, 60, 100], 2.0, Some(2), None)
            .expect("channel 2 note parses");
        match parsed {
            ParsedMidiMessage::Realtime(RealtimeMidiMessage::Note(NoteEvent::NoteOn {
                note,
                velocity,
            })) => {
                assert_eq!(note, 60);
                assert!((velocity - (100.0 / 127.0)).abs() < 0.0001);
            }
            other => panic!("unexpected parsed MIDI message: {other:?}"),
        }
    }

    #[test]
    fn queue_drain_silence_fills_missing_frames_and_counts_underrun() {
        let metrics = Arc::new(TransportMetrics::default());
        let (mut producer, mut consumer) = RingBuffer::<StereoFrame>::new(4);
        push_frames_into_queue(&mut producer, &metrics, &[[0.25, -0.25]]);

        let mut output = vec![1.0_f32; 6];
        drain_queue_into_output(&mut consumer, &metrics, &mut output, 2);

        assert_eq!(output, vec![0.25, -0.25, 0.0, 0.0, 0.0, 0.0]);
        assert_eq!(
            metrics.snapshot(),
            TransportMetricsSnapshot {
                queued_frames: 0,
                queue_target_frames: 0,
                write_frames_hint: 3,
                underrun_batches: 1,
                underrun_frames: 2,
                xrun_recoveries: 0,
                overflow_batches: 0,
                overflow_frames: 0,
            }
        );
    }

    #[test]
    fn queue_drain_zero_fills_partial_tail_samples() {
        let metrics = Arc::new(TransportMetrics::default());
        let (mut producer, mut consumer) = RingBuffer::<StereoFrame>::new(4);
        push_frames_into_queue(&mut producer, &metrics, &[[0.5, -0.5]]);

        let mut output = vec![1.0_f32; 5];
        drain_queue_into_output(&mut consumer, &metrics, &mut output, 2);

        assert_eq!(output, vec![0.5, -0.5, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn s32_playback_conversion_clips_and_sanitizes_samples() {
        assert_eq!(f32_sample_to_s32(1.0), i32::MAX);
        assert_eq!(f32_sample_to_s32(2.0), i32::MAX);
        assert_eq!(f32_sample_to_s32(-1.0), i32::MIN);
        assert_eq!(f32_sample_to_s32(-2.0), i32::MIN);
        assert_eq!(f32_sample_to_s32(0.0), 0);
        assert_eq!(f32_sample_to_s32(f32::NAN), 0);
    }

    #[test]
    fn s32_playback_conversion_preserves_interleaved_order() {
        let samples = [0.0, 1.0, -1.0, f32::INFINITY];
        let mut output = [123_i32; 4];

        convert_f32_samples_to_s32(&samples, &mut output);

        assert_eq!(output, [0, i32::MAX, i32::MIN, 0]);
    }

    #[test]
    fn queue_push_records_overflow_without_blocking() {
        let metrics = Arc::new(TransportMetrics::default());
        let (mut producer, mut consumer) = RingBuffer::<StereoFrame>::new(2);
        push_frames_into_queue(
            &mut producer,
            &metrics,
            &[[0.1, 0.2], [0.3, 0.4], [0.5, 0.6]],
        );

        assert_eq!(
            metrics.snapshot(),
            TransportMetricsSnapshot {
                queued_frames: 2,
                queue_target_frames: 0,
                write_frames_hint: 0,
                underrun_batches: 0,
                underrun_frames: 0,
                xrun_recoveries: 0,
                overflow_batches: 1,
                overflow_frames: 1,
            }
        );
        assert_eq!(consumer.pop().expect("first frame"), [0.1, 0.2]);
        assert_eq!(consumer.pop().expect("second frame"), [0.3, 0.4]);
        assert!(consumer.pop().is_err());
    }

    #[test]
    fn queue_target_stays_on_default_without_write_hint() {
        let metrics = TransportMetrics::default();
        assert_eq!(
            metrics.queue_target_frames(1_024),
            AUDIO_QUEUE_TARGET_FRAMES
        );
    }

    #[test]
    fn queue_target_scales_to_observed_write_size() {
        let metrics = TransportMetrics::default();
        metrics.record_write_request(900);

        assert_eq!(metrics.queue_target_frames(1_536), 1_024);
        assert_eq!(
            metrics.snapshot(),
            TransportMetricsSnapshot {
                queued_frames: 0,
                queue_target_frames: 1_024,
                write_frames_hint: 900,
                underrun_batches: 0,
                underrun_frames: 0,
                xrun_recoveries: 0,
                overflow_batches: 0,
                overflow_frames: 0,
            }
        );
    }
}
