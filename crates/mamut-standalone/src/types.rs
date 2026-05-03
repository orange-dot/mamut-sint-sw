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
pub(crate) const ALSA_PLAYBACK_SAMPLE_RATE_HZ: u32 = 96_000;
pub(crate) const ALSA_PLAYBACK_SAMPLE_RATE_HZ_ALLOWED: [u32; 6] =
    [44_100, 48_000, 88_200, 96_000, 176_400, 192_000];
pub(crate) const ALSA_PERIOD_FRAMES_DEFAULT: usize = 256;
pub(crate) const ALSA_BUFFER_FRAMES_DEFAULT: usize = 1_024;
pub(crate) const ALSA_START_THRESHOLD_FRAMES_DEFAULT: usize = ALSA_BUFFER_FRAMES_DEFAULT;
pub(crate) const PERFORMANCE_UI_REFRESH: Duration = Duration::from_millis(75);
pub(crate) const MIDI_ACTIVITY_FLASH: Duration = Duration::from_millis(700);
pub(crate) const MIDI_STARTUP_GUARD: Duration = MIDI_ACTIVITY_FLASH;
pub(crate) const MIDI_INPUT_QUEUE_CAPACITY: usize = 512;
pub(crate) const MIDI_TRACE_QUEUE_CAPACITY: usize = 4096;
pub(crate) const MIDI_TRACE_RAW_BYTES: usize = 4;
pub(crate) const RUNTIME_CONTROL_QUEUE_CAPACITY: usize = 64;
pub(crate) const RECORDING_QUEUE_CAPACITY_FRAMES: usize = 192_000 * 4;
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
    pub(crate) sample_rate_hz: u32,
    pub(crate) max_frames: Option<usize>,
}

#[derive(Debug, Clone)]
pub(crate) struct MidiTraceLogStart {
    pub(crate) wav_path: PathBuf,
    pub(crate) log_path: PathBuf,
    pub(crate) patch_path: PathBuf,
    pub(crate) patch_name: String,
    pub(crate) sample_rate_hz: u32,
    pub(crate) max_frames: Option<usize>,
    pub(crate) midi_channel: Option<u8>,
    pub(crate) controller_profile: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MidiTraceLogStarted {
    pub(crate) path: PathBuf,
    pub(crate) generation: u64,
}

pub(crate) struct MidiTraceLog {
    pub(crate) active: Mutex<Option<ActiveMidiTraceLog>>,
    pub(crate) active_flag: AtomicBool,
    pub(crate) generation: AtomicU64,
}

pub(crate) struct ActiveMidiTraceLog {
    pub(crate) path: PathBuf,
    pub(crate) generation: u64,
    pub(crate) writer: BufWriter<File>,
    pub(crate) target_end_at: Option<Instant>,
    pub(crate) lines_written: u64,
    pub(crate) write_failed: bool,
}

impl Default for MidiTraceLog {
    fn default() -> Self {
        Self {
            active: Mutex::new(None),
            active_flag: AtomicBool::new(false),
            generation: AtomicU64::new(0),
        }
    }
}

impl MidiTraceLog {
    pub(crate) fn start(&self, request: MidiTraceLogStart) -> io::Result<MidiTraceLogStarted> {
        let mut active = self
            .active
            .lock()
            .map_err(|_| io::Error::other("MIDI trace log lock poisoned"))?;
        if active.is_some() {
            return Err(io::Error::other("MIDI trace log is already active"));
        }

        let started_at = Instant::now();
        let target_end_at = request.max_frames.and_then(|frames| {
            let seconds = frames as f64 / f64::from(request.sample_rate_hz.max(1));
            started_at.checked_add(Duration::from_secs_f64(seconds))
        });
        let mut writer = BufWriter::new(File::create(&request.log_path)?);
        write_midi_trace_log_header(&mut writer, &request)?;
        let path = request.log_path.clone();
        let generation = self.generation.fetch_add(1, Ordering::Relaxed) + 1;

        *active = Some(ActiveMidiTraceLog {
            path: path.clone(),
            generation,
            writer,
            target_end_at,
            lines_written: 0,
            write_failed: false,
        });
        self.active_flag.store(true, Ordering::Relaxed);
        Ok(MidiTraceLogStarted { path, generation })
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active_flag.load(Ordering::Relaxed)
    }

    pub(crate) fn write_line(&self, received_at: Instant, line: &str) {
        if !self.is_active() {
            return;
        }

        let Ok(mut active) = self.active.lock() else {
            return;
        };
        let Some(active) = active.as_mut() else {
            self.active_flag.store(false, Ordering::Relaxed);
            return;
        };
        if active
            .target_end_at
            .is_some_and(|target_end_at| received_at > target_end_at)
        {
            return;
        }
        if active.write_failed {
            return;
        }

        if let Err(error) = writeln!(active.writer, "{line}") {
            active.write_failed = true;
            eprintln!(
                "midi trace sidecar write failed for {}: {error}",
                active.path.display()
            );
            return;
        }
        active.lines_written += 1;
    }

    #[cfg(test)]
    pub(crate) fn finish(&self, reason: &str) -> Option<PathBuf> {
        self.finish_with_trace_drops(reason, 0)
    }

    pub(crate) fn finish_with_trace_drops(
        &self,
        reason: &str,
        trace_records_dropped: u64,
    ) -> Option<PathBuf> {
        self.active_flag.store(false, Ordering::Relaxed);
        let Ok(mut active) = self.active.lock() else {
            return None;
        };
        let Some(mut active) = active.take() else {
            return None;
        };

        let _ = writeln!(active.writer, "#");
        let _ = writeln!(active.writer, "# finish: {reason}");
        let _ = writeln!(active.writer, "# midi_lines: {}", active.lines_written);
        if trace_records_dropped > 0 {
            let _ = writeln!(
                active.writer,
                "# trace_records_dropped: {trace_records_dropped}"
            );
        }
        let _ = active.writer.flush();
        Some(active.path)
    }

    pub(crate) fn finish_generation_with_trace_drops(
        &self,
        generation: u64,
        reason: &str,
        trace_records_dropped: u64,
    ) -> Option<PathBuf> {
        let Ok(mut active) = self.active.lock() else {
            return None;
        };
        if !active
            .as_ref()
            .is_some_and(|active| active.generation == generation)
        {
            return None;
        }
        self.active_flag.store(false, Ordering::Relaxed);
        let Some(mut active) = active.take() else {
            return None;
        };

        let _ = writeln!(active.writer, "#");
        let _ = writeln!(active.writer, "# finish: {reason}");
        let _ = writeln!(active.writer, "# midi_lines: {}", active.lines_written);
        if trace_records_dropped > 0 {
            let _ = writeln!(
                active.writer,
                "# trace_records_dropped: {trace_records_dropped}"
            );
        }
        let _ = active.writer.flush();
        Some(active.path)
    }

    pub(crate) fn abort_and_remove(&self) -> Option<PathBuf> {
        self.active_flag.store(false, Ordering::Relaxed);
        let Ok(mut active) = self.active.lock() else {
            return None;
        };
        let active = active.take()?;
        let path = active.path.clone();
        drop(active);
        let _ = fs::remove_file(&path);
        Some(path)
    }
}

pub(crate) fn output_recording_midi_log_path(wav_path: &Path) -> PathBuf {
    wav_path.with_extension("midi.log")
}

pub(crate) fn write_midi_trace_log_header<W: Write>(
    writer: &mut W,
    request: &MidiTraceLogStart,
) -> io::Result<()> {
    let started_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    writeln!(writer, "# Mamut MIDI trace sidecar v1")?;
    writeln!(writer, "# started_unix_ms: {started_unix_ms}")?;
    writeln!(writer, "# wav_path: {}", request.wav_path.display())?;
    writeln!(writer, "# midi_log_path: {}", request.log_path.display())?;
    writeln!(writer, "# patch_name: {}", request.patch_name)?;
    writeln!(writer, "# patch_path: {}", request.patch_path.display())?;
    writeln!(writer, "# sample_rate_hz: {}", request.sample_rate_hz)?;
    writeln!(
        writer,
        "# max_frames: {}",
        request
            .max_frames
            .map(|frames| frames.to_string())
            .unwrap_or_else(|| "manual-stop".to_string())
    )?;
    writeln!(
        writer,
        "# midi_channel: {}",
        request
            .midi_channel
            .map(|channel| channel.to_string())
            .unwrap_or_else(|| "all".to_string())
    )?;
    writeln!(
        writer,
        "# controller_profile: {}",
        request
            .controller_profile
            .as_deref()
            .unwrap_or("pc4-legacy")
    )?;
    writeln!(
        writer,
        "# timing: t=seconds from MIDI input open; dt=milliseconds from previous traced MIDI event"
    )?;
    writeln!(writer, "#")?;
    Ok(())
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

#[derive(Debug, Clone, Copy)]
pub(crate) enum ParsedMidiMessage {
    Realtime(RealtimeMidiMessage),
    Runtime(RuntimeControlMessage),
    Reserved,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MidiTraceTiming {
    pub(crate) elapsed_seconds: f64,
    pub(crate) delta_millis: f64,
}

impl MidiTraceTiming {
    pub(crate) fn from_received_at(
        started_at: Instant,
        previous_trace_at: &mut Option<Instant>,
        received_at: Instant,
    ) -> Self {
        let elapsed_seconds = received_at
            .checked_duration_since(started_at)
            .unwrap_or_default()
            .as_secs_f64();
        let delta_millis = previous_trace_at
            .and_then(|previous| received_at.checked_duration_since(previous))
            .unwrap_or_default()
            .as_secs_f64()
            * 1000.0;
        *previous_trace_at = Some(received_at);
        Self {
            elapsed_seconds,
            delta_millis,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum SoundLabPage {
    Osc1 = 0,
    Osc2 = 1,
    Noise = 2,
    Spectral = 3,
    Relations = 4,
    BodyFilter = 5,
    Motion = 6,
    Performance = 7,
    Layers = 8,
}

impl SoundLabPage {
    pub(crate) const ALL: [Self; 9] = [
        Self::Osc1,
        Self::Osc2,
        Self::Noise,
        Self::Spectral,
        Self::Relations,
        Self::BodyFilter,
        Self::Motion,
        Self::Performance,
        Self::Layers,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Osc1 => "Osc 1",
            Self::Osc2 => "Osc 2",
            Self::Noise => "Noise",
            Self::Spectral => "Spectral",
            Self::Relations => "Relations",
            Self::BodyFilter => "Body/Filter",
            Self::Motion => "Motion",
            Self::Performance => "Performance",
            Self::Layers => "Layers",
        }
    }

    pub(crate) fn from_index(index: u8) -> Option<Self> {
        match index {
            0 => Some(Self::Osc1),
            1 => Some(Self::Osc2),
            2 => Some(Self::Noise),
            3 => Some(Self::Spectral),
            4 => Some(Self::Relations),
            5 => Some(Self::BodyFilter),
            6 => Some(Self::Motion),
            7 => Some(Self::Performance),
            8 => Some(Self::Layers),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SoundLabMidiSource {
    Knob(u8),
    Slider(u8),
    ModWheel,
}

impl SoundLabMidiSource {
    pub(crate) fn badge(self) -> String {
        match self {
            Self::Knob(index) => format!("K{index}"),
            Self::Slider(index) => format!("S{index}"),
            Self::ModWheel => "MW".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SoundLabMidiParamBinding {
    pub(crate) source: SoundLabMidiSource,
    pub(crate) id: ParamId,
}

pub(crate) const SOUND_LAB_OVERLAY_EVENT_CAPACITY: usize = 4;
pub(crate) const SOUND_LAB_MIDI_FOCUS_DISABLED: u8 = u8::MAX;

#[derive(Debug)]
pub(crate) struct SoundLabMidiFocus {
    page: AtomicU8,
}

impl Default for SoundLabMidiFocus {
    fn default() -> Self {
        Self {
            page: AtomicU8::new(SOUND_LAB_MIDI_FOCUS_DISABLED),
        }
    }
}

impl SoundLabMidiFocus {
    pub(crate) fn set(&self, page: Option<SoundLabPage>) {
        self.page.store(
            page.map(|page| page as u8)
                .unwrap_or(SOUND_LAB_MIDI_FOCUS_DISABLED),
            Ordering::Relaxed,
        );
    }

    pub(crate) fn page(&self) -> Option<SoundLabPage> {
        SoundLabPage::from_index(self.page.load(Ordering::Relaxed))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SoundLabOverlayEvents {
    pub(crate) page: SoundLabPage,
    pub(crate) source: SoundLabMidiSource,
    pub(crate) events: [Option<ControllerEvent>; SOUND_LAB_OVERLAY_EVENT_CAPACITY],
}

impl SoundLabOverlayEvents {
    pub(crate) fn empty(page: SoundLabPage, source: SoundLabMidiSource) -> Self {
        Self {
            page,
            source,
            events: [None; SOUND_LAB_OVERLAY_EVENT_CAPACITY],
        }
    }

    pub(crate) fn single(
        page: SoundLabPage,
        source: SoundLabMidiSource,
        event: ControllerEvent,
    ) -> Self {
        let mut events = Self::empty(page, source);
        events.events[0] = Some(event);
        events
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = ControllerEvent> + '_ {
        self.events.iter().filter_map(|event| *event)
    }

    pub(crate) fn trace_summary(&self) -> String {
        let targets = self
            .iter()
            .map(describe_controller_event_target)
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "sound lab overlay page={} source={} -> {}",
            self.page.label(),
            self.source.badge(),
            targets
        )
    }
}

pub(crate) fn describe_controller_event_target(event: ControllerEvent) -> String {
    match event {
        ControllerEvent::DirectParam { id, value } => {
            format!(
                "{}={}",
                param_spec(id).name,
                format_paramish_value(id, value)
            )
        }
        ControllerEvent::Macro { id, value } => {
            format!("macro {}={value:.3}", id.key())
        }
        ControllerEvent::GfmLayerAmount { amount } => format!("gfm={amount:.3}"),
        ControllerEvent::BcsLayerAmount { amount } => format!("bcs={amount:.3}"),
        ControllerEvent::BcsLayerEnabled { enabled } => format!("bcs_enable={enabled}"),
        ControllerEvent::ModWheel { amount } => format!("mod_wheel={amount:.3}"),
        ControllerEvent::Sustain { down } => format!("sustain={down}"),
        ControllerEvent::ChannelAftertouch { pressure } => format!("aftertouch={pressure:.3}"),
        ControllerEvent::PitchBend { semitones } => format!("bend={semitones:.3}"),
    }
}

pub(crate) fn format_paramish_value(id: ParamId, value: f32) -> String {
    match param_spec(id).unit {
        ParamUnit::Boolean => {
            if value >= 0.5 {
                "on".to_string()
            } else {
                "off".to_string()
            }
        }
        ParamUnit::Indexed => format!("{}", value.round() as i32),
        ParamUnit::Hertz => format!("{value:.1}Hz"),
        ParamUnit::Milliseconds => format!("{value:.1}ms"),
        ParamUnit::Decibels => format!("{value:.1}dB"),
        ParamUnit::Cents => format!("{value:.1}c"),
        ParamUnit::Semitones => format!("{value:.1}st"),
        ParamUnit::Normalized => format!("{value:.3}"),
    }
}

pub(crate) fn sound_lab_page_knob_bindings(
    page: SoundLabPage,
) -> &'static [SoundLabMidiParamBinding] {
    match page {
        SoundLabPage::Osc1 => &OSC1_KNOB_BINDINGS,
        SoundLabPage::Osc2 => &OSC2_KNOB_BINDINGS,
        SoundLabPage::Noise => &NOISE_KNOB_BINDINGS,
        SoundLabPage::Spectral => &SPECTRAL_KNOB_BINDINGS,
        SoundLabPage::Relations => &RELATIONS_KNOB_BINDINGS,
        SoundLabPage::BodyFilter => &BODY_FILTER_KNOB_BINDINGS,
        SoundLabPage::Motion => &MOTION_KNOB_BINDINGS,
        SoundLabPage::Performance => &PERFORMANCE_KNOB_BINDINGS,
        SoundLabPage::Layers => &[],
    }
}

pub(crate) fn sound_lab_page_slider_bindings(
    page: SoundLabPage,
) -> &'static [SoundLabMidiParamBinding] {
    match page {
        SoundLabPage::Osc1 => &OSC1_SLIDER_BINDINGS,
        SoundLabPage::Osc2 => &OSC2_SLIDER_BINDINGS,
        SoundLabPage::Noise => &[],
        SoundLabPage::Spectral => &[],
        SoundLabPage::Relations => &RELATIONS_SLIDER_BINDINGS,
        SoundLabPage::BodyFilter => &BODY_FILTER_SLIDER_BINDINGS,
        SoundLabPage::Motion => &MOTION_SLIDER_BINDINGS,
        SoundLabPage::Performance => &PERFORMANCE_SLIDER_BINDINGS,
        SoundLabPage::Layers => &[],
    }
}

pub(crate) fn sound_lab_page_mod_wheel_params(page: SoundLabPage) -> &'static [ParamId] {
    match page {
        SoundLabPage::Osc1 => &[
            ParamId::Osc1SawBend,
            ParamId::Osc1TriangleFold,
            ParamId::Osc1PulseEdge,
        ],
        SoundLabPage::Osc2 => &[ParamId::Osc2Level, ParamId::Osc2CrossmodAmount],
        SoundLabPage::Noise => &[ParamId::NoiseFilterLevel, ParamId::NoiseBodyLevel],
        SoundLabPage::Spectral => &[ParamId::SpectralPosition],
        SoundLabPage::Relations => &[
            ParamId::FmAmount,
            ParamId::PhaseModAmount,
            ParamId::RingModAmount,
            ParamId::CrossMixAmount,
        ],
        SoundLabPage::BodyFilter => &[ParamId::FilterCutoffHz],
        SoundLabPage::Motion => &[ParamId::FilterEnvDepth],
        SoundLabPage::Performance => &[ParamId::BloomMacro],
        SoundLabPage::Layers => &[],
    }
}

pub(crate) fn sound_lab_page_binding_for_source(
    page: SoundLabPage,
    source: SoundLabMidiSource,
) -> Option<SoundLabMidiParamBinding> {
    let bindings = match source {
        SoundLabMidiSource::Knob(_) => sound_lab_page_knob_bindings(page),
        SoundLabMidiSource::Slider(_) => sound_lab_page_slider_bindings(page),
        SoundLabMidiSource::ModWheel => return None,
    };
    bindings
        .iter()
        .copied()
        .find(|binding| binding.source == source)
}

#[cfg(test)]
pub(crate) fn sound_lab_badges_for_param(page: SoundLabPage, id: ParamId) -> Vec<String> {
    let mut badges = sound_lab_page_knob_bindings(page)
        .iter()
        .chain(sound_lab_page_slider_bindings(page).iter())
        .filter(|binding| binding.id == id)
        .map(|binding| binding.source.badge())
        .collect::<Vec<_>>();
    if sound_lab_page_mod_wheel_params(page).contains(&id) {
        badges.push(SoundLabMidiSource::ModWheel.badge());
    }
    badges
}

macro_rules! k {
    ($index:literal, $id:ident) => {
        SoundLabMidiParamBinding {
            source: SoundLabMidiSource::Knob($index),
            id: ParamId::$id,
        }
    };
}

macro_rules! s {
    ($index:literal, $id:ident) => {
        SoundLabMidiParamBinding {
            source: SoundLabMidiSource::Slider($index),
            id: ParamId::$id,
        }
    };
}

const OSC1_KNOB_BINDINGS: [SoundLabMidiParamBinding; 8] = [
    k!(1, Osc1SawLevel),
    k!(2, Osc1PulseLevel),
    k!(3, Osc1TriangleLevel),
    k!(4, Osc1NoiseLevel),
    k!(5, Osc1FineTuneCents),
    k!(6, Osc1PulseWidth),
    k!(7, Osc1PwmDepth),
    k!(9, Osc1PhaseMode),
];
const OSC1_SLIDER_BINDINGS: [SoundLabMidiParamBinding; 4] = [
    s!(1, Osc1StartPhase),
    s!(2, Osc1SawBend),
    s!(3, Osc1TriangleFold),
    s!(4, Osc1PulseEdge),
];
const OSC2_KNOB_BINDINGS: [SoundLabMidiParamBinding; 8] = [
    k!(1, Osc2SawLevel),
    k!(2, Osc2PulseLevel),
    k!(3, Osc2TriangleLevel),
    k!(4, Osc2IntervalSemitones),
    k!(5, Osc2FineTuneCents),
    k!(6, Osc2Level),
    k!(7, Osc2PitchMode),
    k!(9, Osc2Ratio),
];
const OSC2_SLIDER_BINDINGS: [SoundLabMidiParamBinding; 8] = [
    s!(1, Osc2PulseWidth),
    s!(2, Osc2PwmDepth),
    s!(3, Osc2PhaseMode),
    s!(4, Osc2StartPhase),
    s!(5, Osc2SawBend),
    s!(6, Osc2TriangleFold),
    s!(7, Osc2PulseEdge),
    s!(8, Osc2CrossmodAmount),
];
const NOISE_KNOB_BINDINGS: [SoundLabMidiParamBinding; 6] = [
    k!(1, NoiseColor),
    k!(2, NoiseFilterLevel),
    k!(3, NoiseBodyLevel),
    k!(4, AnalogDrift),
    k!(5, MicroJitter),
    k!(6, SourcePwmRateHz),
];
const SPECTRAL_KNOB_BINDINGS: [SoundLabMidiParamBinding; 6] = [
    k!(1, SpectralLevel),
    k!(2, SpectralTable),
    k!(3, SpectralPosition),
    k!(4, SpectralMorph),
    k!(5, SpectralRatio),
    k!(6, SpectralFineTuneCents),
];
const RELATIONS_KNOB_BINDINGS: [SoundLabMidiParamBinding; 8] = [
    k!(1, FmAmount),
    k!(2, FmDirection),
    k!(3, PhaseModAmount),
    k!(4, PhaseModDirection),
    k!(5, RingModAmount),
    k!(6, AmAmount),
    k!(7, SyncDirection),
    k!(9, SyncSoftness),
];
const RELATIONS_SLIDER_BINDINGS: [SoundLabMidiParamBinding; 6] = [
    s!(1, CrossMixMode),
    s!(2, CrossMixAmount),
    s!(3, Osc2SyncAmount),
    s!(4, Osc2CrossmodAmount),
    s!(5, Osc2Level),
    s!(6, Osc2Ratio),
];
const BODY_FILTER_KNOB_BINDINGS: [SoundLabMidiParamBinding; 8] = [
    k!(1, SubLevel),
    k!(2, SubOctaveOffset),
    k!(3, MixerBodyMix),
    k!(4, MixerPreFilterDrive),
    k!(5, FilterCutoffHz),
    k!(6, FilterResonance),
    k!(7, FilterDrive),
    k!(9, FilterKeytrack),
];
const BODY_FILTER_SLIDER_BINDINGS: [SoundLabMidiParamBinding; 4] = [
    s!(1, FinalStageBodyDrive),
    s!(2, FinalStageAsymmetry),
    s!(3, FinalStageLowMidEmphasis),
    s!(4, FinalStageOutputTrimDb),
];
const MOTION_KNOB_BINDINGS: [SoundLabMidiParamBinding; 8] = [
    k!(1, AmpEnvAttackMs),
    k!(2, AmpEnvDecayMs),
    k!(3, AmpEnvSustain),
    k!(4, AmpEnvReleaseMs),
    k!(5, FilterEnvAttackMs),
    k!(6, FilterEnvDecayMs),
    k!(7, FilterEnvSustain),
    k!(9, FilterEnvReleaseMs),
];
const MOTION_SLIDER_BINDINGS: [SoundLabMidiParamBinding; 8] = [
    s!(1, FilterEnvDepth),
    s!(2, ChorusEnabled),
    s!(3, ChorusMix),
    s!(4, ChorusDepth),
    s!(5, ChorusRateHz),
    s!(6, ReverbEnabled),
    s!(7, ReverbMix),
    s!(8, ReverbSize),
];
const PERFORMANCE_KNOB_BINDINGS: [SoundLabMidiParamBinding; 8] = [
    k!(1, VoiceVelocityToLevel),
    k!(2, VoiceVelocityToFilter),
    k!(3, PerformanceVelocityToLevel),
    k!(4, PerformanceVelocityToFilter),
    k!(5, PerformanceAftertouchToGravitacija),
    k!(6, PerformanceAftertouchToBaklja),
    k!(7, PerformanceModWheelToBloom),
    k!(9, PerformanceModWheelToSwarm),
];
const PERFORMANCE_SLIDER_BINDINGS: [SoundLabMidiParamBinding; 5] = [
    s!(1, GravitacijaMacro),
    s!(2, BloomMacro),
    s!(3, HeatMacro),
    s!(4, RuinMacro),
    s!(5, SwarmMacro),
];

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
    pub(crate) sample_rate_hz: u32,
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
    RequestPatch(mpsc::Sender<PatchFileV1>),
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
    BcsLayerGain,
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
    pub(crate) midi_messages_accepted: u64,
    pub(crate) midi_messages_dropped: u64,
    pub(crate) runtime_controls_dropped: u64,
    pub(crate) trace_records_dropped: u64,
    pub(crate) controllers_coalesced: u64,
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
    pub(crate) midi_messages_accepted: AtomicU64,
    pub(crate) midi_messages_dropped: AtomicU64,
    pub(crate) runtime_controls_dropped: AtomicU64,
    pub(crate) trace_records_dropped: AtomicU64,
    pub(crate) controllers_coalesced: AtomicU64,
    pub(crate) last_control: Mutex<Option<LastControlEvent>>,
}

impl InputMetrics {
    pub(crate) fn snapshot(&self) -> InputMetricsSnapshot {
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

    pub(crate) fn record_midi_message(&self) {
        self.midi_messages.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_midi_message_accepted(&self) {
        self.midi_messages_accepted.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_midi_message_dropped(&self) {
        self.midi_messages_dropped.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_runtime_control_dropped(&self) {
        self.runtime_controls_dropped
            .fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_trace_record_dropped(&self) {
        self.trace_records_dropped.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_controllers_coalesced(&self, count: usize) {
        if count > 0 {
            self.controllers_coalesced
                .fetch_add(count as u64, Ordering::Relaxed);
        }
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
    pub(crate) midi_trace_log: Arc<MidiTraceLog>,
    pub(crate) midi_trace_worker: MidiTraceWorker,
    pub(crate) sound_lab_midi_focus: Arc<SoundLabMidiFocus>,
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
