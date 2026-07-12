use super::*;

pub struct AudioRuntime {
    pub tx: mpsc::Sender<EngineCommand>,
    pub worker: EngineWorker,
    pub stream: AlsaPlaybackStream,
    pub audio_selector: String,
    pub audio_device_name: String,
    pub sample_rate_hz: u32,
    pub channels: usize,
    pub alsa_tuning: AlsaPlaybackTuning,
    pub bend_range: f32,
    pub patch_name: String,
    pub midi_input_queue: Arc<ArrayQueue<RealtimeMidiMessage>>,
    pub priority_actions: Arc<PriorityActions>,
    pub transport_metrics: Arc<TransportMetrics>,
    pub scope_consumer: Consumer<StereoFrame>,
    pub scope_enabled: Arc<AtomicBool>,
}

pub struct PreparedAudioRuntime {
    pub tx: mpsc::Sender<EngineCommand>,
    pub worker: EngineWorker,
    pub consumer: Consumer<StereoFrame>,
    pub selected_device: AlsaOutputDevice,
    pub audio_selector: String,
    pub audio_device_name: String,
    pub sample_rate_hz: u32,
    pub channels: usize,
    pub alsa_tuning: AlsaPlaybackTuning,
    pub bend_range: f32,
    pub patch_name: String,
    pub midi_input_queue: Arc<ArrayQueue<RealtimeMidiMessage>>,
    pub priority_actions: Arc<PriorityActions>,
    pub transport_metrics: Arc<TransportMetrics>,
    pub scope_consumer: Consumer<StereoFrame>,
    pub scope_enabled: Arc<AtomicBool>,
}

pub struct DemoPerformer {
    pub stop: Arc<AtomicBool>,
    pub join_handle: Option<JoinHandle<()>>,
}

impl DemoPerformer {
    pub fn spawn(tx: mpsc::Sender<EngineCommand>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let join_handle = thread::spawn(move || run_demo_performance(tx, thread_stop));
        Self {
            stop,
            join_handle: Some(join_handle),
        }
    }

    pub fn shutdown(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

pub enum PerformanceDriver {
    Demo(DemoPerformer),
    Midi(OpenedMidiConnection),
    Idle,
}

impl PerformanceDriver {
    pub fn is_demo(&self) -> bool {
        matches!(self, Self::Demo(_))
    }

    pub fn detail(&self) -> String {
        match self {
            Self::Demo(_) => "demo performer active".to_string(),
            Self::Midi(connection) => format!("connected ({})", connection.port_name),
            Self::Idle => "no active input".to_string(),
        }
    }

    pub fn stop(&mut self) {
        let previous = std::mem::replace(self, Self::Idle);
        previous.shutdown();
    }

    pub fn shutdown(self) {
        match self {
            Self::Demo(demo) => demo.shutdown(),
            Self::Midi(_) | Self::Idle => {}
        }
    }
}

pub struct RuntimeSession {
    pub patch_path: PathBuf,
    pub patch_name: String,
    pub audio_selector: Option<String>,
    pub alsa_tuning: AlsaPlaybackTuning,
    pub midi_selector: Option<String>,
    pub midi_channel: Option<u8>,
    pub controller_profile: Option<Arc<ControllerProfile>>,
    pub trace_midi: bool,
    pub terminal_output_enabled: bool,
    pub gfm_layer_seed: Option<u64>,
    pub bcs_layer_scenario: Option<BcsScenario>,
    pub mozaik_seed: Option<u64>,
    pub bend_range: f32,
    pub tx: mpsc::Sender<EngineCommand>,
    pub midi_input_queue: Arc<ArrayQueue<RealtimeMidiMessage>>,
    pub runtime_control_queue: Arc<ArrayQueue<RuntimeControlMessage>>,
    pub priority_actions: Arc<PriorityActions>,
    pub worker: EngineWorker,
    pub stream: Option<AlsaPlaybackStream>,
    pub driver: PerformanceDriver,
    pub audio_device_name: String,
    pub sample_rate_hz: u32,
    pub channels: usize,
    pub transport_metrics: Arc<TransportMetrics>,
    pub input_metrics: Arc<InputMetrics>,
    pub recording_metrics: Arc<RecordingMetrics>,
    pub scope_consumer: Consumer<StereoFrame>,
    pub scope_enabled: Arc<AtomicBool>,
    pub midi_trace_log: Arc<MidiTraceLog>,
    pub midi_trace_worker: MidiTraceWorker,
    pub sound_lab_midi_focus: Arc<SoundLabMidiFocus>,
}

pub struct EngineWorker {
    pub join_handle: Option<JoinHandle<()>>,
}

impl EngineWorker {
    pub fn new(join_handle: JoinHandle<()>) -> Self {
        Self {
            join_handle: Some(join_handle),
        }
    }

    pub fn shutdown(mut self, tx: mpsc::Sender<EngineCommand>) {
        let _ = tx.send(EngineCommand::Shutdown);
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

pub struct OpenedAlsaPlayback {
    pub pcm: PCM,
    pub audio_selector: String,
    pub audio_device_name: String,
    pub sample_rate_hz: u32,
    pub channels: usize,
    pub sample_format: AlsaPlaybackSampleFormat,
    pub tuning: AlsaPlaybackTuning,
}

pub struct AlsaPlaybackStream {
    pub stop: Arc<AtomicBool>,
    pub join_handle: Option<JoinHandle<()>>,
}

impl AlsaPlaybackStream {
    pub fn spawn(
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

    pub fn shutdown(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}
