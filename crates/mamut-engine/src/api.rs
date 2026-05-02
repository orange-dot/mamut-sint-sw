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
pub(crate) const BCS_ENGINE_LAYER_GAIN: f32 = 0.24;
pub const DEFAULT_GFM_LAYER_SEED: u64 = 0x6A46_4D40;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GfmLayerMode {
    Disabled,
    Enabled { seed: u64 },
}

impl Default for GfmLayerMode {
    fn default() -> Self {
        Self::Disabled
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GfmLayerSnapshot {
    pub mode: GfmLayerMode,
    pub selection: GfmVoiceProgramSelection,
    pub active_program_id: Option<GfmProgramId>,
    pub diagnostics: Option<GfmDiagnostics>,
    pub amount: f32,
    pub pressure: f32,
    pub effective_amount: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BcsLayerMode {
    Disabled,
    Enabled { scenario: BcsScenario },
}

impl Default for BcsLayerMode {
    fn default() -> Self {
        Self::Disabled
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
            pitch_note: None,
            pitch_frequency_hz: None,
            max_state_abs: 0.0,
            unsafe_events: 0,
            unsafe_state: false,
        }
    }
}
