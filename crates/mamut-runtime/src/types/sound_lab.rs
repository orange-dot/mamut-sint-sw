use super::*;

#[derive(Debug, Clone, Copy)]
pub enum RealtimeMidiMessage {
    Note(NoteEvent),
    Controller(ControllerEvent),
}

#[derive(Debug, Clone, Copy)]
pub enum ParsedMidiMessage {
    Realtime(RealtimeMidiMessage),
    Runtime(RuntimeControlMessage),
    Reserved,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MidiTraceTiming {
    pub elapsed_seconds: f64,
    pub delta_millis: f64,
}

impl MidiTraceTiming {
    pub fn from_received_at(
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
pub enum SoundLabPage {
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
    pub const ALL: [Self; 9] = [
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

    pub fn label(self) -> &'static str {
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

    pub fn from_index(index: u8) -> Option<Self> {
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
pub enum SoundLabMidiSource {
    Knob(u8),
    Slider(u8),
    Switch(u8),
    ModWheel,
}

impl SoundLabMidiSource {
    pub fn badge(self) -> String {
        match self {
            Self::Knob(index) => format!("K{index}"),
            Self::Slider(index) => format!("S{index}"),
            Self::Switch(index) => format!("SW{index}"),
            Self::ModWheel => "MW".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundLabMidiParamBinding {
    pub source: SoundLabMidiSource,
    pub id: ParamId,
}

pub const SOUND_LAB_OVERLAY_EVENT_CAPACITY: usize = 4;
pub const SOUND_LAB_MIDI_FOCUS_DISABLED: u8 = u8::MAX;

#[derive(Debug)]
pub struct SoundLabMidiFocus {
    page: AtomicU8,
    page_request: AtomicU8,
}

impl Default for SoundLabMidiFocus {
    fn default() -> Self {
        Self {
            page: AtomicU8::new(SOUND_LAB_MIDI_FOCUS_DISABLED),
            page_request: AtomicU8::new(SOUND_LAB_MIDI_FOCUS_DISABLED),
        }
    }
}

impl SoundLabMidiFocus {
    pub fn set(&self, page: Option<SoundLabPage>) {
        self.page.store(
            page.map(|page| page as u8)
                .unwrap_or(SOUND_LAB_MIDI_FOCUS_DISABLED),
            Ordering::Relaxed,
        );
    }

    pub fn page(&self) -> Option<SoundLabPage> {
        SoundLabPage::from_index(self.page.load(Ordering::Relaxed))
    }

    pub fn request_page(&self, page: SoundLabPage) {
        self.page_request.store(page as u8, Ordering::Relaxed);
    }

    pub fn take_page_request(&self) -> Option<SoundLabPage> {
        SoundLabPage::from_index(
            self.page_request
                .swap(SOUND_LAB_MIDI_FOCUS_DISABLED, Ordering::Relaxed),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoundLabOverlayEvents {
    pub page: SoundLabPage,
    pub source: SoundLabMidiSource,
    pub page_select: Option<SoundLabPage>,
    pub events: [Option<ControllerEvent>; SOUND_LAB_OVERLAY_EVENT_CAPACITY],
}

impl SoundLabOverlayEvents {
    pub fn empty(page: SoundLabPage, source: SoundLabMidiSource) -> Self {
        Self {
            page,
            source,
            page_select: None,
            events: [None; SOUND_LAB_OVERLAY_EVENT_CAPACITY],
        }
    }

    pub fn single(page: SoundLabPage, source: SoundLabMidiSource, event: ControllerEvent) -> Self {
        let mut events = Self::empty(page, source);
        events.events[0] = Some(event);
        events
    }

    pub fn select_page(current_page: SoundLabPage, switch_index: u8, page: SoundLabPage) -> Self {
        let mut events = Self::empty(current_page, SoundLabMidiSource::Switch(switch_index));
        events.page_select = Some(page);
        events
    }

    pub fn iter(&self) -> impl Iterator<Item = ControllerEvent> + '_ {
        self.events.iter().filter_map(|event| *event)
    }

    pub fn trace_summary(&self) -> String {
        if let Some(page) = self.page_select {
            return format!(
                "sound lab page source={} -> {}",
                self.source.badge(),
                page.label()
            );
        }

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

pub fn describe_controller_event_target(event: ControllerEvent) -> String {
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

pub fn format_paramish_value(id: ParamId, value: f32) -> String {
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

pub fn sound_lab_page_knob_bindings(page: SoundLabPage) -> &'static [SoundLabMidiParamBinding] {
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

pub fn sound_lab_page_slider_bindings(page: SoundLabPage) -> &'static [SoundLabMidiParamBinding] {
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

pub fn sound_lab_page_mod_wheel_params(page: SoundLabPage) -> &'static [ParamId] {
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

pub fn sound_lab_page_binding_for_source(
    page: SoundLabPage,
    source: SoundLabMidiSource,
) -> Option<SoundLabMidiParamBinding> {
    let bindings = match source {
        SoundLabMidiSource::Knob(_) => sound_lab_page_knob_bindings(page),
        SoundLabMidiSource::Slider(_) => sound_lab_page_slider_bindings(page),
        SoundLabMidiSource::Switch(_) | SoundLabMidiSource::ModWheel => return None,
    };
    bindings
        .iter()
        .copied()
        .find(|binding| binding.source == source)
}

pub fn sound_lab_badges_for_param(page: SoundLabPage, id: ParamId) -> Vec<String> {
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
const OSC1_SLIDER_BINDINGS: [SoundLabMidiParamBinding; 5] = [
    s!(1, Osc1StartPhase),
    s!(2, Osc1SawBend),
    s!(3, Osc1TriangleFold),
    s!(4, Osc1PulseEdge),
    s!(5, Osc1Bandlimit),
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
    s!(8, Osc2Bandlimit),
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
const BODY_FILTER_SLIDER_BINDINGS: [SoundLabMidiParamBinding; 5] = [
    s!(1, FinalStageBodyDrive),
    s!(2, FinalStageAsymmetry),
    s!(3, FinalStageLowMidEmphasis),
    s!(4, FinalStageOutputTrimDb),
    s!(5, FilterModel),
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
