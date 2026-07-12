use super::*;

pub(crate) const MASTER_DC_BLOCKER_CUTOFF_HZ: f32 = 5.0;
pub(crate) const GFM_LAYER_CONTROL_DEADZONE: f32 = 0.01;
pub(crate) const GFM_LAYER_MIX_ATTACK_MS: f32 = 110.0;
pub(crate) const GFM_LAYER_MIX_RELEASE_MS: f32 = 85.0;
pub(crate) const GFM_LAYER_PRESSURE_ATTACK_MS: f32 = 70.0;
pub(crate) const GFM_LAYER_PRESSURE_RELEASE_MS: f32 = 90.0;
pub(crate) const BCS_LAYER_CONTROL_DEADZONE: f32 = 0.01;
pub(crate) const BCS_LAYER_MIX_ATTACK_MS: f32 = 45.0;
pub(crate) const BCS_LAYER_MIX_RELEASE_MS: f32 = 100.0;
pub(crate) const BCS_ENGINE_LAYER_GAIN: f32 = 1.0;
pub const DEFAULT_GFM_LAYER_SEED: u64 = 0x6A46_4D40;
pub(crate) const MOZAIK_MIX_ATTACK_MS: f32 = 60.0;
pub(crate) const MOZAIK_MIX_RELEASE_MS: f32 = 90.0;
pub(crate) const MOZAIK_CONTROL_SMOOTHING_MS: f32 = 30.0;
pub const DEFAULT_MOZAIK_SEED: u64 = 0x4D6F_7A31;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EngineConfig {
    pub sample_rate_hz: f32,
    pub max_block_frames: usize,
    pub voice_count: usize,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            sample_rate_hz: 48_000.0,
            max_block_frames: 256,
            voice_count: 6,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NoteEvent {
    NoteOn { note: u8, velocity: f32 },
    NoteOff { note: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ControllerEvent {
    PitchBend { semitones: f32 },
    ModWheel { amount: f32 },
    Sustain { down: bool },
    ChannelAftertouch { pressure: f32 },
    GfmLayerAmount { amount: f32 },
    BcsLayerEnabled { enabled: bool },
    BcsLayerAmount { amount: f32 },
    Macro { id: MacroId, value: f32 },
    DirectParam { id: ParamId, value: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scheduled<T> {
    pub frame_offset: usize,
    pub event: T,
}

pub type ScheduledNoteEvent = Scheduled<NoteEvent>;
pub type ScheduledControllerEvent = Scheduled<ControllerEvent>;

#[derive(Debug)]
pub struct ProcessBlock<'a> {
    pub frame_count: usize,
    /// Scheduled note events must be sorted by nondecreasing `frame_offset`.
    pub note_events: &'a [ScheduledNoteEvent],
    /// Scheduled controller events must be sorted by nondecreasing `frame_offset`.
    pub controller_events: &'a [ScheduledControllerEvent],
    pub macro_state: Option<MacroState>,
    /// Audio beyond the available output capacity is discarded, but control and voice state still
    /// advance across `frame_count`.
    pub output: Option<StereoBlockMut<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GfmVoiceProgramSelection {
    pub program_id: Option<GfmProgramId>,
    pub horizont_score: f32,
    pub pec_score: f32,
    pub baklja_score: f32,
}

impl Default for GfmVoiceProgramSelection {
    fn default() -> Self {
        Self {
            program_id: None,
            horizont_score: 0.0,
            pec_score: 0.0,
            baklja_score: 0.0,
        }
    }
}

impl GfmVoiceProgramSelection {
    pub(crate) const MIN_PROGRAM_SCORE: f32 = 0.15;

    pub fn best_score(self) -> f32 {
        self.horizont_score
            .max(self.pec_score)
            .max(self.baklja_score)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum GfmLayerMode {
    #[default]
    Disabled,
    Enabled {
        seed: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GfmLayerSnapshot {
    pub mode: GfmLayerMode,
    pub selection: GfmVoiceProgramSelection,
    pub active_program_id: Option<GfmProgramId>,
    pub controls: Option<GfmPerformanceControls>,
    pub diagnostics: Option<GfmDiagnostics>,
    pub amount: f32,
    pub pressure: f32,
    pub effective_amount: f32,
    pub note_strikes_enabled: bool,
    /// Present while a GFM voice is armed. Fixed-size (16x16 quantized
    /// planes); copying the snapshot allocates nothing.
    pub terrain: Option<GfmTerrainSnapshot16>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BcsLayerMode {
    #[default]
    Disabled,
    Enabled {
        scenario: BcsScenario,
    },
}

/// Session-only Mozaik voice-source mode (`SET5-4`). The seed offsets the
/// per-voice initial phason; it never touches the patch schema.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MozaikMode {
    #[default]
    Disabled,
    Enabled {
        seed: u64,
    },
}

/// The five Mozaik session controls (plus the on/off mode). All take a
/// normalized `[0, 1]` value; the engine maps slope to `sigma in
/// [0.45, 0.75]` (with detent snap) and contrast to `gamma in [1.0, 2.2]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MozaikParam {
    Mix,
    Slope,
    Contrast,
    Phason,
    Drift,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MozaikSnapshot {
    pub mode: MozaikMode,
    /// Control-domain mix target `[0, 1]`.
    pub mix: f32,
    /// Smoothed mix currently applied in the render loop.
    pub effective_mix: f32,
    /// Control-domain slope `[0, 1]`.
    pub slope: f32,
    /// Mapped (and possibly detent-snapped) slope target `sigma`.
    pub slope_sigma: f32,
    pub slope_snapped: bool,
    /// Control-domain contrast `[0, 1]`.
    pub contrast: f32,
    /// Mapped contrast target `gamma`.
    pub contrast_gamma: f32,
    /// Control-domain phason offset `[0, 1]`.
    pub phason: f32,
    /// Control-domain drift `[0, 1]` (`0` = frozen).
    pub drift: f32,
    /// Accumulated auto-phason offset from drift, wrapped to `[0, 1)`.
    pub drift_phason: f32,
}

impl Default for MozaikSnapshot {
    fn default() -> Self {
        Self {
            mode: MozaikMode::Disabled,
            mix: 0.0,
            effective_mix: 0.0,
            slope: 0.0,
            slope_sigma: 0.0,
            slope_snapped: false,
            contrast: 0.0,
            contrast_gamma: 0.0,
            phason: 0.0,
            drift: 0.0,
            drift_phason: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BcsLayerSnapshot {
    pub mode: BcsLayerMode,
    pub active_scenario: Option<BcsScenario>,
    pub sample_rate_hz: Option<u32>,
    pub enabled: bool,
    pub amount: f32,
    pub effective_amount: f32,
    pub gain: f32,
    pub effective_gain: f32,
    pub pitch_note: Option<u8>,
    pub pitch_frequency_hz: Option<f32>,
    pub max_state_abs: f32,
    pub unsafe_events: u64,
    pub unsafe_state: bool,
}

impl Default for BcsLayerSnapshot {
    fn default() -> Self {
        Self {
            mode: BcsLayerMode::Disabled,
            active_scenario: None,
            sample_rate_hz: None,
            enabled: false,
            amount: 0.0,
            effective_amount: 0.0,
            gain: 0.0,
            effective_gain: 0.0,
            pitch_note: None,
            pitch_frequency_hz: None,
            max_state_abs: 0.0,
            unsafe_events: 0,
            unsafe_state: false,
        }
    }
}
