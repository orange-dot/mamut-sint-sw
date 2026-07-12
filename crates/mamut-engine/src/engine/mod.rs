use super::*;

#[derive(Debug, Clone)]
pub struct Engine {
    config: EngineConfig,
    pub(crate) patch: PatchFileV1,
    live_macros: MacroState,
    control: ControlState,
    voices: Vec<VoiceState>,
    age_counter: u64,
    last_frame: ResolvedIdentityFrame,
    last_direct: DirectParameters,
    render_smoothers: RenderSmoothers,
    control_smoothing_samples: usize,
    last_block_frames: usize,
    last_events_processed: usize,
    last_peak_output: f32,
    last_output_safety: OutputSafetySnapshot,
    final_body_left: f32,
    final_body_right: f32,
    chorus: SimpleChorus,
    reverb: SimpleReverb,
    master_dc_blocker: StereoDcBlocker,
    patch_switch_mute_frames: usize,
    gfm_layer_mode: GfmLayerMode,
    gfm_layer_auto_armed: bool,
    gfm_layer_auto_disarm_pending: bool,
    gfm_layer_mix: LinearSmoother,
    gfm_layer_pressure: LinearSmoother,
    gfm_layer_selection: GfmVoiceProgramSelection,
    gfm_layer_voice: Option<GfmFieldVoice>,
    gfm_note_strikes_enabled: bool,
    bcs_layer_mode: BcsLayerMode,
    bcs_layer_mix: LinearSmoother,
    bcs_layer_note: Option<u8>,
    bcs_layer_frequency_hz: Option<f32>,
    bcs_layer_voice: Option<BcsVoice>,
    mozaik_mode: MozaikMode,
    mozaik_controls: MozaikControlValues,
    mozaik_mix: LinearSmoother,
    mozaik_slope_sigma: LinearSmoother,
    mozaik_contrast_gamma: LinearSmoother,
    mozaik_phason: LinearSmoother,
    mozaik_drift: LinearSmoother,
    mozaik_slope_snapped: bool,
    mozaik_drift_accum: f64,
    mozaik_frame: MozaikFrame,
}

mod conversions;
mod direct_params;
mod events;
mod layers;
mod lifecycle;
mod macro_state;
mod mozaik;
mod process;
mod state_refresh;
use conversions::*;
pub(crate) use mozaik::{MozaikControlValues, MozaikFrame};
