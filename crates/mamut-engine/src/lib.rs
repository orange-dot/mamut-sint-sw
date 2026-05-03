use std::sync::LazyLock;

use mamut_dsp::{
    AdsrEnvelope, AdsrTiming, DENORMAL_FLUSH_ABS, LinearSmoother, MASTER_SAFETY_KNEE, NoiseRng,
    Oscillator, SimpleChorus, SimpleReverb, StateVariableFilter, StereoBlockMut, StereoDcBlocker,
    db_to_gain, master_safety_limit, midi_note_hz, sanitize_sample, soft_clip,
};
pub use mamut_field::bcs::BcsScenario;
use mamut_field::{
    GfmDiagnostics, GfmExcitation, GfmLattice16, GfmPerformanceGesture, GfmPerformanceProgram,
    GfmProgramId,
    bcs::{BcsGesture, BcsParams, BcsVoice},
};
use mamut_identity::{
    DerivedState, IdentityState, MacroState, ResolvedIdentityFrame, resolve_identity,
};
use mamut_params::{MacroId, ParamId, param_spec};
use mamut_patch::{
    CrossMixMode, ModDirection, NoiseColor, Osc2PitchMode, OscPhaseMode, PatchFileV1,
    PatchValidationError, SpectralTable, validate_patch_v1,
};

mod api;
mod engine;
mod gfm_layer;
mod helpers;
mod state;

pub use api::{
    BcsLayerMode, BcsLayerSnapshot, ControllerEvent, DEFAULT_GFM_LAYER_SEED, EngineConfig,
    GfmLayerMode, GfmLayerSnapshot, GfmVoiceProgramSelection, NoteEvent, ProcessBlock, Scheduled,
    ScheduledControllerEvent, ScheduledNoteEvent,
};
pub use engine::Engine;
pub use gfm_layer::{
    GfmFieldVoice, GfmPatchVoiceConfig, GfmPatchVoiceDecision, select_gfm_program,
    select_gfm_program_for_patch,
};
pub use state::{
    DirectParameters, EngineSnapshot, OutputSafetySnapshot, PerformanceResponseSnapshot,
    VoicePhase, VoiceSnapshot,
};

pub(crate) use api::{
    BCS_ENGINE_LAYER_GAIN, BCS_LAYER_CONTROL_DEADZONE, BCS_LAYER_MIX_ATTACK_MS,
    BCS_LAYER_MIX_RELEASE_MS, GFM_LAYER_CONTROL_DEADZONE, GFM_LAYER_MIX_ATTACK_MS,
    GFM_LAYER_MIX_RELEASE_MS, GFM_LAYER_PRESSURE_ATTACK_MS, GFM_LAYER_PRESSURE_RELEASE_MS,
    MASTER_DC_BLOCKER_CUTOFF_HZ,
};
pub(crate) use gfm_layer::{
    gfm_momentary_layer_amount, normalize_bcs_layer_control, normalize_gfm_layer_control,
};
pub(crate) use helpers::*;
pub(crate) use state::{ControlState, RenderSmoothers, VoiceState};

#[cfg(test)]
mod tests;
