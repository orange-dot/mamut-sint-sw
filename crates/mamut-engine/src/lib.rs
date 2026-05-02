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
use mamut_patch::{PatchFileV1, PatchValidationError, validate_patch_v1};

const MASTER_DC_BLOCKER_CUTOFF_HZ: f32 = 5.0;
const GFM_LAYER_CONTROL_DEADZONE: f32 = 0.01;
const GFM_LAYER_MIX_ATTACK_MS: f32 = 110.0;
const GFM_LAYER_MIX_RELEASE_MS: f32 = 85.0;
const GFM_LAYER_PRESSURE_ATTACK_MS: f32 = 70.0;
const GFM_LAYER_PRESSURE_RELEASE_MS: f32 = 90.0;
const BCS_LAYER_CONTROL_DEADZONE: f32 = 0.01;
const BCS_LAYER_MIX_ATTACK_MS: f32 = 45.0;
const BCS_LAYER_MIX_RELEASE_MS: f32 = 100.0;
const BCS_ENGINE_LAYER_GAIN: f32 = 0.24;
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
    const MIN_PROGRAM_SCORE: f32 = 0.15;

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

pub fn select_gfm_program(frame: ResolvedIdentityFrame) -> GfmVoiceProgramSelection {
    let identity = frame.identity;
    let derived = frame.derived;
    let shaped = frame.shaped_macros;

    let horizont_score = sanitize_gfm_selection_score(
        identity.horizont_open * 0.40
            + identity.horizont_air * 0.34
            + identity.horizont_span * 0.18
            + shaped.bloom * 0.08,
    );
    let pec_score = sanitize_gfm_selection_score(
        identity.pec_mass * 0.30
            + identity.pec_heat * 0.32
            + identity.pec_pressure * 0.20
            + derived.mass * 0.12
            + shaped.heat * 0.06,
    );
    let baklja_score = sanitize_gfm_selection_score(
        identity.baklja_ready * 0.32
            + identity.baklja_edge * 0.34
            + identity.baklja_sync_bias * 0.16
            + derived.rupture_response * 0.12
            + shaped.ruin * 0.06,
    );

    let best_score = horizont_score.max(pec_score).max(baklja_score);
    let program_id = if best_score < GfmVoiceProgramSelection::MIN_PROGRAM_SCORE {
        None
    } else if baklja_score >= pec_score && baklja_score >= horizont_score {
        Some(GfmProgramId::BakljaPerformance)
    } else if pec_score >= horizont_score {
        Some(GfmProgramId::PecPerformance)
    } else {
        Some(GfmProgramId::HorizontPerformance)
    };

    GfmVoiceProgramSelection {
        program_id,
        horizont_score,
        pec_score,
        baklja_score,
    }
}

pub fn select_gfm_program_for_patch(patch: &PatchFileV1) -> GfmVoiceProgramSelection {
    let macros = MacroState::from_defaults(&patch.macros);
    select_gfm_program(resolve_identity(patch, &macros))
}

fn sanitize_gfm_selection_score(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GfmPatchVoiceConfig {
    pub seed: u64,
    pub sample_rate_hz: u32,
}

impl GfmPatchVoiceConfig {
    pub const fn new(seed: u64, sample_rate_hz: u32) -> Self {
        Self {
            seed,
            sample_rate_hz,
        }
    }

    const fn normalized(self) -> Self {
        Self {
            seed: self.seed,
            sample_rate_hz: if self.sample_rate_hz == 0 {
                1
            } else {
                self.sample_rate_hz
            },
        }
    }
}

impl Default for GfmPatchVoiceConfig {
    fn default() -> Self {
        Self {
            seed: mamut_field::GFM_PERFORMANCE_BASELINE_SEED,
            sample_rate_hz: 48_000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GfmPatchVoiceDecision {
    config: GfmPatchVoiceConfig,
    selection: GfmVoiceProgramSelection,
    voice: Option<GfmFieldVoice>,
}

impl GfmPatchVoiceDecision {
    pub const fn config(&self) -> GfmPatchVoiceConfig {
        self.config
    }

    pub const fn selection(&self) -> GfmVoiceProgramSelection {
        self.selection
    }

    pub const fn is_enabled(&self) -> bool {
        self.voice.is_some()
    }

    pub fn voice(&self) -> Option<&GfmFieldVoice> {
        self.voice.as_ref()
    }

    pub fn voice_mut(&mut self) -> Option<&mut GfmFieldVoice> {
        self.voice.as_mut()
    }

    pub fn into_voice(self) -> Option<GfmFieldVoice> {
        self.voice
    }
}

#[derive(Debug, Clone)]
pub struct GfmFieldVoice {
    program: GfmPerformanceProgram,
    gesture: GfmPerformanceGesture,
    lattice: GfmLattice16,
    sample_rate_hz: u32,
    frame_index: usize,
}

impl GfmFieldVoice {
    pub fn new(program_id: GfmProgramId, seed: u64, sample_rate_hz: u32) -> Self {
        let sample_rate_hz = sample_rate_hz.max(1);
        let program = GfmPerformanceProgram::new(program_id, sample_rate_hz as f32);
        Self {
            program,
            gesture: GfmPerformanceGesture::v0_4(),
            lattice: GfmLattice16::new(seed, program.params()),
            sample_rate_hz,
            frame_index: 0,
        }
    }

    pub fn from_patch(patch: &PatchFileV1, config: GfmPatchVoiceConfig) -> GfmPatchVoiceDecision {
        let config = config.normalized();
        let selection = select_gfm_program_for_patch(patch);
        let voice = selection
            .program_id
            .map(|program_id| Self::new(program_id, config.seed, config.sample_rate_hz));

        GfmPatchVoiceDecision {
            config,
            selection,
            voice,
        }
    }

    pub const fn program_id(&self) -> GfmProgramId {
        self.program.id()
    }

    pub const fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    pub const fn frame_index(&self) -> usize {
        self.frame_index
    }

    pub fn frames(&self) -> usize {
        self.gesture.frames(self.sample_rate_hz)
    }

    pub fn probe_position(&self) -> (usize, usize) {
        self.lattice.probe_position()
    }

    pub fn diagnostics(&self) -> GfmDiagnostics {
        self.lattice.diagnostics()
    }

    pub fn next_sample(&mut self) -> f32 {
        self.next_sample_with_live_pressure(0.0)
    }

    pub fn next_sample_with_live_pressure(&mut self, pressure: f32) -> f32 {
        let excitation = self
            .gesture
            .excitation_at_frame(self.frame_index, self.sample_rate_hz);
        let excitation = gfm_live_pressure_excitation(excitation, pressure);
        self.frame_index = self.frame_index.wrapping_add(1);
        self.lattice.next_sample_with_excitation(excitation)
    }

    pub fn render_mono_block(&mut self, output: &mut [f32]) {
        for sample in output {
            *sample = self.next_sample();
        }
    }

    pub fn render_stereo_block(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (left, right) in left.iter_mut().zip(right.iter_mut()) {
            let sample = self.next_sample();
            *left = sample;
            *right = sample;
        }
    }
}

fn gfm_live_pressure_excitation(base: GfmExcitation, pressure: f32) -> GfmExcitation {
    let pressure = normalize_gfm_layer_control(pressure);
    if pressure <= f32::EPSILON {
        return base;
    }

    GfmExcitation {
        pressure: (base.pressure + pressure * 0.22).clamp(0.0, 1.0),
        heat: (base.heat + pressure * 0.10).clamp(0.0, 1.0),
        rupture_bias: (base.rupture_bias + pressure * 0.16).clamp(0.0, 1.0),
    }
}

fn gfm_momentary_layer_amount(amount: f32, pressure: f32) -> f32 {
    let amount = normalize_gfm_layer_control(amount);
    let pressure = normalize_gfm_layer_control(pressure);
    if amount <= f32::EPSILON || pressure <= f32::EPSILON {
        return 0.0;
    }
    let gate = expressive_amount(pressure, 0.70);
    (amount * gate).clamp(0.0, 1.0)
}

fn normalize_gfm_layer_control(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    if value <= GFM_LAYER_CONTROL_DEADZONE {
        0.0
    } else {
        value
    }
}

fn normalize_bcs_layer_control(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    if value <= BCS_LAYER_CONTROL_DEADZONE {
        0.0
    } else {
        value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoicePhase {
    Idle,
    Held,
    Released,
    SustainedReleased,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoiceSnapshot {
    pub slot: usize,
    pub note: Option<u8>,
    pub velocity: f32,
    pub phase: VoicePhase,
    pub age: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectParameters {
    pub osc1_wave_mix: [f32; 4],
    pub osc2_wave_mix: [f32; 3],
    pub sub_level: f32,
    pub mixer_pre_filter_drive: f32,
    pub mixer_body_mix: f32,
    pub osc2_interval_semitones: f32,
    pub sync_amount: f32,
    pub crossmod_amount: f32,
    pub detune_spread_cents: f32,
    pub cutoff_hz: f32,
    pub resonance: f32,
    pub filter_drive: f32,
    pub filter_env_depth: f32,
    pub filter_tracking: f32,
    pub amp_env: AdsrTiming,
    pub filter_env: AdsrTiming,
    pub voice_level: f32,
    pub body_drive: f32,
    pub output_trim_db: f32,
    pub stereo_width: f32,
    pub stereo_crossfeed: f32,
    pub final_saturation: f32,
    pub final_asymmetry: f32,
    pub low_mid_emphasis: f32,
    pub chorus_enabled: bool,
    pub chorus_mix: f32,
    pub chorus_depth: f32,
    pub chorus_rate_hz: f32,
    pub reverb_enabled: bool,
    pub reverb_mix: f32,
    pub reverb_size: f32,
    pub reverb_damping: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EngineSnapshot {
    pub patch_name: String,
    pub patch_description: Option<String>,
    pub patch_tags: Vec<String>,
    pub patch_favorite: bool,
    pub sample_rate_hz: f32,
    pub last_block_frames: usize,
    pub events_processed: usize,
    pub active_voice_count: usize,
    pub sustain_down: bool,
    pub live_macros: MacroState,
    pub effective_macros: MacroState,
    pub identity: IdentityState,
    pub derived: DerivedState,
    pub direct: DirectParameters,
    pub gfm_layer: GfmLayerSnapshot,
    pub bcs_layer: BcsLayerSnapshot,
    pub voices: Vec<VoiceSnapshot>,
    pub held_notes: Vec<u8>,
    pub output_safety: OutputSafetySnapshot,
    pub peak_output: f32,
    pub clip_detected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct OutputSafetySnapshot {
    pub pre_safety_peak: f32,
    pub post_safety_peak: f32,
    pub safety_limiter_hits: u64,
    pub max_safety_reduction: f32,
    pub tiny_flush_events: u64,
}

impl OutputSafetySnapshot {
    fn observe_channel(&mut self, pre_safety: f32, post_limiter: f32) {
        if pre_safety.is_finite()
            && pre_safety.abs() > MASTER_SAFETY_KNEE
            && (pre_safety - post_limiter).abs() > f32::EPSILON
        {
            self.safety_limiter_hits += 1;
            self.max_safety_reduction = self
                .max_safety_reduction
                .max((pre_safety.abs() - post_limiter.abs()).max(0.0));
        }

        if pre_safety.is_finite()
            && pre_safety != 0.0
            && pre_safety.abs() < DENORMAL_FLUSH_ABS
            && post_limiter == 0.0
        {
            self.tiny_flush_events += 1;
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ControlState {
    pitch_bend_semitones: f32,
    mod_wheel: f32,
    sustain_down: bool,
    aftertouch: f32,
    gfm_layer_pressure: f32,
    gfm_layer_amount: f32,
    bcs_layer_enabled: bool,
    bcs_layer_amount: f32,
}

impl Default for ControlState {
    fn default() -> Self {
        Self {
            pitch_bend_semitones: 0.0,
            mod_wheel: 0.0,
            sustain_down: false,
            aftertouch: 0.0,
            gfm_layer_pressure: 0.0,
            gfm_layer_amount: 0.0,
            bcs_layer_enabled: false,
            bcs_layer_amount: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
struct RenderSmoothers {
    sync_amount: LinearSmoother,
    crossmod_amount: LinearSmoother,
    cutoff_hz: LinearSmoother,
    resonance: LinearSmoother,
    filter_drive: LinearSmoother,
    voice_level: LinearSmoother,
    body_drive: LinearSmoother,
    stereo_width: LinearSmoother,
    stereo_crossfeed: LinearSmoother,
    final_saturation: LinearSmoother,
    final_asymmetry: LinearSmoother,
    low_mid_emphasis: LinearSmoother,
}

impl RenderSmoothers {
    fn new(direct: DirectParameters) -> Self {
        Self {
            sync_amount: LinearSmoother::new(direct.sync_amount),
            crossmod_amount: LinearSmoother::new(direct.crossmod_amount),
            cutoff_hz: LinearSmoother::new(direct.cutoff_hz),
            resonance: LinearSmoother::new(direct.resonance),
            filter_drive: LinearSmoother::new(direct.filter_drive),
            voice_level: LinearSmoother::new(direct.voice_level),
            body_drive: LinearSmoother::new(direct.body_drive),
            stereo_width: LinearSmoother::new(direct.stereo_width),
            stereo_crossfeed: LinearSmoother::new(direct.stereo_crossfeed),
            final_saturation: LinearSmoother::new(direct.final_saturation),
            final_asymmetry: LinearSmoother::new(direct.final_asymmetry),
            low_mid_emphasis: LinearSmoother::new(direct.low_mid_emphasis),
        }
    }

    fn set_targets(&mut self, direct: DirectParameters, sample_count: usize) {
        self.sync_amount
            .set_target(direct.sync_amount, sample_count);
        self.crossmod_amount
            .set_target(direct.crossmod_amount, sample_count);
        self.cutoff_hz.set_target(direct.cutoff_hz, sample_count);
        self.resonance.set_target(direct.resonance, sample_count);
        self.filter_drive
            .set_target(direct.filter_drive, sample_count);
        self.voice_level
            .set_target(direct.voice_level, sample_count);
        self.body_drive.set_target(direct.body_drive, sample_count);
        self.stereo_width
            .set_target(direct.stereo_width, sample_count);
        self.stereo_crossfeed
            .set_target(direct.stereo_crossfeed, sample_count);
        self.final_saturation
            .set_target(direct.final_saturation, sample_count);
        self.final_asymmetry
            .set_target(direct.final_asymmetry, sample_count);
        self.low_mid_emphasis
            .set_target(direct.low_mid_emphasis, sample_count);
    }

    fn next_direct(&mut self, mut direct: DirectParameters) -> DirectParameters {
        direct.sync_amount = self.sync_amount.next_value();
        direct.crossmod_amount = self.crossmod_amount.next_value();
        direct.cutoff_hz = self.cutoff_hz.next_value();
        direct.resonance = self.resonance.next_value();
        direct.filter_drive = self.filter_drive.next_value();
        direct.voice_level = self.voice_level.next_value();
        direct.body_drive = self.body_drive.next_value();
        direct.stereo_width = self.stereo_width.next_value();
        direct.stereo_crossfeed = self.stereo_crossfeed.next_value();
        direct.final_saturation = self.final_saturation.next_value();
        direct.final_asymmetry = self.final_asymmetry.next_value();
        direct.low_mid_emphasis = self.low_mid_emphasis.next_value();
        direct
    }
}

#[derive(Debug, Clone)]
struct VoiceState {
    note: Option<u8>,
    velocity: f32,
    phase: VoicePhase,
    age: u64,
    osc1: Oscillator,
    osc2: Oscillator,
    sub: Oscillator,
    noise: NoiseRng,
    filter: StateVariableFilter,
    amp_env: AdsrEnvelope,
    filter_env: AdsrEnvelope,
}

impl VoiceState {
    fn idle(sample_rate_hz: f32, slot: usize) -> Self {
        let seed = (0x1234_5678_u32).wrapping_add((slot as u32).wrapping_mul(0x9E37_79B9));
        Self {
            note: None,
            velocity: 0.0,
            phase: VoicePhase::Idle,
            age: 0,
            osc1: Oscillator::new(),
            osc2: Oscillator::new(),
            sub: Oscillator::new(),
            noise: NoiseRng::new(seed),
            filter: StateVariableFilter::new(),
            amp_env: AdsrEnvelope::new(
                sample_rate_hz,
                AdsrTiming {
                    attack_ms: 10.0,
                    decay_ms: 50.0,
                    sustain: 1.0,
                    release_ms: 100.0,
                },
            ),
            filter_env: AdsrEnvelope::new(
                sample_rate_hz,
                AdsrTiming {
                    attack_ms: 10.0,
                    decay_ms: 50.0,
                    sustain: 1.0,
                    release_ms: 100.0,
                },
            ),
        }
    }

    fn trigger(
        &mut self,
        note: u8,
        velocity: f32,
        age: u64,
        amp_env: AdsrTiming,
        filter_env: AdsrTiming,
        slot: usize,
    ) {
        self.note = Some(note);
        self.velocity = velocity.clamp(0.0, 1.0);
        self.phase = VoicePhase::Held;
        self.age = age;
        self.filter.reset();
        self.amp_env.set_timing(amp_env);
        self.filter_env.set_timing(filter_env);
        self.amp_env.note_on();
        self.filter_env.note_on();
        self.osc1
            .set_phase((((slot as f32) * 0.137) + note as f32 * 0.017).fract());
        self.osc2
            .set_phase((((slot as f32) * 0.223) + note as f32 * 0.031).fract());
        self.sub
            .set_phase((((slot as f32) * 0.089) + note as f32 * 0.013).fract());
    }

    fn start_release(&mut self) {
        if self.phase != VoicePhase::Idle {
            self.phase = VoicePhase::Released;
            self.amp_env.note_off();
            self.filter_env.note_off();
        }
    }
}

#[derive(Debug, Clone)]
pub struct Engine {
    config: EngineConfig,
    patch: PatchFileV1,
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
    bcs_layer_mode: BcsLayerMode,
    bcs_layer_mix: LinearSmoother,
    bcs_layer_note: Option<u8>,
    bcs_layer_frequency_hz: Option<f32>,
    bcs_layer_voice: Option<BcsVoice>,
}

impl Engine {
    pub fn new(config: EngineConfig, patch: PatchFileV1) -> Result<Self, PatchValidationError> {
        validate_patch_v1(&patch)?;
        let config = EngineConfig {
            sample_rate_hz: config.sample_rate_hz.max(8_000.0),
            max_block_frames: config.max_block_frames.max(1),
            voice_count: config.voice_count.max(1),
        };
        let live_macros = MacroState::from_defaults(&patch.macros);
        let last_frame = resolve_identity(&patch, &live_macros);
        let last_direct = resolve_direct_parameters(&patch, last_frame, ControlState::default());
        let render_smoothers = RenderSmoothers::new(last_direct);
        let mut engine = Self {
            config,
            patch,
            live_macros,
            control: ControlState::default(),
            voices: (0..config.voice_count)
                .map(|slot| VoiceState::idle(config.sample_rate_hz, slot))
                .collect(),
            age_counter: 0,
            last_frame,
            last_direct,
            render_smoothers,
            control_smoothing_samples: control_smoothing_samples(config.sample_rate_hz),
            last_block_frames: 0,
            last_events_processed: 0,
            last_peak_output: 0.0,
            last_output_safety: OutputSafetySnapshot::default(),
            final_body_left: 0.0,
            final_body_right: 0.0,
            chorus: SimpleChorus::new(config.sample_rate_hz),
            reverb: SimpleReverb::new(config.sample_rate_hz),
            master_dc_blocker: StereoDcBlocker::new(
                config.sample_rate_hz,
                MASTER_DC_BLOCKER_CUTOFF_HZ,
            ),
            patch_switch_mute_frames: 0,
            gfm_layer_mode: GfmLayerMode::Disabled,
            gfm_layer_auto_armed: false,
            gfm_layer_auto_disarm_pending: false,
            gfm_layer_mix: LinearSmoother::new(0.0),
            gfm_layer_pressure: LinearSmoother::new(0.0),
            gfm_layer_selection: GfmVoiceProgramSelection::default(),
            gfm_layer_voice: None,
            bcs_layer_mode: BcsLayerMode::Disabled,
            bcs_layer_mix: LinearSmoother::new(0.0),
            bcs_layer_note: None,
            bcs_layer_frequency_hz: None,
            bcs_layer_voice: None,
        };
        engine.refresh_resolved_state();
        Ok(engine)
    }

    pub fn load_patch(&mut self, patch: PatchFileV1) -> Result<(), PatchValidationError> {
        validate_patch_v1(&patch)?;
        self.patch = patch;
        self.live_macros = MacroState::from_defaults(&self.patch.macros);
        self.reset_runtime_state();
        self.patch_switch_mute_frames = patch_switch_mute_frames(self.config.sample_rate_hz);
        self.last_frame = resolve_identity(&self.patch, &self.live_macros);
        self.last_direct = resolve_direct_parameters(&self.patch, self.last_frame, self.control);
        self.render_smoothers = RenderSmoothers::new(self.last_direct);
        self.rebuild_gfm_layer();
        self.rebuild_bcs_layer();
        Ok(())
    }

    pub fn panic(&mut self) {
        self.live_macros = MacroState::from_defaults(&self.patch.macros);
        self.reset_runtime_state();
        self.patch_switch_mute_frames = 0;
        self.refresh_resolved_state();
        self.rebuild_gfm_layer();
        self.rebuild_bcs_layer();
    }

    pub fn reset_controllers(&mut self) {
        let sustain_was_down = self.control.sustain_down;
        self.control.pitch_bend_semitones = 0.0;
        self.control.mod_wheel = 0.0;
        self.control.aftertouch = 0.0;
        self.control.gfm_layer_pressure = 0.0;
        self.control.gfm_layer_amount = 0.0;
        self.control.bcs_layer_enabled = false;
        self.control.bcs_layer_amount = 0.0;
        self.control.sustain_down = false;
        self.live_macros = MacroState::from_defaults(&self.patch.macros);
        self.force_disable_auto_armed_gfm_layer();
        self.reset_gfm_layer_smoothing();
        self.reset_bcs_layer_smoothing();

        if sustain_was_down {
            for voice in &mut self.voices {
                if voice.phase == VoicePhase::SustainedReleased {
                    voice.start_release();
                }
            }
        }

        self.refresh_resolved_state();
    }

    pub fn set_gfm_layer_mode(&mut self, mode: GfmLayerMode) -> GfmVoiceProgramSelection {
        self.gfm_layer_mode = mode;
        self.gfm_layer_auto_armed = false;
        self.gfm_layer_auto_disarm_pending = false;
        if matches!(mode, GfmLayerMode::Enabled { .. }) {
            self.control.gfm_layer_pressure = 0.0;
            self.control.gfm_layer_amount = 0.0;
        }
        self.reset_gfm_layer_smoothing();
        self.rebuild_gfm_layer();
        self.gfm_layer_selection
    }

    pub fn set_bcs_layer_mode(&mut self, mode: BcsLayerMode) -> BcsLayerSnapshot {
        self.bcs_layer_mode = mode;
        if matches!(mode, BcsLayerMode::Disabled) {
            self.reset_bcs_layer_smoothing();
        }
        self.rebuild_bcs_layer();
        self.update_bcs_layer_smoothing_targets();
        self.bcs_layer_snapshot()
    }

    fn auto_enable_gfm_layer_for_momentary_control(&mut self) {
        if matches!(self.gfm_layer_mode, GfmLayerMode::Disabled) {
            self.gfm_layer_mode = GfmLayerMode::Enabled {
                seed: DEFAULT_GFM_LAYER_SEED,
            };
            self.gfm_layer_auto_armed = true;
            self.gfm_layer_auto_disarm_pending = false;
            self.rebuild_gfm_layer();
        }
    }

    fn maybe_auto_enable_gfm_layer_for_momentary_control(&mut self) {
        if self.control.gfm_layer_amount > f32::EPSILON
            && self.control.gfm_layer_pressure > f32::EPSILON
        {
            self.auto_enable_gfm_layer_for_momentary_control();
            self.gfm_layer_auto_disarm_pending = false;
        }
    }

    fn begin_auto_disarm_gfm_layer_if_momentary_control_released(&mut self) {
        if self.gfm_layer_auto_armed {
            self.gfm_layer_auto_disarm_pending = true;
        }
    }

    fn finish_pending_auto_disarm_if_silent(&mut self) {
        if self.gfm_layer_auto_disarm_pending && self.gfm_layer_mix.current() <= f32::EPSILON {
            self.force_disable_auto_armed_gfm_layer();
        }
    }

    fn force_disable_auto_armed_gfm_layer(&mut self) {
        if self.gfm_layer_auto_armed || self.gfm_layer_auto_disarm_pending {
            self.gfm_layer_mode = GfmLayerMode::Disabled;
            self.gfm_layer_auto_armed = false;
            self.gfm_layer_auto_disarm_pending = false;
            self.rebuild_gfm_layer();
        }
    }

    fn reset_gfm_layer_smoothing(&mut self) {
        self.gfm_layer_mix = LinearSmoother::new(0.0);
        self.gfm_layer_pressure = LinearSmoother::new(0.0);
    }

    fn update_gfm_layer_smoothing_targets(&mut self) {
        let mix_target = gfm_momentary_layer_amount(
            self.control.gfm_layer_amount,
            self.control.gfm_layer_pressure,
        );
        let pressure_target = if mix_target <= f32::EPSILON {
            0.0
        } else {
            self.control.gfm_layer_pressure
        };
        let mix_ms = if mix_target > self.gfm_layer_mix.current() {
            GFM_LAYER_MIX_ATTACK_MS
        } else {
            GFM_LAYER_MIX_RELEASE_MS
        };
        let pressure_ms = if pressure_target > self.gfm_layer_pressure.current() {
            GFM_LAYER_PRESSURE_ATTACK_MS
        } else {
            GFM_LAYER_PRESSURE_RELEASE_MS
        };
        self.gfm_layer_mix.set_target(
            mix_target,
            smoothing_sample_count(self.config.sample_rate_hz, mix_ms),
        );
        self.gfm_layer_pressure.set_target(
            pressure_target,
            smoothing_sample_count(self.config.sample_rate_hz, pressure_ms),
        );
    }

    fn reset_bcs_layer_smoothing(&mut self) {
        self.bcs_layer_mix = LinearSmoother::new(0.0);
    }

    fn update_bcs_layer_smoothing_targets(&mut self) {
        let amount = normalize_bcs_layer_control(self.control.bcs_layer_amount);
        let target = if matches!(self.bcs_layer_mode, BcsLayerMode::Enabled { .. })
            && self.control.bcs_layer_enabled
            && self.bcs_layer_note.is_some()
        {
            amount
        } else {
            0.0
        };
        let mix_ms = if target > self.bcs_layer_mix.current() {
            BCS_LAYER_MIX_ATTACK_MS
        } else {
            BCS_LAYER_MIX_RELEASE_MS
        };
        self.bcs_layer_mix.set_target(
            target,
            smoothing_sample_count(self.config.sample_rate_hz, mix_ms),
        );
    }

    pub const fn gfm_layer_mode(&self) -> GfmLayerMode {
        self.gfm_layer_mode
    }

    pub const fn bcs_layer_mode(&self) -> BcsLayerMode {
        self.bcs_layer_mode
    }

    pub fn gfm_layer_diagnostics(&self) -> Option<GfmDiagnostics> {
        self.gfm_layer_voice
            .as_ref()
            .map(GfmFieldVoice::diagnostics)
    }

    pub fn process_block(&mut self, block: ProcessBlock<'_>) {
        let note_events = block.note_events;
        let controller_events = block.controller_events;
        debug_assert!(is_sorted_by_frame(note_events));
        debug_assert!(is_sorted_by_frame(controller_events));

        let requested_frames = block.frame_count.min(self.config.max_block_frames);
        let mut output = block.output;
        let rendered_frames = output
            .as_ref()
            .map(|buffer| requested_frames.min(buffer.frames()))
            .unwrap_or(0);
        if let Some(buffer) = output.as_mut() {
            buffer.clear();
        }

        if let Some(macro_state) = block.macro_state {
            self.live_macros = macro_state.clamped();
        }
        self.refresh_resolved_state();

        let mut note_index = 0;
        let mut controller_index = 0;
        let mut events_processed = 0;
        let mut peak_output: f32 = 0.0;
        let mut output_safety = OutputSafetySnapshot::default();

        for frame in 0..requested_frames {
            while controller_index < controller_events.len()
                && controller_events[controller_index].frame_offset <= frame
            {
                self.handle_controller_event(controller_events[controller_index].event);
                controller_index += 1;
                events_processed += 1;
                self.refresh_resolved_state();
            }

            while note_index < note_events.len() && note_events[note_index].frame_offset <= frame {
                self.handle_note_event(note_events[note_index].event);
                note_index += 1;
                events_processed += 1;
            }

            let (left, right) = self.render_frame();
            let (left, right) = self.master_dc_blocker.process(left, right);
            output_safety.pre_safety_peak = output_safety
                .pre_safety_peak
                .max(left.abs().max(right.abs()));

            let limited_left = master_safety_limit(left);
            let limited_right = master_safety_limit(right);
            output_safety.observe_channel(left, limited_left);
            output_safety.observe_channel(right, limited_right);

            let left = sanitize_sample(limited_left);
            let right = sanitize_sample(limited_right);
            peak_output = peak_output.max(left.abs().max(right.abs()));
            output_safety.post_safety_peak = output_safety
                .post_safety_peak
                .max(left.abs().max(right.abs()));

            if frame < rendered_frames {
                let Some(buffer) = output.as_mut() else {
                    continue;
                };
                buffer.left[frame] = left;
                buffer.right[frame] = right;
            }
        }

        self.last_block_frames = requested_frames;
        self.last_events_processed = events_processed;
        self.last_peak_output = peak_output;
        self.last_output_safety = output_safety;
    }

    pub fn snapshot(&self) -> EngineSnapshot {
        let effective_macros = self.effective_macro_state();
        let voices: Vec<VoiceSnapshot> = self
            .voices
            .iter()
            .enumerate()
            .map(|(slot, voice)| VoiceSnapshot {
                slot,
                note: voice.note,
                velocity: voice.velocity,
                phase: voice.phase,
                age: voice.age,
            })
            .collect();

        let held_notes = voices
            .iter()
            .filter_map(|voice| match voice.phase {
                VoicePhase::Held | VoicePhase::Released | VoicePhase::SustainedReleased => {
                    voice.note
                }
                VoicePhase::Idle => None,
            })
            .collect();

        EngineSnapshot {
            patch_name: self.patch.meta.patch_name.clone(),
            patch_description: self.patch.meta.description.clone(),
            patch_tags: self.patch.meta.tags.clone().unwrap_or_default(),
            patch_favorite: self
                .patch
                .ui
                .as_ref()
                .and_then(|ui| ui.favorite)
                .unwrap_or(false),
            sample_rate_hz: self.config.sample_rate_hz,
            last_block_frames: self.last_block_frames,
            events_processed: self.last_events_processed,
            active_voice_count: self
                .voices
                .iter()
                .filter(|voice| voice.phase != VoicePhase::Idle)
                .count(),
            sustain_down: self.control.sustain_down,
            live_macros: self.live_macros,
            effective_macros,
            identity: self.last_frame.identity,
            derived: self.last_frame.derived,
            direct: self.last_direct,
            gfm_layer: self.gfm_layer_snapshot(),
            bcs_layer: self.bcs_layer_snapshot(),
            voices,
            held_notes,
            output_safety: self.last_output_safety,
            peak_output: self.last_peak_output,
            clip_detected: self.last_peak_output >= 0.98,
        }
    }

    fn reset_runtime_state(&mut self) {
        self.control = ControlState::default();
        self.age_counter = 0;
        self.last_block_frames = 0;
        self.last_events_processed = 0;
        self.last_peak_output = 0.0;
        self.last_output_safety = OutputSafetySnapshot::default();
        self.final_body_left = 0.0;
        self.final_body_right = 0.0;
        self.chorus = SimpleChorus::new(self.config.sample_rate_hz);
        self.reverb = SimpleReverb::new(self.config.sample_rate_hz);
        self.master_dc_blocker.reset();
        self.voices = (0..self.config.voice_count)
            .map(|slot| VoiceState::idle(self.config.sample_rate_hz, slot))
            .collect();
        if self.gfm_layer_auto_armed || self.gfm_layer_auto_disarm_pending {
            self.gfm_layer_mode = GfmLayerMode::Disabled;
            self.gfm_layer_auto_armed = false;
            self.gfm_layer_auto_disarm_pending = false;
        }
        self.reset_gfm_layer_smoothing();
        self.gfm_layer_selection = GfmVoiceProgramSelection::default();
        self.gfm_layer_voice = None;
        self.reset_bcs_layer_smoothing();
        self.bcs_layer_note = None;
        self.bcs_layer_frequency_hz = None;
        self.bcs_layer_voice = None;
    }

    fn refresh_resolved_state(&mut self) {
        let effective_macros = self.effective_macro_state();
        self.last_frame = resolve_identity(&self.patch, &effective_macros);
        self.last_direct = resolve_direct_parameters(&self.patch, self.last_frame, self.control);
        self.render_smoothers
            .set_targets(self.last_direct, self.control_smoothing_samples);

        for voice in &mut self.voices {
            voice.amp_env.set_timing(self.last_direct.amp_env);
            voice.filter_env.set_timing(self.last_direct.filter_env);
        }
    }

    fn rebuild_gfm_layer(&mut self) {
        match self.gfm_layer_mode {
            GfmLayerMode::Disabled => {
                self.gfm_layer_selection = GfmVoiceProgramSelection::default();
                self.gfm_layer_voice = None;
            }
            GfmLayerMode::Enabled { seed } => {
                let config = GfmPatchVoiceConfig::new(
                    seed,
                    engine_gfm_sample_rate_hz(self.config.sample_rate_hz),
                );
                let decision = GfmFieldVoice::from_patch(&self.patch, config);
                self.gfm_layer_selection = decision.selection();
                self.gfm_layer_voice = decision.into_voice();
            }
        }
    }

    fn rebuild_bcs_layer(&mut self) {
        match self.bcs_layer_mode {
            BcsLayerMode::Disabled => {
                self.bcs_layer_voice = None;
            }
            BcsLayerMode::Enabled { scenario } => {
                let mut params = BcsParams::for_scenario(scenario);
                params.sample_rate_hz = self.config.sample_rate_hz;
                if let Some(frequency_hz) = self.bcs_layer_frequency_hz {
                    params.base_frequency_hz = frequency_hz;
                }
                self.bcs_layer_voice = Some(BcsVoice::new(params));
            }
        }
    }

    fn refresh_bcs_layer_pitch(&mut self) {
        let desired = self.current_bcs_layer_pitch();
        match desired {
            Some((note, frequency_hz)) => {
                let changed = self.bcs_layer_note != Some(note)
                    || self
                        .bcs_layer_frequency_hz
                        .map_or(true, |current| (current - frequency_hz).abs() > 0.001);
                self.bcs_layer_note = Some(note);
                self.bcs_layer_frequency_hz = Some(frequency_hz);
                if changed {
                    if let Some(voice) = self.bcs_layer_voice.as_mut() {
                        voice.set_base_frequency_hz(frequency_hz);
                    } else {
                        self.rebuild_bcs_layer();
                    }
                }
            }
            None => {
                self.bcs_layer_note = None;
                self.bcs_layer_frequency_hz = None;
            }
        }
        self.update_bcs_layer_smoothing_targets();
    }

    fn current_bcs_layer_pitch(&self) -> Option<(u8, f32)> {
        self.voices
            .iter()
            .filter_map(|voice| match voice.phase {
                VoicePhase::Held => voice.note,
                VoicePhase::Idle | VoicePhase::Released | VoicePhase::SustainedReleased => None,
            })
            .min()
            .map(|note| {
                (
                    note,
                    midi_note_hz(note as f32 + self.control.pitch_bend_semitones),
                )
            })
    }

    fn gfm_layer_snapshot(&self) -> GfmLayerSnapshot {
        GfmLayerSnapshot {
            mode: self.gfm_layer_mode,
            selection: self.gfm_layer_selection,
            active_program_id: self.gfm_layer_voice.as_ref().map(GfmFieldVoice::program_id),
            diagnostics: self.gfm_layer_diagnostics(),
            amount: self.control.gfm_layer_amount,
            pressure: self.control.gfm_layer_pressure,
            effective_amount: self.gfm_layer_mix.current(),
        }
    }

    fn bcs_layer_snapshot(&self) -> BcsLayerSnapshot {
        let mode_enabled = matches!(self.bcs_layer_mode, BcsLayerMode::Enabled { .. });
        let active_scenario = match (self.bcs_layer_mode, self.bcs_layer_voice.as_ref()) {
            (BcsLayerMode::Enabled { scenario }, Some(_)) => Some(scenario),
            _ => None,
        };
        let sample_rate_hz = self.bcs_layer_voice.as_ref().map(|voice| {
            voice
                .params()
                .sample_rate_hz
                .round()
                .clamp(1.0, u32::MAX as f32) as u32
        });

        BcsLayerSnapshot {
            mode: self.bcs_layer_mode,
            active_scenario,
            sample_rate_hz,
            enabled: self.control.bcs_layer_enabled,
            amount: self.control.bcs_layer_amount,
            effective_amount: self.bcs_layer_mix.current(),
            pitch_note: if mode_enabled {
                self.bcs_layer_note
            } else {
                None
            },
            pitch_frequency_hz: if mode_enabled {
                self.bcs_layer_frequency_hz
            } else {
                None
            },
            max_state_abs: self
                .bcs_layer_voice
                .as_ref()
                .map(BcsVoice::max_state_abs)
                .unwrap_or(0.0),
            unsafe_events: self
                .bcs_layer_voice
                .as_ref()
                .map(BcsVoice::unsafe_events)
                .unwrap_or(0),
            unsafe_state: self
                .bcs_layer_voice
                .as_ref()
                .map(BcsVoice::unsafe_state)
                .unwrap_or(false),
        }
    }

    fn handle_note_event(&mut self, event: NoteEvent) {
        match event {
            NoteEvent::NoteOn { note, velocity } if velocity > 0.0 => self.note_on(note, velocity),
            NoteEvent::NoteOn { note, .. } | NoteEvent::NoteOff { note } => self.note_off(note),
        }
    }

    fn handle_controller_event(&mut self, event: ControllerEvent) {
        match event {
            ControllerEvent::PitchBend { semitones } => {
                self.control.pitch_bend_semitones = semitones;
                self.refresh_bcs_layer_pitch();
            }
            ControllerEvent::ModWheel { amount } => {
                self.control.mod_wheel = amount.clamp(0.0, 1.0);
            }
            ControllerEvent::Sustain { down } => {
                self.control.sustain_down = down;
                if !down {
                    for voice in &mut self.voices {
                        if voice.phase == VoicePhase::SustainedReleased {
                            voice.start_release();
                        }
                    }
                }
                self.refresh_bcs_layer_pitch();
            }
            ControllerEvent::ChannelAftertouch { pressure } => {
                let pressure = pressure.clamp(0.0, 1.0);
                let gfm_amount = normalize_gfm_layer_control(pressure);
                self.control.aftertouch = pressure;
                self.control.gfm_layer_amount = gfm_amount;
                if gfm_amount > f32::EPSILON {
                    self.maybe_auto_enable_gfm_layer_for_momentary_control();
                } else {
                    self.begin_auto_disarm_gfm_layer_if_momentary_control_released();
                }
                self.update_gfm_layer_smoothing_targets();
            }
            ControllerEvent::GfmLayerAmount { amount } => {
                let pressure = normalize_gfm_layer_control(amount);
                self.control.gfm_layer_pressure = pressure;
                if pressure > f32::EPSILON {
                    self.maybe_auto_enable_gfm_layer_for_momentary_control();
                } else {
                    self.begin_auto_disarm_gfm_layer_if_momentary_control_released();
                }
                self.update_gfm_layer_smoothing_targets();
            }
            ControllerEvent::BcsLayerEnabled { enabled } => {
                self.control.bcs_layer_enabled = enabled;
                self.refresh_bcs_layer_pitch();
            }
            ControllerEvent::BcsLayerAmount { amount } => {
                self.control.bcs_layer_amount = normalize_bcs_layer_control(amount);
                self.refresh_bcs_layer_pitch();
            }
            ControllerEvent::Macro { id, value } => {
                self.live_macros.set(id, value.clamp(0.0, 1.0));
            }
            ControllerEvent::DirectParam { id, value } => self.apply_direct_param(id, value),
        }
    }

    fn note_on(&mut self, note: u8, velocity: f32) {
        let voice_index = self.allocate_voice_index();
        self.age_counter += 1;
        self.voices[voice_index].trigger(
            note,
            velocity,
            self.age_counter,
            self.last_direct.amp_env,
            self.last_direct.filter_env,
            voice_index,
        );
        self.refresh_bcs_layer_pitch();
    }

    fn note_off(&mut self, note: u8) {
        if let Some(index) = self
            .voices
            .iter()
            .enumerate()
            .filter(|(_, voice)| voice.note == Some(note) && voice.phase == VoicePhase::Held)
            .min_by_key(|(_, voice)| voice.age)
            .map(|(index, _)| index)
        {
            let voice = &mut self.voices[index];
            if self.control.sustain_down {
                voice.phase = VoicePhase::SustainedReleased;
            } else {
                voice.start_release();
            }
        }
        if !self
            .voices
            .iter()
            .any(|voice| voice.phase == VoicePhase::Held)
        {
            self.control.gfm_layer_amount = 0.0;
            self.begin_auto_disarm_gfm_layer_if_momentary_control_released();
            self.update_gfm_layer_smoothing_targets();
        }
        self.refresh_bcs_layer_pitch();
    }

    fn allocate_voice_index(&self) -> usize {
        if let Some(index) = self
            .voices
            .iter()
            .position(|voice| voice.phase == VoicePhase::Idle)
        {
            return index;
        }

        if let Some((index, _)) = self
            .voices
            .iter()
            .enumerate()
            .filter(|(_, voice)| voice.phase == VoicePhase::Released)
            .min_by(|left, right| compare_voice_reuse(left.1, right.1))
        {
            return index;
        }

        if let Some((index, _)) = self
            .voices
            .iter()
            .enumerate()
            .filter(|(_, voice)| voice.phase == VoicePhase::SustainedReleased)
            .min_by(|left, right| compare_voice_reuse(left.1, right.1))
        {
            return index;
        }

        self.voices
            .iter()
            .enumerate()
            .filter(|(_, voice)| voice.phase == VoicePhase::Held)
            .min_by_key(|(_, voice)| voice.age)
            .map(|(index, _)| index)
            .unwrap_or(0)
    }

    fn effective_macro_state(&self) -> MacroState {
        let performance = self.patch.performance_response;
        let mut macros = self.live_macros;
        let aftertouch = expressive_amount(self.control.aftertouch, 0.82);
        let mod_wheel = expressive_amount(self.control.mod_wheel, 0.92);
        macros.gravitacija += aftertouch * performance.aftertouch_to_gravitacija;
        macros.ruin += aftertouch.powf(1.08) * performance.aftertouch_to_baklja;
        macros.bloom += mod_wheel * performance.mod_wheel_to_bloom;
        macros.swarm += mod_wheel.powf(0.88) * performance.mod_wheel_to_swarm;
        macros.clamped()
    }

    fn render_frame(&mut self) -> (f32, f32) {
        if self.patch_switch_mute_frames > 0 {
            self.patch_switch_mute_frames -= 1;
            return (0.0, 0.0);
        }

        let direct = self.render_smoothers.next_direct(self.last_direct);
        let derived = self.last_frame.derived;
        let identity = self.last_frame.identity;
        let engine_patch = &self.patch.engine;
        let sample_rate_hz = self.config.sample_rate_hz;
        let pitch_bend_semitones = self.control.pitch_bend_semitones;
        let note_velocity_to_level = engine_patch
            .voice
            .velocity_to_level
            .unwrap_or(self.patch.performance_response.velocity_to_level)
            .clamp(0.0, 1.0);
        let note_velocity_to_filter = engine_patch
            .voice
            .velocity_to_filter
            .unwrap_or(self.patch.performance_response.velocity_to_filter)
            .clamp(0.0, 1.0);
        let voice_count = self.voices.len().max(1);
        let osc1_fine = engine_patch.osc1.fine_tune_cents.unwrap_or(0.0) / 100.0;
        let osc2_fine = engine_patch.osc2.fine_tune_cents / 100.0;
        let sub_octave = engine_patch.sub.octave_offset as f32 * 12.0;
        let mixer_body_gain = 0.4 + engine_patch.mixer.body_mix * 0.6;
        let pre_filter_gain = 1.0 + engine_patch.mixer.pre_filter_drive * 2.0;
        let voice_level_gain = 0.40 + direct.voice_level * 0.60;
        let stereo_width = direct.stereo_width;
        let strain_drive = 1.0 + derived.strain * 0.22;
        let strain_bias = identity.baklja_edge * 0.18;

        let mut left = 0.0;
        let mut right = 0.0;

        for (slot, voice) in self.voices.iter_mut().enumerate() {
            if voice.phase == VoicePhase::Idle {
                continue;
            }

            let Some(note) = voice.note else {
                continue;
            };

            let spread_position = if voice_count == 1 {
                0.0
            } else {
                (slot as f32 / (voice_count - 1) as f32) * 2.0 - 1.0
            };
            let spread_detune_semitones =
                spread_position * direct.detune_spread_cents * 0.5 / 100.0;
            let pulse_width = (0.50 + identity.baklja_edge * 0.18 - identity.horizont_air * 0.05)
                .clamp(0.08, 0.92);

            let osc1_note =
                note as f32 + pitch_bend_semitones + osc1_fine + spread_detune_semitones;
            let osc1_freq = midi_note_hz(osc1_note);
            let osc1_mix = mixed_wave(
                &voice.osc1,
                direct.osc1_wave_mix,
                pulse_width,
                &mut voice.noise,
            );
            let osc1_wrapped = voice.osc1.advance(osc1_freq, sample_rate_hz);

            if osc1_wrapped && direct.sync_amount > 0.0 {
                voice.osc2.hard_sync(direct.sync_amount);
            }

            let osc2_note =
                note as f32 + direct.osc2_interval_semitones + osc2_fine + spread_detune_semitones;
            let osc2_freq = midi_note_hz(osc2_note)
                * (1.0 + osc1_mix * direct.crossmod_amount * 0.25).clamp(0.25, 4.0);
            let osc2_mix = mixed_wave_osc2(&voice.osc2, direct.osc2_wave_mix, pulse_width);
            voice.osc2.advance(osc2_freq, sample_rate_hz);

            let sub_freq = midi_note_hz(note as f32 + pitch_bend_semitones + sub_octave);
            let sub_mix = voice.sub.square_sample() * direct.sub_level;
            voice.sub.advance(sub_freq, sample_rate_hz);

            let body_mix = sub_mix * mixer_body_gain * (0.62 + derived.mass * 0.46);
            let pre_filter = soft_clip(
                (osc1_mix + osc2_mix + body_mix) * (pre_filter_gain + direct.filter_drive * 0.8),
                strain_bias,
            );

            let filter_env = voice.filter_env.next_sample();
            let keytrack = (1.0 + ((note as f32 - 60.0) / 48.0) * direct.filter_tracking * 0.42)
                .clamp(0.55, 1.35);
            let velocity_level = shaped_velocity_level(voice.velocity);
            let velocity_filter_curve = shaped_velocity_filter(voice.velocity);
            let velocity_filter = 1.0 + velocity_filter_curve * note_velocity_to_filter * 0.85;
            let cutoff_hz = (direct.cutoff_hz
                * (0.42 + filter_env * direct.filter_env_depth * 0.78)
                * keytrack
                * velocity_filter)
                .clamp(20.0, sample_rate_hz * 0.42);
            let filtered = voice.filter.process(
                pre_filter,
                cutoff_hz,
                direct.resonance,
                direct.filter_drive,
                derived.strain,
                sample_rate_hz,
            );

            let amp = voice.amp_env.next_sample();
            let velocity_gain =
                1.0 - note_velocity_to_level + velocity_level * note_velocity_to_level;
            let mut sample = filtered * amp * velocity_gain * voice_level_gain;
            sample = soft_clip(sample * strain_drive, direct.final_asymmetry * 0.14);

            if voice.phase != VoicePhase::Held && voice.amp_env.is_idle() {
                *voice = VoiceState::idle(sample_rate_hz, slot);
                continue;
            }

            let pan = spread_position * stereo_width;
            let left_gain = ((1.0 - pan) * 0.5).clamp(0.0, 1.0).sqrt();
            let right_gain = ((1.0 + pan) * 0.5).clamp(0.0, 1.0).sqrt();
            left += sample * left_gain;
            right += sample * right_gain;
        }

        self.final_body_left += (left - self.final_body_left) * (0.022 + derived.mass * 0.026);
        self.final_body_right += (right - self.final_body_right) * (0.022 + derived.mass * 0.026);

        let raw_mid = (left + right) * 0.5;
        let raw_side = (left - right) * 0.5;
        let body_mid = (self.final_body_left + self.final_body_right) * 0.5;
        let focus_amount = (derived.body_focus * 0.26
            + identity.grav_pull * 0.22
            + direct.stereo_crossfeed * 0.18)
            .clamp(0.0, 0.75);
        let saturated_mid = soft_clip(
            (raw_mid + body_mid * (0.30 + direct.low_mid_emphasis * 0.58))
                * (1.0 + direct.body_drive * 1.75 + direct.final_saturation * 1.25),
            direct.final_asymmetry * 0.70,
        );
        let saturated_side = soft_clip(
            raw_side * (1.0 + direct.final_saturation * 0.28),
            -direct.final_asymmetry * 0.18,
        ) * (1.0 - focus_amount);

        left = saturated_mid + saturated_side;
        right = saturated_mid - saturated_side;

        if direct.chorus_enabled {
            (left, right) = self.chorus.process(
                left,
                right,
                direct.chorus_mix,
                direct.chorus_depth,
                direct.chorus_rate_hz,
            );
        }

        if direct.reverb_enabled {
            (left, right) = self.reverb.process(
                left,
                right,
                direct.reverb_mix,
                direct.reverb_size,
                direct.reverb_damping,
            );
        }

        (left, right) = self.apply_gfm_layer(left, right, direct);
        (left, right) = self.apply_bcs_layer(left, right, direct);

        let crossfeed = (0.04 + direct.stereo_crossfeed * 0.16).clamp(0.0, 0.22);
        let crossfed_left = left * (1.0 - crossfeed) + right * crossfeed;
        let crossfed_right = right * (1.0 - crossfeed) + left * crossfeed;
        let output_gain = db_to_gain(direct.output_trim_db);

        (crossfed_left * output_gain, crossfed_right * output_gain)
    }

    fn apply_gfm_layer(&mut self, left: f32, right: f32, direct: DirectParameters) -> (f32, f32) {
        let Some(program_id) = self.gfm_layer_voice.as_ref().map(GfmFieldVoice::program_id) else {
            return (left, right);
        };

        let pressure = self.gfm_layer_pressure.next_value();
        let effective_amount = self.gfm_layer_mix.next_value();
        let gfm_sample = {
            let Some(voice) = self.gfm_layer_voice.as_mut() else {
                return (left, right);
            };
            voice.next_sample_with_live_pressure(pressure)
        };
        if effective_amount <= f32::EPSILON {
            self.finish_pending_auto_disarm_if_silent();
            return (left, right);
        }
        let activity = ((left.abs() + right.abs()) * 0.75).clamp(0.0, 1.0);
        let gate = if activity <= 0.000_01 {
            0.0
        } else {
            (0.20 + activity * 0.80).clamp(0.0, 1.0)
        };
        let layer = gfm_sample * gate * gfm_engine_layer_gain(program_id) * effective_amount;
        let spread = (direct.stereo_width * 0.16 + direct.stereo_crossfeed * 0.04).clamp(0.0, 0.22);
        let left = soft_clip(left + layer * (1.0 - spread), direct.final_asymmetry * 0.20);
        let right = soft_clip(
            right + layer * (1.0 + spread),
            -direct.final_asymmetry * 0.20,
        );

        (left, right)
    }

    fn apply_bcs_layer(&mut self, left: f32, right: f32, direct: DirectParameters) -> (f32, f32) {
        let BcsLayerMode::Enabled { scenario } = self.bcs_layer_mode else {
            return (left, right);
        };

        let effective_amount = self.bcs_layer_mix.next_value();
        let Some(voice) = self.bcs_layer_voice.as_mut() else {
            return (left, right);
        };

        let bcs_sample = voice.next_sample(BcsGesture::new(scenario));
        if voice.unsafe_state() || !bcs_sample.is_finite() {
            return (left, right);
        }
        if effective_amount <= f32::EPSILON {
            return (left, right);
        }

        let activity = ((left.abs() + right.abs()) * 0.75).clamp(0.0, 1.0);
        if activity <= 0.000_01 {
            return (left, right);
        }

        let gate = (0.18 + activity * 0.82).clamp(0.0, 1.0);
        let layer = bcs_sample * BCS_ENGINE_LAYER_GAIN * gate * effective_amount;
        let spread = (direct.stereo_width * 0.10 + direct.stereo_crossfeed * 0.03).clamp(0.0, 0.16);
        let left = soft_clip(left + layer * (1.0 - spread), direct.final_asymmetry * 0.16);
        let right = soft_clip(
            right + layer * (1.0 + spread),
            -direct.final_asymmetry * 0.16,
        );

        (left, right)
    }

    fn apply_direct_param(&mut self, id: ParamId, value: f32) {
        let spec = param_spec(id);
        let clamped = value.clamp(spec.min, spec.max);

        match id {
            ParamId::GravitacijaMacro
            | ParamId::BloomMacro
            | ParamId::HeatMacro
            | ParamId::RuinMacro
            | ParamId::SwarmMacro => {
                let macro_id = spec.macro_id.unwrap_or(MacroId::Gravitacija);
                self.live_macros.set(macro_id, clamped);
                self.patch.macros =
                    macro_defaults_with_override(&self.patch.macros, macro_id, clamped);
            }
            ParamId::Osc1SawLevel => self.patch.engine.osc1.saw_level = clamped,
            ParamId::Osc1PulseLevel => self.patch.engine.osc1.pulse_level = clamped,
            ParamId::Osc1TriangleLevel => self.patch.engine.osc1.triangle_level = clamped,
            ParamId::Osc1NoiseLevel => self.patch.engine.osc1.noise_level = clamped,
            ParamId::Osc1FineTuneCents => self.patch.engine.osc1.fine_tune_cents = Some(clamped),
            ParamId::Osc2SawLevel => self.patch.engine.osc2.saw_level = clamped,
            ParamId::Osc2PulseLevel => self.patch.engine.osc2.pulse_level = clamped,
            ParamId::Osc2TriangleLevel => self.patch.engine.osc2.triangle_level = clamped,
            ParamId::Osc2IntervalSemitones => {
                self.patch.engine.osc2.interval_semitones = clamped.round() as i8
            }
            ParamId::Osc2FineTuneCents => self.patch.engine.osc2.fine_tune_cents = clamped,
            ParamId::Osc2SyncAmount => self.patch.engine.osc2.sync_amount = clamped,
            ParamId::Osc2CrossmodAmount => self.patch.engine.osc2.crossmod_amount = clamped,
            ParamId::SubLevel => self.patch.engine.sub.level = clamped,
            ParamId::SubOctaveOffset => self.patch.engine.sub.octave_offset = clamped.round() as i8,
            ParamId::MixerPreFilterDrive => self.patch.engine.mixer.pre_filter_drive = clamped,
            ParamId::MixerBodyMix => self.patch.engine.mixer.body_mix = clamped,
            ParamId::FilterCutoffHz => self.patch.engine.filter.cutoff_hz = clamped,
            ParamId::FilterResonance => self.patch.engine.filter.resonance = clamped,
            ParamId::FilterDrive => self.patch.engine.filter.drive = clamped,
            ParamId::FilterKeytrack => self.patch.engine.filter.keytrack = clamped,
            ParamId::AmpEnvAttackMs => self.patch.engine.amp_env.attack_ms = clamped,
            ParamId::AmpEnvDecayMs => self.patch.engine.amp_env.decay_ms = clamped,
            ParamId::AmpEnvSustain => self.patch.engine.amp_env.sustain = clamped,
            ParamId::AmpEnvReleaseMs => self.patch.engine.amp_env.release_ms = clamped,
            ParamId::FilterEnvAttackMs => self.patch.engine.filter_env.adsr.attack_ms = clamped,
            ParamId::FilterEnvDecayMs => self.patch.engine.filter_env.adsr.decay_ms = clamped,
            ParamId::FilterEnvSustain => self.patch.engine.filter_env.adsr.sustain = clamped,
            ParamId::FilterEnvReleaseMs => self.patch.engine.filter_env.adsr.release_ms = clamped,
            ParamId::FilterEnvDepth => self.patch.engine.filter_env.depth = clamped,
            ParamId::VoiceStereoWidth => self.patch.engine.voice.stereo_width = clamped,
            ParamId::VoiceDetuneSpreadCents => {
                self.patch.engine.voice.detune_spread_cents = clamped
            }
            ParamId::VoiceVelocityToLevel => {
                self.patch.engine.voice.velocity_to_level = Some(clamped)
            }
            ParamId::VoiceVelocityToFilter => {
                self.patch.engine.voice.velocity_to_filter = Some(clamped)
            }
            ParamId::FinalStageBodyDrive => self.patch.engine.final_stage.body_drive = clamped,
            ParamId::FinalStageAsymmetry => self.patch.engine.final_stage.asymmetry = clamped,
            ParamId::FinalStageLowMidEmphasis => {
                self.patch.engine.final_stage.low_mid_emphasis = clamped
            }
            ParamId::FinalStageOutputTrimDb => {
                self.patch.engine.final_stage.output_trim_db = clamped
            }
            ParamId::ChorusEnabled => self.patch.engine.fx.chorus.enabled = clamped >= 0.5,
            ParamId::ChorusMix => self.patch.engine.fx.chorus.mix = clamped,
            ParamId::ChorusDepth => self.patch.engine.fx.chorus.depth = clamped,
            ParamId::ChorusRateHz => self.patch.engine.fx.chorus.rate_hz = clamped,
            ParamId::ReverbEnabled => self.patch.engine.fx.reverb.enabled = clamped >= 0.5,
            ParamId::ReverbMix => self.patch.engine.fx.reverb.mix = clamped,
            ParamId::ReverbSize => self.patch.engine.fx.reverb.size = clamped,
            ParamId::ReverbDamping => self.patch.engine.fx.reverb.damping = clamped,
        }
    }
}

fn is_sorted_by_frame<T>(events: &[Scheduled<T>]) -> bool {
    events
        .windows(2)
        .all(|pair| pair[0].frame_offset <= pair[1].frame_offset)
}

fn compare_voice_reuse(left: &VoiceState, right: &VoiceState) -> std::cmp::Ordering {
    left.amp_env
        .current_level()
        .partial_cmp(&right.amp_env.current_level())
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| left.age.cmp(&right.age))
}

fn expressive_amount(value: f32, exponent: f32) -> f32 {
    value.clamp(0.0, 1.0).powf(exponent.max(0.01))
}

fn shaped_velocity_level(value: f32) -> f32 {
    value.clamp(0.0, 1.0).powf(0.78)
}

fn shaped_velocity_filter(value: f32) -> f32 {
    value.clamp(0.0, 1.0).powf(1.08)
}

fn patch_switch_mute_frames(sample_rate_hz: f32) -> usize {
    ((sample_rate_hz * 0.004).round() as usize).clamp(32, 256)
}

fn mixed_wave(
    oscillator: &Oscillator,
    mix_levels: [f32; 4],
    pulse_width: f32,
    noise: &mut NoiseRng,
) -> f32 {
    let saw = oscillator.saw_sample() * mix_levels[0];
    let pulse = oscillator.pulse_sample(pulse_width) * mix_levels[1];
    let triangle = oscillator.triangle_sample() * mix_levels[2];
    let noise = noise.next_bipolar() * mix_levels[3] * 0.85;
    normalize_weighted_mix(saw + pulse + triangle + noise, &mix_levels)
}

fn mixed_wave_osc2(oscillator: &Oscillator, mix_levels: [f32; 3], pulse_width: f32) -> f32 {
    let saw = oscillator.saw_sample() * mix_levels[0];
    let pulse = oscillator.pulse_sample(pulse_width) * mix_levels[1];
    let triangle = oscillator.triangle_sample() * mix_levels[2];
    normalize_weighted_mix(saw + pulse + triangle, &mix_levels)
}

fn normalize_weighted_mix<const N: usize>(sample_sum: f32, mix_levels: &[f32; N]) -> f32 {
    let normalizer = mix_levels.iter().copied().sum::<f32>().max(1.0);
    sample_sum / normalizer
}

fn control_smoothing_samples(sample_rate_hz: f32) -> usize {
    let smoothing_window = (sample_rate_hz * 0.006).round() as usize;
    smoothing_window.max(8).min(512)
}

fn smoothing_sample_count(sample_rate_hz: f32, milliseconds: f32) -> usize {
    let sample_count = (sample_rate_hz * milliseconds.max(0.0) * 0.001).round() as usize;
    sample_count.max(1)
}

fn engine_gfm_sample_rate_hz(sample_rate_hz: f32) -> u32 {
    if sample_rate_hz.is_finite() {
        sample_rate_hz.round().clamp(1.0, u32::MAX as f32) as u32
    } else {
        48_000
    }
}

fn gfm_engine_layer_gain(program_id: GfmProgramId) -> f32 {
    match program_id {
        GfmProgramId::HorizontPerformance => 0.14,
        GfmProgramId::PecPerformance => 0.20,
        GfmProgramId::BakljaPerformance => 0.075,
    }
}

fn resolve_direct_parameters(
    patch: &PatchFileV1,
    resolved_frame: ResolvedIdentityFrame,
    control: ControlState,
) -> DirectParameters {
    let engine = &patch.engine;
    let identity = resolved_frame.identity;
    let derived = resolved_frame.derived;
    let shaped = resolved_frame.shaped_macros;

    let cutoff_scale =
        0.90 + shaped.bloom * 0.62 + identity.horizont_air * 0.18 - shaped.gravitacija * 0.40;
    let cutoff_hz = (engine.filter.cutoff_hz * cutoff_scale).clamp(20.0, 20_000.0);
    let stereo_width = (engine.voice.stereo_width + identity.horizont_span * 0.18).clamp(0.0, 1.0);
    let stereo_crossfeed = (0.10 + derived.body_focus * 0.28 + identity.grav_pull * 0.10
        - derived.spatial_dispersion * 0.10)
        .clamp(0.0, 1.0);

    DirectParameters {
        osc1_wave_mix: [
            engine.osc1.saw_level,
            engine.osc1.pulse_level,
            engine.osc1.triangle_level,
            engine.osc1.noise_level,
        ],
        osc2_wave_mix: [
            engine.osc2.saw_level,
            engine.osc2.pulse_level,
            engine.osc2.triangle_level,
        ],
        sub_level: (engine.sub.level * (0.70 + derived.mass * 0.30)).clamp(0.0, 1.0),
        mixer_pre_filter_drive: engine.mixer.pre_filter_drive,
        mixer_body_mix: engine.mixer.body_mix,
        osc2_interval_semitones: engine.osc2.interval_semitones as f32
            + control.pitch_bend_semitones.clamp(
                -(patch.performance_response.bend_range_semitones as f32),
                patch.performance_response.bend_range_semitones as f32,
            ),
        sync_amount: (engine.osc2.sync_amount + identity.baklja_sync_bias * 0.30).clamp(0.0, 1.0),
        crossmod_amount: (engine.osc2.crossmod_amount + identity.baklja_ready * 0.24)
            .clamp(0.0, 1.0),
        detune_spread_cents: (engine.voice.detune_spread_cents * (0.72 + shaped.swarm * 0.42))
            .clamp(0.0, 50.0),
        cutoff_hz,
        resonance: (engine.filter.resonance + identity.baklja_edge * 0.14 + shaped.ruin * 0.06
            - derived.mass * 0.04)
            .clamp(0.0, 1.0),
        filter_drive: (engine.filter.drive + identity.pec_heat * 0.18 + derived.strain * 0.10)
            .clamp(0.0, 1.0),
        filter_env_depth: (engine.filter_env.depth + identity.horizont_open * 0.12).clamp(0.0, 1.0),
        filter_tracking: (engine.filter.keytrack * (0.92 - derived.mass * 0.12)).clamp(0.0, 1.0),
        amp_env: AdsrTiming {
            attack_ms: engine.amp_env.attack_ms,
            decay_ms: engine.amp_env.decay_ms,
            sustain: engine.amp_env.sustain,
            release_ms: engine.amp_env.release_ms,
        }
        .clamp(),
        filter_env: AdsrTiming {
            attack_ms: engine.filter_env.adsr.attack_ms,
            decay_ms: engine.filter_env.adsr.decay_ms,
            sustain: engine.filter_env.adsr.sustain,
            release_ms: engine.filter_env.adsr.release_ms,
        }
        .clamp(),
        voice_level: (0.75
            + derived.mass * 0.20
            + patch.performance_response.velocity_to_level * 0.05)
            .clamp(0.0, 1.0),
        body_drive: (engine.final_stage.body_drive + derived.body_focus * 0.25).clamp(0.0, 1.0),
        output_trim_db: engine.final_stage.output_trim_db,
        stereo_width,
        stereo_crossfeed,
        final_saturation: (engine.final_stage.body_drive
            + derived.mass * 0.14
            + derived.strain * 0.08)
            .clamp(0.0, 1.0),
        final_asymmetry: (engine.final_stage.asymmetry + identity.baklja_edge * 0.22)
            .clamp(0.0, 1.0),
        low_mid_emphasis: (engine.final_stage.low_mid_emphasis + derived.mass * 0.15)
            .clamp(0.0, 1.0),
        chorus_enabled: engine.fx.chorus.enabled,
        chorus_mix: engine.fx.chorus.mix,
        chorus_depth: engine.fx.chorus.depth,
        chorus_rate_hz: engine.fx.chorus.rate_hz,
        reverb_enabled: engine.fx.reverb.enabled,
        reverb_mix: engine.fx.reverb.mix,
        reverb_size: engine.fx.reverb.size,
        reverb_damping: engine.fx.reverb.damping,
    }
}

fn macro_defaults_with_override(
    defaults: &mamut_patch::MacroDefaults,
    macro_id: MacroId,
    value: f32,
) -> mamut_patch::MacroDefaults {
    let mut macros = MacroState::from_defaults(defaults);
    macros.set(macro_id, value);
    mamut_patch::MacroDefaults {
        gravitacija: macros.gravitacija,
        bloom: macros.bloom,
        heat: macros.heat,
        ruin: macros.ruin,
        swarm: macros.swarm,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mamut_dsp::{DENORMAL_FLUSH_ABS, MASTER_SAFETY_CEILING};
    use mamut_field::{
        GFM_PERFORMANCE_BASELINE_SEED, GFM_PERFORMANCE_DURATION_SECONDS, GFM_V1_HEIGHT,
        GFM_V1_WIDTH, GfmHealthHistogram, GfmLattice16, GfmPerformanceGesture,
        GfmPerformanceProgram, GfmProgramId, sample_to_pcm16,
    };
    use mamut_patch::load_patch_toml;

    const MOLTEN_HORIZON: &str = include_str!("../../../patches/factory/molten-horizon.toml");
    const CATHEDRAL_BLOOM: &str = include_str!("../../../patches/factory/cathedral-bloom.toml");
    const EMBER_VAULT: &str = include_str!("../../../patches/factory/ember-vault.toml");
    const FURNACE_CHOIR: &str = include_str!("../../../patches/factory/furnace-choir.toml");
    const GLASS_TIDE: &str = include_str!("../../../patches/factory/glass-tide.toml");
    const GRANITE_PLAIN: &str = include_str!("../../../patches/factory/granite-plain.toml");
    const GRAVITY_WAKE: &str = include_str!("../../../patches/factory/gravity-wake.toml");
    const RAZOR_THAW: &str = include_str!("../../../patches/factory/razor-thaw.toml");
    const SAWYER_REZZ: &str = include_str!("../../../patches/factory/sawyer-rezz.toml");
    const GFM_TEST_RATE_HZ: u32 = 1_000;
    const ENGINE_LAYER_TEST_RATE_HZ: u32 = 8_000;
    const ENGINE_LAYER_RECOVERY_RATE_HZ: u32 = 48_000;
    const ENGINE_LAYER_BLOCK_FRAMES: usize = 256;
    const FACTORY_PATCHES: [(&str, &str); 9] = [
        ("cathedral-bloom", CATHEDRAL_BLOOM),
        ("ember-vault", EMBER_VAULT),
        ("furnace-choir", FURNACE_CHOIR),
        ("glass-tide", GLASS_TIDE),
        ("granite-plain", GRANITE_PLAIN),
        ("gravity-wake", GRAVITY_WAKE),
        ("molten-horizon", MOLTEN_HORIZON),
        ("razor-thaw", RAZOR_THAW),
        ("sawyer-rezz", SAWYER_REZZ),
    ];

    #[derive(Clone, Copy)]
    struct GfmDryRunStats {
        finite: bool,
        peak_abs: f32,
        max_rupture_count: usize,
        health: GfmHealthHistogram,
        probe_position: (usize, usize),
        frames: usize,
        final_frame_index: usize,
    }

    #[derive(Debug, Clone, Copy)]
    struct EngineLayerRenderStats {
        finite: bool,
        rms: f32,
        peak_abs: f32,
        gfm_selection: GfmVoiceProgramSelection,
        gfm_program_id: Option<GfmProgramId>,
        gfm_diagnostics: Option<GfmDiagnostics>,
    }

    fn fixture_engine() -> Engine {
        let patch = load_patch_toml(MOLTEN_HORIZON).expect("fixture must parse");
        Engine::new(EngineConfig::default(), patch).expect("fixture must validate")
    }

    #[derive(Debug, Clone, Copy)]
    struct OutputStats {
        left_mean: f64,
        right_mean: f64,
        peak_abs: f32,
    }

    fn render_post_warmup_stats(patch_source: &str, note: Option<u8>) -> OutputStats {
        const SAMPLE_RATE_HZ: usize = 48_000;
        const BLOCK_FRAMES: usize = 240;
        const TOTAL_FRAMES: usize = SAMPLE_RATE_HZ * 3;
        const WARMUP_FRAMES: usize = SAMPLE_RATE_HZ;

        let patch = load_patch_toml(patch_source).expect("factory patch must parse");
        let mut engine = Engine::new(
            EngineConfig {
                sample_rate_hz: SAMPLE_RATE_HZ as f32,
                max_block_frames: BLOCK_FRAMES,
                voice_count: 6,
            },
            patch,
        )
        .expect("engine must validate");
        let note_on = [Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: note.unwrap_or(60),
                velocity: 0.9,
            },
        }];
        let mut left = [0.0_f32; BLOCK_FRAMES];
        let mut right = [0.0_f32; BLOCK_FRAMES];
        let mut frame_cursor = 0_usize;
        let mut left_sum = 0.0_f64;
        let mut right_sum = 0.0_f64;
        let mut peak_abs = 0.0_f32;
        let mut measured_frames = 0_usize;

        while frame_cursor < TOTAL_FRAMES {
            let frame_count = (TOTAL_FRAMES - frame_cursor).min(BLOCK_FRAMES);
            let note_events = if frame_cursor == 0 && note.is_some() {
                &note_on[..]
            } else {
                &[]
            };
            engine.process_block(ProcessBlock {
                frame_count,
                note_events,
                controller_events: &[],
                macro_state: None,
                output: Some(StereoBlockMut::new(&mut left, &mut right)),
            });

            for frame in 0..frame_count {
                if frame_cursor + frame >= WARMUP_FRAMES {
                    let left_sample = left[frame];
                    let right_sample = right[frame];
                    assert!(left_sample.is_finite());
                    assert!(right_sample.is_finite());
                    left_sum += left_sample as f64;
                    right_sum += right_sample as f64;
                    peak_abs = peak_abs.max(left_sample.abs().max(right_sample.abs()));
                    measured_frames += 1;
                }
            }

            frame_cursor += frame_count;
        }

        OutputStats {
            left_mean: left_sum / measured_frames as f64,
            right_mean: right_sum / measured_frames as f64,
            peak_abs,
        }
    }

    #[test]
    fn gfm_field_voice_matches_direct_field_pcm() {
        for program_id in GfmProgramId::ALL_PERFORMANCE {
            let (adapter_signature, adapter_stats) = render_gfm_adapter_signature(program_id);
            let (direct_signature, direct_stats) = render_direct_field_signature(program_id);

            assert_eq!(adapter_signature, direct_signature);
            assert_eq!(adapter_stats.probe_position, direct_stats.probe_position);
            assert_eq!(adapter_stats.frames, direct_stats.frames);
            assert_eq!(adapter_stats.final_frame_index, adapter_stats.frames);
            assert_eq!(
                adapter_stats.max_rupture_count,
                direct_stats.max_rupture_count
            );
            assert_eq!(adapter_stats.health, direct_stats.health);
        }
    }

    #[test]
    fn gfm_field_voice_dry_run_is_finite_bounded_and_recovery_safe() {
        for program_id in GfmProgramId::ALL_PERFORMANCE {
            let (_, stats) = render_gfm_adapter_signature(program_id);

            assert!(stats.finite);
            assert!(stats.peak_abs <= 1.0);
            match program_id {
                GfmProgramId::HorizontPerformance | GfmProgramId::PecPerformance => {
                    assert_eq!(stats.max_rupture_count, 0);
                    assert_eq!(stats.health.suspect, 0);
                    assert_eq!(stats.health.quarantined, 0);
                }
                GfmProgramId::BakljaPerformance => {
                    let recovered = stats.health.healthy + stats.health.recovering;
                    assert!(stats.max_rupture_count > 0);
                    assert!(stats.max_rupture_count <= (GFM_V1_WIDTH * GFM_V1_HEIGHT * 2) / 5);
                    assert!(recovered >= (GFM_V1_WIDTH * GFM_V1_HEIGHT * 3) / 4);
                }
            }
        }
    }

    #[test]
    fn gfm_field_voice_live_pressure_changes_excitation_and_stays_bounded() {
        for program_id in GfmProgramId::ALL_PERFORMANCE {
            let (plain_signature, plain_stats) =
                render_gfm_adapter_signature_with_live_pressure(program_id, 0.0, 4_096);
            let (pressured_signature, pressured_stats) =
                render_gfm_adapter_signature_with_live_pressure(program_id, 0.85, 4_096);

            assert_ne!(pressured_signature, plain_signature);
            assert!(plain_stats.finite);
            assert!(pressured_stats.finite);
            assert!(pressured_stats.peak_abs <= 1.0);
        }
    }

    #[test]
    fn gfm_field_voice_mono_block_matches_sample_step_and_direct_field() {
        for program_id in GfmProgramId::ALL_PERFORMANCE {
            let (sample_signature, sample_stats) = render_gfm_adapter_signature(program_id);
            let (block_signature, block_stats) = render_gfm_block_signature(program_id);
            let (direct_signature, direct_stats) = render_direct_field_signature(program_id);

            assert_eq!(block_signature, sample_signature);
            assert_eq!(block_signature, direct_signature);
            assert_eq!(block_stats.probe_position, sample_stats.probe_position);
            assert_eq!(block_stats.probe_position, direct_stats.probe_position);
            assert_eq!(block_stats.final_frame_index, block_stats.frames);
            assert_eq!(
                block_stats.max_rupture_count,
                sample_stats.max_rupture_count
            );
            assert_eq!(block_stats.health, direct_stats.health);
        }
    }

    #[test]
    fn gfm_field_voice_stereo_block_is_dual_mono() {
        let mut voice = GfmFieldVoice::new(
            GfmProgramId::BakljaPerformance,
            GFM_PERFORMANCE_BASELINE_SEED,
            GFM_TEST_RATE_HZ,
        );
        let mut left = [0.0_f32; 257];
        let mut right = [0.0_f32; 257];

        voice.render_stereo_block(&mut left, &mut right);

        assert_eq!(voice.frame_index(), 257);
        for (left, right) in left.iter().zip(right.iter()) {
            assert_eq!(left.to_bits(), right.to_bits());
            assert!(left.is_finite());
        }
    }

    #[test]
    fn gfm_field_voice_chunking_is_invariant() {
        for program_id in GfmProgramId::ALL_PERFORMANCE {
            let (one_block_signature, one_block_stats) = render_gfm_block_signature(program_id);

            for chunk_size in [17, 64, 251] {
                let (chunked_signature, chunked_stats) =
                    render_gfm_chunked_signature(program_id, chunk_size);
                assert_eq!(chunked_signature, one_block_signature);
                assert_eq!(chunked_stats.frames, one_block_stats.frames);
                assert_eq!(
                    chunked_stats.final_frame_index,
                    one_block_stats.final_frame_index
                );
                assert_eq!(
                    chunked_stats.max_rupture_count,
                    one_block_stats.max_rupture_count
                );
                assert_eq!(chunked_stats.health, one_block_stats.health);
            }
        }
    }

    #[test]
    fn gfm_program_selection_is_deterministic_for_factory_patches() {
        let mut saw_horizont = false;
        let mut saw_pec = false;
        let mut saw_baklja = false;

        for (_, patch_source) in FACTORY_PATCHES {
            let patch = load_patch_toml(patch_source).expect("factory patch must parse");
            let left = select_gfm_program_for_patch(&patch);
            let right = select_gfm_program_for_patch(&patch);

            assert_eq!(left, right);
            assert!(left.best_score() >= 0.15);
            match left
                .program_id
                .expect("factory patch should select a GFM program")
            {
                GfmProgramId::HorizontPerformance => saw_horizont = true,
                GfmProgramId::PecPerformance => saw_pec = true,
                GfmProgramId::BakljaPerformance => saw_baklja = true,
            }
        }

        assert!(saw_horizont);
        assert!(saw_pec);
        assert!(saw_baklja);
    }

    #[test]
    fn selected_factory_gfm_programs_render_finite_and_recovery_safe() {
        for (_, patch_source) in FACTORY_PATCHES {
            let patch = load_patch_toml(patch_source).expect("factory patch must parse");
            let decision = GfmFieldVoice::from_patch(&patch, gfm_patch_voice_test_config());
            let selection = decision.selection();
            let program_id = selection
                .program_id
                .expect("factory patch should select a GFM program");
            let (_, stats) = render_gfm_voice_block_signature(
                decision
                    .into_voice()
                    .expect("selected patch should build a GFM voice"),
            );

            assert!(stats.finite);
            assert!(stats.peak_abs <= 1.0);
            match program_id {
                GfmProgramId::HorizontPerformance | GfmProgramId::PecPerformance => {
                    assert_eq!(stats.max_rupture_count, 0);
                }
                GfmProgramId::BakljaPerformance => {
                    let recovered = stats.health.healthy + stats.health.recovering;
                    assert!(stats.max_rupture_count > 0);
                    assert!(recovered >= (GFM_V1_WIDTH * GFM_V1_HEIGHT * 3) / 4);
                }
            }
        }
    }

    #[test]
    fn gfm_patch_voice_factory_matches_direct_selection_render() {
        for (_, patch_source) in FACTORY_PATCHES {
            let patch = load_patch_toml(patch_source).expect("factory patch must parse");
            let selection = select_gfm_program_for_patch(&patch);
            let program_id = selection
                .program_id
                .expect("factory patch should select a GFM program");
            let decision = GfmFieldVoice::from_patch(&patch, gfm_patch_voice_test_config());
            let (factory_signature, factory_stats) = render_gfm_voice_block_signature(
                decision
                    .into_voice()
                    .expect("selected patch should build a GFM voice"),
            );
            let (direct_signature, direct_stats) = render_gfm_selected_patch_signature(program_id);

            assert_eq!(factory_signature, direct_signature);
            assert_eq!(factory_stats.probe_position, direct_stats.probe_position);
            assert_eq!(factory_stats.frames, direct_stats.frames);
            assert_eq!(
                factory_stats.final_frame_index,
                direct_stats.final_frame_index
            );
            assert_eq!(
                factory_stats.max_rupture_count,
                direct_stats.max_rupture_count
            );
            assert_eq!(factory_stats.health, direct_stats.health);
        }
    }

    #[test]
    fn gfm_patch_voice_factory_is_deterministic_for_patch_seed_and_rate() {
        for (_, patch_source) in FACTORY_PATCHES {
            let patch = load_patch_toml(patch_source).expect("factory patch must parse");
            let left = GfmFieldVoice::from_patch(&patch, gfm_patch_voice_test_config());
            let right = GfmFieldVoice::from_patch(&patch, gfm_patch_voice_test_config());

            assert_eq!(left.config(), right.config());
            assert_eq!(left.selection(), right.selection());

            let (left_signature, left_stats) = render_gfm_voice_block_signature(
                left.into_voice()
                    .expect("selected patch should build a GFM voice"),
            );
            let (right_signature, right_stats) = render_gfm_voice_block_signature(
                right
                    .into_voice()
                    .expect("selected patch should build a GFM voice"),
            );

            assert_eq!(left_signature, right_signature);
            assert_eq!(left_stats.frames, right_stats.frames);
            assert_eq!(left_stats.final_frame_index, right_stats.final_frame_index);
            assert_eq!(left_stats.max_rupture_count, right_stats.max_rupture_count);
            assert_eq!(left_stats.health, right_stats.health);
        }
    }

    #[test]
    fn gfm_patch_voice_factory_disables_low_score_patch() {
        let patch = low_score_gfm_patch();
        let decision = GfmFieldVoice::from_patch(&patch, gfm_patch_voice_test_config());

        assert!(!decision.is_enabled());
        assert_eq!(decision.selection().program_id, None);
        assert!(decision.selection().best_score() < GfmVoiceProgramSelection::MIN_PROGRAM_SCORE);
        assert!(decision.into_voice().is_none());
    }

    #[test]
    fn gfm_layer_defaults_to_disabled_snapshot() {
        let engine = fixture_engine();
        let snapshot = engine.snapshot();

        assert_eq!(engine.gfm_layer_mode(), GfmLayerMode::Disabled);
        assert_eq!(snapshot.gfm_layer.mode, GfmLayerMode::Disabled);
        assert_eq!(
            snapshot.gfm_layer.selection,
            GfmVoiceProgramSelection::default()
        );
        assert_eq!(snapshot.gfm_layer.active_program_id, None);
        assert!(snapshot.gfm_layer.diagnostics.is_none());
        assert_eq!(snapshot.gfm_layer.amount, 0.0);
        assert_eq!(snapshot.gfm_layer.pressure, 0.0);
        assert_eq!(snapshot.gfm_layer.effective_amount, 0.0);
        assert!(engine.gfm_layer_diagnostics().is_none());
    }

    #[test]
    fn gfm_layer_gate_without_aftertouch_does_not_auto_arm() {
        let patch = load_patch_toml(CATHEDRAL_BLOOM).expect("factory patch must parse");
        let mut engine = Engine::new(
            EngineConfig {
                sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
                max_block_frames: ENGINE_LAYER_TEST_RATE_HZ as usize,
                voice_count: 6,
            },
            patch,
        )
        .expect("engine must validate");

        engine.process_block(ProcessBlock {
            frame_count: 8,
            note_events: &[],
            controller_events: &[Scheduled {
                frame_offset: 0,
                event: ControllerEvent::GfmLayerAmount { amount: 0.65 },
            }],
            macro_state: None,
            output: None,
        });
        let snapshot = engine.snapshot();

        assert_eq!(snapshot.gfm_layer.mode, GfmLayerMode::Disabled);
        assert_eq!(snapshot.gfm_layer.amount, 0.0);
        assert_eq!(snapshot.gfm_layer.pressure, 0.65);
        assert_eq!(snapshot.gfm_layer.effective_amount, 0.0);
        assert_eq!(snapshot.gfm_layer.active_program_id, None);
        assert!(snapshot.gfm_layer.diagnostics.is_none());
    }

    #[test]
    fn gfm_layer_gate_zero_disarms_auto_armed_layer_after_release() {
        let patch = load_patch_toml(CATHEDRAL_BLOOM).expect("factory patch must parse");
        let mut engine = Engine::new(
            EngineConfig {
                sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
                max_block_frames: ENGINE_LAYER_TEST_RATE_HZ as usize,
                voice_count: 6,
            },
            patch,
        )
        .expect("engine must validate");

        engine.process_block(ProcessBlock {
            frame_count: 8,
            note_events: &[],
            controller_events: &[
                Scheduled {
                    frame_offset: 0,
                    event: ControllerEvent::GfmLayerAmount { amount: 0.65 },
                },
                Scheduled {
                    frame_offset: 0,
                    event: ControllerEvent::ChannelAftertouch { pressure: 0.72 },
                },
            ],
            macro_state: None,
            output: None,
        });
        assert!(matches!(
            engine.snapshot().gfm_layer.mode,
            GfmLayerMode::Enabled { .. }
        ));

        engine.process_block(ProcessBlock {
            frame_count: 8,
            note_events: &[],
            controller_events: &[Scheduled {
                frame_offset: 0,
                event: ControllerEvent::GfmLayerAmount { amount: 0.0 },
            }],
            macro_state: None,
            output: None,
        });
        engine.process_block(ProcessBlock {
            frame_count: smoothing_sample_count(
                ENGINE_LAYER_TEST_RATE_HZ as f32,
                GFM_LAYER_MIX_RELEASE_MS,
            ) + 8,
            note_events: &[],
            controller_events: &[],
            macro_state: None,
            output: None,
        });
        let snapshot = engine.snapshot();

        assert_eq!(snapshot.gfm_layer.mode, GfmLayerMode::Disabled);
        assert_eq!(snapshot.gfm_layer.pressure, 0.0);
        assert_eq!(snapshot.gfm_layer.effective_amount, 0.0);
        assert_eq!(snapshot.gfm_layer.active_program_id, None);
        assert!(snapshot.gfm_layer.diagnostics.is_none());
    }

    #[test]
    fn gfm_layer_amount_zero_keeps_manual_gui_enabled_layer_ready() {
        let patch = load_patch_toml(CATHEDRAL_BLOOM).expect("factory patch must parse");
        let mut engine = Engine::new(
            EngineConfig {
                sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
                max_block_frames: ENGINE_LAYER_TEST_RATE_HZ as usize,
                voice_count: 6,
            },
            patch,
        )
        .expect("engine must validate");
        engine.set_gfm_layer_mode(GfmLayerMode::Enabled {
            seed: GFM_PERFORMANCE_BASELINE_SEED,
        });

        engine.process_block(ProcessBlock {
            frame_count: 8,
            note_events: &[],
            controller_events: &[Scheduled {
                frame_offset: 0,
                event: ControllerEvent::GfmLayerAmount { amount: 0.0 },
            }],
            macro_state: None,
            output: None,
        });
        let snapshot = engine.snapshot();

        assert_eq!(
            snapshot.gfm_layer.mode,
            GfmLayerMode::Enabled {
                seed: GFM_PERFORMANCE_BASELINE_SEED
            }
        );
        assert_eq!(snapshot.gfm_layer.amount, 0.0);
        assert_eq!(snapshot.gfm_layer.effective_amount, 0.0);
        assert!(snapshot.gfm_layer.active_program_id.is_some());
        assert!(snapshot.gfm_layer.diagnostics.is_some());
    }

    #[test]
    fn gfm_layer_momentary_gate_smooths_in_and_out() {
        let patch = load_patch_toml(CATHEDRAL_BLOOM).expect("factory patch must parse");
        let mut engine = Engine::new(
            EngineConfig {
                sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
                max_block_frames: ENGINE_LAYER_TEST_RATE_HZ as usize,
                voice_count: 6,
            },
            patch,
        )
        .expect("engine must validate");
        let controller_events = [
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::GfmLayerAmount { amount: 1.0 },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::ChannelAftertouch { pressure: 1.0 },
            },
        ];

        engine.process_block(ProcessBlock {
            frame_count: 1,
            note_events: &[],
            controller_events: &controller_events,
            macro_state: None,
            output: None,
        });
        let first = engine.snapshot();
        assert!(first.gfm_layer.effective_amount > 0.0);
        assert!(
            first.gfm_layer.effective_amount < 0.01,
            "first smoothed GFM amount should be tiny, got {}",
            first.gfm_layer.effective_amount
        );

        engine.process_block(ProcessBlock {
            frame_count: smoothing_sample_count(
                ENGINE_LAYER_TEST_RATE_HZ as f32,
                GFM_LAYER_MIX_ATTACK_MS,
            ),
            note_events: &[],
            controller_events: &[],
            macro_state: None,
            output: None,
        });
        let open = engine.snapshot();
        assert!(open.gfm_layer.effective_amount > 0.95);

        engine.process_block(ProcessBlock {
            frame_count: 1,
            note_events: &[],
            controller_events: &[Scheduled {
                frame_offset: 0,
                event: ControllerEvent::GfmLayerAmount { amount: 0.0 },
            }],
            macro_state: None,
            output: None,
        });
        let first_release = engine.snapshot();
        assert!(matches!(
            first_release.gfm_layer.mode,
            GfmLayerMode::Enabled { .. }
        ));
        assert!(first_release.gfm_layer.effective_amount > 0.90);

        engine.process_block(ProcessBlock {
            frame_count: smoothing_sample_count(
                ENGINE_LAYER_TEST_RATE_HZ as f32,
                GFM_LAYER_MIX_RELEASE_MS,
            ) + 8,
            note_events: &[],
            controller_events: &[],
            macro_state: None,
            output: None,
        });
        let closed = engine.snapshot();
        assert_eq!(closed.gfm_layer.mode, GfmLayerMode::Disabled);
        assert_eq!(closed.gfm_layer.effective_amount, 0.0);
    }

    #[test]
    fn gfm_layer_midi_controls_can_reintroduce_layer_without_gui_enable() {
        let patch = load_patch_toml(CATHEDRAL_BLOOM).expect("factory patch must parse");
        let frames = ENGINE_LAYER_TEST_RATE_HZ as usize * 2;
        let note_events =
            engine_layer_note_on_events(select_gfm_program_for_patch(&patch).program_id);
        let aftertouch_only = [
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::GfmLayerAmount { amount: 0.0 },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::ChannelAftertouch { pressure: 0.72 },
            },
        ];
        let momentary_gfm = [
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::GfmLayerAmount { amount: 1.0 },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::ChannelAftertouch { pressure: 0.72 },
            },
        ];
        let mut baseline_engine = Engine::new(
            EngineConfig {
                sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
                max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
                voice_count: 6,
            },
            patch.clone(),
        )
        .expect("engine must validate");
        let (baseline_signature, _) = render_engine_layer_signature_with_notes_and_controllers(
            &mut baseline_engine,
            frames,
            note_events,
            &aftertouch_only,
        );
        let mut engine = Engine::new(
            EngineConfig {
                sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
                max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
                voice_count: 6,
            },
            patch,
        )
        .expect("engine must validate");

        let (layer_signature, layer_stats) =
            render_engine_layer_signature_with_notes_and_controllers(
                &mut engine,
                frames,
                note_events,
                &momentary_gfm,
            );
        let snapshot = engine.snapshot();

        assert_ne!(layer_signature, baseline_signature);
        assert_eq!(
            snapshot.gfm_layer.mode,
            GfmLayerMode::Enabled {
                seed: DEFAULT_GFM_LAYER_SEED
            }
        );
        assert!(snapshot.gfm_layer.effective_amount > 0.0);
        assert!(layer_stats.finite);
        assert!(layer_stats.gfm_program_id.is_some());
    }

    #[test]
    fn gfm_layer_low_score_enabled_patch_matches_baseline_engine_render() {
        let patch = low_score_gfm_patch();
        let frames = ENGINE_LAYER_TEST_RATE_HZ as usize * 2;
        let (baseline_signature, baseline_stats) =
            render_engine_layer_signature(patch.clone(), None, frames);
        let (layer_signature, layer_stats) =
            render_engine_layer_signature(patch, Some(GFM_PERFORMANCE_BASELINE_SEED), frames);

        assert_eq!(layer_signature, baseline_signature);
        assert_eq!(layer_stats.gfm_selection.program_id, None);
        assert_eq!(layer_stats.gfm_program_id, None);
        assert!(layer_stats.gfm_diagnostics.is_none());
        assert_eq!(layer_stats.finite, baseline_stats.finite);
        assert_eq!(
            layer_stats.peak_abs.to_bits(),
            baseline_stats.peak_abs.to_bits()
        );
    }

    #[test]
    fn gfm_layer_enabled_without_momentary_controls_matches_baseline_engine_render() {
        for patch_source in [CATHEDRAL_BLOOM, EMBER_VAULT, RAZOR_THAW] {
            let patch = load_patch_toml(patch_source).expect("factory patch must parse");
            let frames = ENGINE_LAYER_TEST_RATE_HZ as usize * 2;
            let (baseline_signature, baseline_stats) =
                render_engine_layer_signature(patch.clone(), None, frames);
            let (armed_signature, armed_stats) =
                render_engine_layer_signature(patch, Some(GFM_PERFORMANCE_BASELINE_SEED), frames);

            assert_eq!(armed_signature, baseline_signature);
            assert_eq!(
                armed_stats.peak_abs.to_bits(),
                baseline_stats.peak_abs.to_bits()
            );
            assert!(armed_stats.gfm_program_id.is_some());
            assert!(armed_stats.gfm_diagnostics.is_some());
        }
    }

    #[test]
    fn gfm_layer_enable_clears_stale_momentary_controls_without_clearing_aftertouch_macros() {
        let patch = load_patch_toml(CATHEDRAL_BLOOM).expect("factory patch must parse");
        let mut engine = Engine::new(
            EngineConfig {
                sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
                max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
                voice_count: 6,
            },
            patch,
        )
        .expect("engine must validate");
        let controller_events = [
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::GfmLayerAmount { amount: 1.0 },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::ChannelAftertouch { pressure: 0.72 },
            },
        ];

        engine.process_block(ProcessBlock {
            frame_count: 8,
            note_events: &[],
            controller_events: &controller_events,
            macro_state: None,
            output: None,
        });
        let before = engine.snapshot();
        assert_eq!(before.gfm_layer.amount, 0.72);
        assert_eq!(before.gfm_layer.pressure, 1.0);
        assert!(before.effective_macros.gravitacija > before.live_macros.gravitacija);

        engine.set_gfm_layer_mode(GfmLayerMode::Enabled {
            seed: GFM_PERFORMANCE_BASELINE_SEED,
        });
        let after = engine.snapshot();

        assert_eq!(after.gfm_layer.amount, 0.0);
        assert_eq!(after.gfm_layer.pressure, 0.0);
        assert_eq!(after.gfm_layer.effective_amount, 0.0);
        assert_eq!(
            after.effective_macros.gravitacija.to_bits(),
            before.effective_macros.gravitacija.to_bits()
        );
    }

    #[test]
    fn gfm_layer_changes_enabled_patch_when_momentary_pressure_is_present() {
        for patch_source in [CATHEDRAL_BLOOM, EMBER_VAULT, RAZOR_THAW] {
            let patch = load_patch_toml(patch_source).expect("factory patch must parse");
            let frames = ENGINE_LAYER_TEST_RATE_HZ as usize * 2;
            let (baseline_signature, _) =
                render_engine_layer_signature(patch.clone(), None, frames);
            let note_events =
                engine_layer_note_on_events(select_gfm_program_for_patch(&patch).program_id);
            let mut engine = Engine::new(
                EngineConfig {
                    sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
                    max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
                    voice_count: 6,
                },
                patch,
            )
            .expect("engine must validate");
            engine.set_gfm_layer_mode(GfmLayerMode::Enabled {
                seed: GFM_PERFORMANCE_BASELINE_SEED,
            });
            let controller_events = [
                Scheduled {
                    frame_offset: 0,
                    event: ControllerEvent::GfmLayerAmount { amount: 1.0 },
                },
                Scheduled {
                    frame_offset: 0,
                    event: ControllerEvent::ChannelAftertouch { pressure: 0.72 },
                },
            ];
            let (layer_signature, layer_stats) =
                render_engine_layer_signature_with_notes_and_controllers(
                    &mut engine,
                    frames,
                    note_events,
                    &controller_events,
                );

            assert_ne!(layer_signature, baseline_signature);
            assert!(layer_stats.finite);
            assert!(layer_stats.rms > 0.0);
            assert!(
                layer_stats.peak_abs <= MASTER_SAFETY_CEILING,
                "stats={layer_stats:?}"
            );
            assert!(layer_stats.gfm_program_id.is_some());
        }
    }

    #[test]
    fn gfm_layer_disabled_after_enabled_matches_baseline_engine_render() {
        let patch = load_patch_toml(CATHEDRAL_BLOOM).expect("factory patch must parse");
        let frames = ENGINE_LAYER_TEST_RATE_HZ as usize * 2;
        let (baseline_signature, baseline_stats) =
            render_engine_layer_signature(patch.clone(), None, frames);
        let mut engine = Engine::new(
            EngineConfig {
                sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
                max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
                voice_count: 6,
            },
            patch,
        )
        .expect("engine must validate");

        engine.set_gfm_layer_mode(GfmLayerMode::Enabled {
            seed: GFM_PERFORMANCE_BASELINE_SEED,
        });
        engine.set_gfm_layer_mode(GfmLayerMode::Disabled);
        let (disabled_signature, disabled_stats) =
            render_engine_layer_signature_for_engine(&mut engine, frames);

        assert_eq!(disabled_signature, baseline_signature);
        assert_eq!(disabled_stats.gfm_selection, baseline_stats.gfm_selection);
        assert_eq!(disabled_stats.gfm_program_id, None);
        assert!(disabled_stats.gfm_diagnostics.is_none());
        assert_eq!(
            disabled_stats.peak_abs.to_bits(),
            baseline_stats.peak_abs.to_bits()
        );
    }

    #[test]
    fn gfm_layer_amount_zero_mutes_layer_without_disabling_voice() {
        let patch = load_patch_toml(CATHEDRAL_BLOOM).expect("factory patch must parse");
        let frames = ENGINE_LAYER_TEST_RATE_HZ as usize * 2;
        let note_events =
            engine_layer_note_on_events(select_gfm_program_for_patch(&patch).program_id);
        let controller_events = [
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::GfmLayerAmount { amount: 0.0 },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::ChannelAftertouch { pressure: 0.85 },
            },
        ];
        let mut baseline_engine = Engine::new(
            EngineConfig {
                sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
                max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
                voice_count: 6,
            },
            patch.clone(),
        )
        .expect("engine must validate");
        let (baseline_signature, baseline_stats) =
            render_engine_layer_signature_with_notes_and_controllers(
                &mut baseline_engine,
                frames,
                note_events,
                &controller_events,
            );
        let mut engine = Engine::new(
            EngineConfig {
                sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
                max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
                voice_count: 6,
            },
            patch,
        )
        .expect("engine must validate");
        engine.set_gfm_layer_mode(GfmLayerMode::Enabled {
            seed: GFM_PERFORMANCE_BASELINE_SEED,
        });

        let (muted_signature, muted_stats) =
            render_engine_layer_signature_with_notes_and_controllers(
                &mut engine,
                frames,
                note_events,
                &controller_events,
            );
        let snapshot = engine.snapshot();

        assert_eq!(muted_signature, baseline_signature);
        assert_eq!(
            muted_stats.peak_abs.to_bits(),
            baseline_stats.peak_abs.to_bits()
        );
        assert_eq!(snapshot.gfm_layer.amount, 0.85);
        assert_eq!(snapshot.gfm_layer.pressure, 0.0);
        assert_eq!(snapshot.gfm_layer.effective_amount, 0.0);
        assert_eq!(
            snapshot.gfm_layer.active_program_id,
            Some(GfmProgramId::HorizontPerformance)
        );
        assert!(snapshot.gfm_layer.diagnostics.is_some());
    }

    #[test]
    fn gfm_layer_is_finite_bounded_and_recovery_safe() {
        let short_frames = ENGINE_LAYER_TEST_RATE_HZ as usize * 2;

        for patch_source in [CATHEDRAL_BLOOM, EMBER_VAULT] {
            let patch = load_patch_toml(patch_source).expect("factory patch must parse");
            let (_, stats) = render_engine_layer_signature(
                patch,
                Some(GFM_PERFORMANCE_BASELINE_SEED),
                short_frames,
            );
            let diagnostics = stats
                .gfm_diagnostics
                .expect("selected patch should keep GFM diagnostics");

            assert!(stats.finite);
            assert!(stats.peak_abs <= MASTER_SAFETY_CEILING, "stats={stats:?}");
            assert_eq!(diagnostics.max_rupture_count, 0);
        }

        let patch = load_patch_toml(RAZOR_THAW).expect("factory patch must parse");
        let frames = ENGINE_LAYER_RECOVERY_RATE_HZ as usize * GFM_PERFORMANCE_DURATION_SECONDS;
        let (_, stats) = render_engine_layer_signature_at_rate(
            patch,
            Some(GFM_PERFORMANCE_BASELINE_SEED),
            ENGINE_LAYER_RECOVERY_RATE_HZ,
            frames,
        );
        let diagnostics = stats
            .gfm_diagnostics
            .expect("selected patch should keep GFM diagnostics");
        let recovered = diagnostics.health.healthy + diagnostics.health.recovering;

        assert!(stats.finite);
        assert!(stats.peak_abs <= MASTER_SAFETY_CEILING, "stats={stats:?}");
        assert_eq!(stats.gfm_program_id, Some(GfmProgramId::BakljaPerformance));
        assert!(diagnostics.max_rupture_count > 0);
        assert!(
            recovered >= (GFM_V1_WIDTH * GFM_V1_HEIGHT * 3) / 4,
            "diagnostics={diagnostics:?}"
        );
    }

    #[test]
    fn gfm_layer_snapshot_exposes_mode_selection_program_and_diagnostics() {
        let patch = load_patch_toml(EMBER_VAULT).expect("factory patch must parse");
        let mut engine = Engine::new(
            EngineConfig {
                sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
                max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
                voice_count: 6,
            },
            patch,
        )
        .expect("engine must validate");
        let mode = GfmLayerMode::Enabled {
            seed: GFM_PERFORMANCE_BASELINE_SEED,
        };

        let selection = engine.set_gfm_layer_mode(mode);
        let snapshot = engine.snapshot();

        assert_eq!(engine.gfm_layer_mode(), mode);
        assert_eq!(snapshot.gfm_layer.mode, mode);
        assert_eq!(snapshot.gfm_layer.selection, selection);
        assert_eq!(
            snapshot.gfm_layer.active_program_id,
            Some(GfmProgramId::PecPerformance)
        );
        assert!(snapshot.gfm_layer.diagnostics.is_some());
        assert_eq!(snapshot.gfm_layer.amount, 0.0);
        assert_eq!(snapshot.gfm_layer.pressure, 0.0);
        assert_eq!(snapshot.gfm_layer.effective_amount, 0.0);
        assert_eq!(
            snapshot.gfm_layer.diagnostics,
            engine.gfm_layer_diagnostics()
        );
    }

    #[test]
    fn gfm_layer_rebuilds_when_patch_loads() {
        let first_patch = load_patch_toml(CATHEDRAL_BLOOM).expect("factory patch must parse");
        let second_patch = load_patch_toml(RAZOR_THAW).expect("factory patch must parse");
        let mut engine = Engine::new(
            EngineConfig {
                sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
                max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
                voice_count: 6,
            },
            first_patch,
        )
        .expect("engine must validate");

        let first_selection = engine.set_gfm_layer_mode(GfmLayerMode::Enabled {
            seed: GFM_PERFORMANCE_BASELINE_SEED,
        });
        assert_eq!(
            first_selection.program_id,
            Some(GfmProgramId::HorizontPerformance)
        );

        engine
            .load_patch(second_patch)
            .expect("patch must validate");

        assert_eq!(
            engine.snapshot().gfm_layer.selection.program_id,
            Some(GfmProgramId::BakljaPerformance)
        );
        assert_eq!(
            engine.snapshot().gfm_layer.active_program_id,
            Some(GfmProgramId::BakljaPerformance)
        );
    }

    #[test]
    fn bcs_layer_defaults_to_disabled_snapshot() {
        let engine = fixture_engine();
        let snapshot = engine.snapshot();

        assert_eq!(engine.bcs_layer_mode(), BcsLayerMode::Disabled);
        assert_eq!(snapshot.bcs_layer, BcsLayerSnapshot::default());
        assert!(!snapshot.bcs_layer.enabled);
        assert_eq!(snapshot.bcs_layer.amount, 0.0);
        assert_eq!(snapshot.bcs_layer.effective_amount, 0.0);
    }

    #[test]
    fn bcs_layer_disabled_matches_baseline_engine_render() {
        let patch = load_patch_toml(CATHEDRAL_BLOOM).expect("factory patch must parse");
        let frames = ENGINE_LAYER_TEST_RATE_HZ as usize * 2;
        let (baseline_signature, baseline_stats) =
            render_engine_layer_signature(patch.clone(), None, frames);
        let mut engine = Engine::new(
            EngineConfig {
                sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
                max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
                voice_count: 6,
            },
            patch,
        )
        .expect("engine must validate");
        engine.set_bcs_layer_mode(BcsLayerMode::Disabled);

        let (disabled_signature, disabled_stats) =
            render_engine_layer_signature_for_engine(&mut engine, frames);

        assert_eq!(disabled_signature, baseline_signature);
        assert_eq!(
            disabled_stats.peak_abs.to_bits(),
            baseline_stats.peak_abs.to_bits()
        );
        assert_eq!(engine.snapshot().bcs_layer, BcsLayerSnapshot::default());
    }

    #[test]
    fn bcs_layer_mode_enabled_is_silent_until_playable_controls_open() {
        let patch = load_patch_toml(CATHEDRAL_BLOOM).expect("factory patch must parse");
        let frames = ENGINE_LAYER_TEST_RATE_HZ as usize * 2;
        let (baseline_signature, _) = render_engine_layer_signature(patch.clone(), None, frames);
        let mut engine = Engine::new(
            EngineConfig {
                sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
                max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
                voice_count: 6,
            },
            patch,
        )
        .expect("engine must validate");
        let layer_snapshot = engine.set_bcs_layer_mode(BcsLayerMode::Enabled {
            scenario: BcsScenario::StableAnchor,
        });

        assert_eq!(
            layer_snapshot.active_scenario,
            Some(BcsScenario::StableAnchor)
        );
        assert_eq!(
            layer_snapshot.sample_rate_hz,
            Some(ENGINE_LAYER_TEST_RATE_HZ)
        );
        assert!(!layer_snapshot.enabled);
        assert_eq!(layer_snapshot.amount, 0.0);
        assert_eq!(layer_snapshot.effective_amount, 0.0);

        let (layer_signature, layer_stats) =
            render_engine_layer_signature_for_engine(&mut engine, frames);
        let snapshot = engine.snapshot();

        assert_eq!(layer_signature, baseline_signature);
        assert!(layer_stats.finite);
        assert!(layer_stats.rms > 0.0);
        assert!(
            layer_stats.peak_abs <= MASTER_SAFETY_CEILING,
            "stats={layer_stats:?}"
        );
        assert_eq!(snapshot.bcs_layer.mode, engine.bcs_layer_mode());
        assert_eq!(
            snapshot.bcs_layer.active_scenario,
            Some(BcsScenario::StableAnchor)
        );
        assert_eq!(snapshot.bcs_layer.unsafe_events, 0);
        assert!(!snapshot.bcs_layer.unsafe_state);
        assert!(snapshot.bcs_layer.max_state_abs < 32.0);
        assert_eq!(snapshot.bcs_layer.pitch_note, Some(48));
        assert_eq!(snapshot.bcs_layer.effective_amount, 0.0);
    }

    #[test]
    fn bcs_layer_sw9_enable_with_zero_s9_amount_stays_silent() {
        let patch = load_patch_toml(CATHEDRAL_BLOOM).expect("factory patch must parse");
        let frames = ENGINE_LAYER_TEST_RATE_HZ as usize * 2;
        let (baseline_signature, _) = render_engine_layer_signature(patch.clone(), None, frames);
        let mut engine = Engine::new(
            EngineConfig {
                sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
                max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
                voice_count: 6,
            },
            patch,
        )
        .expect("engine must validate");
        engine.set_bcs_layer_mode(BcsLayerMode::Enabled {
            scenario: BcsScenario::StableAnchor,
        });

        let controls = [
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::BcsLayerEnabled { enabled: true },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::BcsLayerAmount { amount: 0.0 },
            },
        ];
        let note_events =
            engine_layer_note_on_events(select_gfm_program_for_patch(&engine.patch).program_id);
        let (layer_signature, _) = render_engine_layer_signature_with_notes_and_controllers(
            &mut engine,
            frames,
            note_events,
            &controls,
        );
        let snapshot = engine.snapshot();

        assert_eq!(layer_signature, baseline_signature);
        assert!(snapshot.bcs_layer.enabled);
        assert_eq!(snapshot.bcs_layer.amount, 0.0);
        assert_eq!(snapshot.bcs_layer.effective_amount, 0.0);
    }

    #[test]
    fn bcs_layer_playable_controls_change_render_but_stay_bounded() {
        let patch = load_patch_toml(CATHEDRAL_BLOOM).expect("factory patch must parse");
        let frames = ENGINE_LAYER_TEST_RATE_HZ as usize * 2;
        let (baseline_signature, _) = render_engine_layer_signature(patch.clone(), None, frames);
        let mut engine = Engine::new(
            EngineConfig {
                sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
                max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
                voice_count: 6,
            },
            patch,
        )
        .expect("engine must validate");
        engine.set_bcs_layer_mode(BcsLayerMode::Enabled {
            scenario: BcsScenario::StableAnchor,
        });

        let controls = bcs_layer_playable_controller_events(0.75);
        let note_events =
            engine_layer_note_on_events(select_gfm_program_for_patch(&engine.patch).program_id);
        let (layer_signature, layer_stats) =
            render_engine_layer_signature_with_notes_and_controllers(
                &mut engine,
                frames,
                note_events,
                &controls,
            );
        let snapshot = engine.snapshot();

        assert_ne!(layer_signature, baseline_signature);
        assert!(layer_stats.finite);
        assert!(layer_stats.rms > 0.0);
        assert!(
            layer_stats.peak_abs <= MASTER_SAFETY_CEILING,
            "stats={layer_stats:?}"
        );
        assert!(snapshot.bcs_layer.enabled);
        assert!(snapshot.bcs_layer.amount > 0.70);
        assert!(snapshot.bcs_layer.effective_amount > 0.70);
        assert_eq!(snapshot.bcs_layer.pitch_note, Some(48));
        assert_eq!(snapshot.bcs_layer.unsafe_events, 0);
        assert!(!snapshot.bcs_layer.unsafe_state);
        assert!(snapshot.bcs_layer.max_state_abs < 32.0);
    }

    #[test]
    fn bcs_layer_standard_scenarios_stay_finite_and_bounded() {
        let patch = load_patch_toml(CATHEDRAL_BLOOM).expect("factory patch must parse");
        let frames = ENGINE_LAYER_TEST_RATE_HZ as usize;

        for scenario in BcsScenario::ALL {
            let mut engine = Engine::new(
                EngineConfig {
                    sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
                    max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
                    voice_count: 6,
                },
                patch.clone(),
            )
            .expect("engine must validate");
            engine.set_bcs_layer_mode(BcsLayerMode::Enabled { scenario });

            let note_events =
                engine_layer_note_on_events(select_gfm_program_for_patch(&engine.patch).program_id);
            let controls = bcs_layer_playable_controller_events(0.68);
            let (_, stats) = render_engine_layer_signature_with_notes_and_controllers(
                &mut engine,
                frames,
                note_events,
                &controls,
            );
            let snapshot = engine.snapshot();

            assert!(stats.finite, "scenario={scenario:?}");
            assert!(
                stats.peak_abs <= MASTER_SAFETY_CEILING,
                "scenario={scenario:?} stats={stats:?}"
            );
            assert_eq!(snapshot.bcs_layer.active_scenario, Some(scenario));
            assert_eq!(snapshot.bcs_layer.unsafe_events, 0, "scenario={scenario:?}");
            assert!(!snapshot.bcs_layer.unsafe_state, "scenario={scenario:?}");
            assert!(
                snapshot.bcs_layer.max_state_abs < 32.0,
                "scenario={scenario:?}"
            );
            assert!(snapshot.bcs_layer.effective_amount > 0.0);
        }
    }

    #[test]
    fn bcs_layer_pitch_follows_lowest_held_note() {
        let mut engine = Engine::new(
            EngineConfig {
                sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
                max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
                voice_count: 6,
            },
            load_patch_toml(CATHEDRAL_BLOOM).expect("factory patch must parse"),
        )
        .expect("engine must validate");
        engine.set_bcs_layer_mode(BcsLayerMode::Enabled {
            scenario: BcsScenario::StableAnchor,
        });

        engine.process_block(ProcessBlock {
            frame_count: 64,
            note_events: &[
                Scheduled {
                    frame_offset: 0,
                    event: NoteEvent::NoteOn {
                        note: 60,
                        velocity: 0.80,
                    },
                },
                Scheduled {
                    frame_offset: 0,
                    event: NoteEvent::NoteOn {
                        note: 48,
                        velocity: 0.80,
                    },
                },
            ],
            controller_events: &bcs_layer_playable_controller_events(0.80),
            macro_state: None,
            output: None,
        });
        let snapshot = engine.snapshot().bcs_layer;

        assert_eq!(snapshot.pitch_note, Some(48));
        assert!(
            snapshot
                .pitch_frequency_hz
                .is_some_and(|hz| (hz - midi_note_hz(48.0)).abs() < 0.01)
        );
    }

    #[test]
    fn bcs_layer_releasing_all_held_notes_targets_silence() {
        let mut engine = Engine::new(
            EngineConfig {
                sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
                max_block_frames: ENGINE_LAYER_TEST_RATE_HZ as usize,
                voice_count: 6,
            },
            load_patch_toml(CATHEDRAL_BLOOM).expect("factory patch must parse"),
        )
        .expect("engine must validate");
        engine.set_bcs_layer_mode(BcsLayerMode::Enabled {
            scenario: BcsScenario::StableAnchor,
        });
        engine.process_block(ProcessBlock {
            frame_count: ENGINE_LAYER_BLOCK_FRAMES,
            note_events: &[Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: 48,
                    velocity: 0.80,
                },
            }],
            controller_events: &bcs_layer_playable_controller_events(1.0),
            macro_state: None,
            output: None,
        });
        assert!(engine.snapshot().bcs_layer.effective_amount > 0.0);

        engine.process_block(ProcessBlock {
            frame_count: ENGINE_LAYER_BLOCK_FRAMES,
            note_events: &[Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOff { note: 48 },
            }],
            controller_events: &[],
            macro_state: None,
            output: None,
        });
        let release_frames =
            smoothing_sample_count(ENGINE_LAYER_TEST_RATE_HZ as f32, BCS_LAYER_MIX_RELEASE_MS)
                + ENGINE_LAYER_BLOCK_FRAMES;
        engine.process_block(ProcessBlock {
            frame_count: release_frames,
            note_events: &[],
            controller_events: &[],
            macro_state: None,
            output: None,
        });
        let snapshot = engine.snapshot().bcs_layer;

        assert_eq!(snapshot.pitch_note, None);
        assert!(snapshot.enabled);
        assert!(snapshot.amount > 0.99);
        assert_eq!(snapshot.effective_amount, 0.0);
    }

    #[test]
    fn bcs_layer_rebuilds_when_patch_loads_and_resets_voice_state() {
        let first_patch = load_patch_toml(CATHEDRAL_BLOOM).expect("factory patch must parse");
        let second_patch = load_patch_toml(RAZOR_THAW).expect("factory patch must parse");
        let mut engine = Engine::new(
            EngineConfig {
                sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
                max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
                voice_count: 6,
            },
            first_patch,
        )
        .expect("engine must validate");
        engine.set_bcs_layer_mode(BcsLayerMode::Enabled {
            scenario: BcsScenario::EdgeSweep,
        });
        render_engine_layer_signature_for_engine(&mut engine, ENGINE_LAYER_TEST_RATE_HZ as usize);
        let before_load = engine.snapshot().bcs_layer;
        assert!(before_load.max_state_abs > 0.025);

        engine
            .load_patch(second_patch)
            .expect("patch load must validate");
        let after_load = engine.snapshot().bcs_layer;

        assert_eq!(after_load.mode, before_load.mode);
        assert_eq!(after_load.active_scenario, Some(BcsScenario::EdgeSweep));
        assert!(after_load.max_state_abs < before_load.max_state_abs);
        assert_eq!(after_load.unsafe_events, 0);
        assert!(!after_load.unsafe_state);
    }

    #[test]
    fn allocator_prefers_idle_then_released_then_oldest_active() {
        let mut engine = fixture_engine();

        for note in 60..66 {
            engine.process_block(ProcessBlock {
                frame_count: 64,
                note_events: &[Scheduled {
                    frame_offset: 0,
                    event: NoteEvent::NoteOn {
                        note,
                        velocity: 0.8,
                    },
                }],
                controller_events: &[],
                macro_state: None,
                output: None,
            });
        }

        let full_snapshot = engine.snapshot();
        assert_eq!(full_snapshot.active_voice_count, 6);
        assert_eq!(full_snapshot.voices[0].note, Some(60));

        engine.process_block(ProcessBlock {
            frame_count: 64,
            note_events: &[
                Scheduled {
                    frame_offset: 0,
                    event: NoteEvent::NoteOff { note: 60 },
                },
                Scheduled {
                    frame_offset: 1,
                    event: NoteEvent::NoteOn {
                        note: 72,
                        velocity: 0.8,
                    },
                },
            ],
            controller_events: &[],
            macro_state: None,
            output: None,
        });

        let released_reused = engine.snapshot();
        assert!(released_reused.held_notes.contains(&72));

        engine.process_block(ProcessBlock {
            frame_count: 64,
            note_events: &[Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: 73,
                    velocity: 0.8,
                },
            }],
            controller_events: &[],
            macro_state: None,
            output: None,
        });

        let stolen = engine.snapshot();
        assert!(stolen.held_notes.contains(&73));
        assert!(!stolen.held_notes.contains(&61));
    }

    #[test]
    fn note_off_prefers_held_voice_for_repeated_pitch() {
        let mut engine = fixture_engine();

        engine.process_block(ProcessBlock {
            frame_count: 64,
            note_events: &[
                Scheduled {
                    frame_offset: 0,
                    event: NoteEvent::NoteOn {
                        note: 60,
                        velocity: 0.8,
                    },
                },
                Scheduled {
                    frame_offset: 1,
                    event: NoteEvent::NoteOff { note: 60 },
                },
                Scheduled {
                    frame_offset: 2,
                    event: NoteEvent::NoteOn {
                        note: 60,
                        velocity: 0.7,
                    },
                },
                Scheduled {
                    frame_offset: 3,
                    event: NoteEvent::NoteOff { note: 60 },
                },
            ],
            controller_events: &[],
            macro_state: None,
            output: None,
        });

        let snapshot = engine.snapshot();
        assert!(
            snapshot
                .voices
                .iter()
                .filter(|voice| voice.note == Some(60))
                .all(|voice| voice.phase != VoicePhase::Held)
        );
    }

    #[test]
    fn sustain_state_transitions_are_stable() {
        let mut engine = fixture_engine();

        engine.process_block(ProcessBlock {
            frame_count: 64,
            note_events: &[Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: 60,
                    velocity: 1.0,
                },
            }],
            controller_events: &[Scheduled {
                frame_offset: 0,
                event: ControllerEvent::Sustain { down: true },
            }],
            macro_state: None,
            output: None,
        });

        engine.process_block(ProcessBlock {
            frame_count: 64,
            note_events: &[Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOff { note: 60 },
            }],
            controller_events: &[],
            macro_state: None,
            output: None,
        });

        let sustained = engine.snapshot();
        assert_eq!(sustained.voices[0].phase, VoicePhase::SustainedReleased);

        engine.process_block(ProcessBlock {
            frame_count: 64,
            note_events: &[],
            controller_events: &[Scheduled {
                frame_offset: 0,
                event: ControllerEvent::Sustain { down: false },
            }],
            macro_state: None,
            output: None,
        });
        assert_eq!(engine.snapshot().voices[0].phase, VoicePhase::Released);
    }

    #[test]
    fn rendered_audio_is_non_silent_and_finite() {
        let mut engine = fixture_engine();
        let mut left = [0.0_f32; 256];
        let mut right = [0.0_f32; 256];
        engine.process_block(ProcessBlock {
            frame_count: 256,
            note_events: &[Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: 60,
                    velocity: 0.9,
                },
            }],
            controller_events: &[],
            macro_state: None,
            output: Some(StereoBlockMut::new(&mut left, &mut right)),
        });

        let peak = left
            .iter()
            .zip(right.iter())
            .map(|(left, right)| left.abs().max(right.abs()))
            .fold(0.0, f32::max);
        assert!(peak > 0.0001);
        assert!(left.iter().all(|sample| sample.is_finite()));
        assert!(right.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn idle_master_output_rejects_patch_dc_bias() {
        let stats = render_post_warmup_stats(RAZOR_THAW, None);

        assert!(stats.left_mean.abs() < 0.002, "stats={stats:?}");
        assert!(stats.right_mean.abs() < 0.002, "stats={stats:?}");
        assert!(stats.peak_abs < 0.01, "stats={stats:?}");
    }

    #[test]
    fn sustained_gravity_wake_output_has_low_dc_mean() {
        let stats = render_post_warmup_stats(GRAVITY_WAKE, Some(60));

        assert!(stats.left_mean.abs() < 0.02, "stats={stats:?}");
        assert!(stats.right_mean.abs() < 0.02, "stats={stats:?}");
        assert!(stats.peak_abs > 0.05, "stats={stats:?}");
    }

    #[test]
    fn process_block_clamps_to_output_capacity() {
        let mut engine = fixture_engine();
        let mut left = [0.0_f32; 64];
        let mut right = [0.0_f32; 32];

        engine.process_block(ProcessBlock {
            frame_count: 128,
            note_events: &[Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: 60,
                    velocity: 0.9,
                },
            }],
            controller_events: &[],
            macro_state: None,
            output: Some(StereoBlockMut::new(&mut left, &mut right)),
        });

        assert_eq!(engine.snapshot().last_block_frames, 128);
        assert!(left[..32].iter().all(|sample| sample.is_finite()));
        assert!(right.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn macro_sweeps_remain_finite() {
        let mut engine = fixture_engine();
        let mut left = [0.0_f32; 512];
        let mut right = [0.0_f32; 512];

        engine.process_block(ProcessBlock {
            frame_count: 512,
            note_events: &[Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: 48,
                    velocity: 0.92,
                },
            }],
            controller_events: &[
                Scheduled {
                    frame_offset: 64,
                    event: ControllerEvent::Macro {
                        id: MacroId::Gravitacija,
                        value: 0.84,
                    },
                },
                Scheduled {
                    frame_offset: 160,
                    event: ControllerEvent::Macro {
                        id: MacroId::Ruin,
                        value: 0.76,
                    },
                },
                Scheduled {
                    frame_offset: 320,
                    event: ControllerEvent::Macro {
                        id: MacroId::Bloom,
                        value: 0.72,
                    },
                },
            ],
            macro_state: None,
            output: Some(StereoBlockMut::new(&mut left, &mut right)),
        });

        assert!(left.iter().all(|sample| sample.is_finite()));
        assert!(right.iter().all(|sample| sample.is_finite()));
        let peak = left
            .iter()
            .zip(right.iter())
            .map(|(left, right)| left.abs().max(right.abs()))
            .fold(0.0, f32::max);
        assert!(peak > 0.0001);
        assert!(peak <= MASTER_SAFETY_CEILING, "peak={peak}");
    }

    #[test]
    fn output_safety_snapshot_tracks_neutral_low_level_block() {
        let mut engine = fixture_engine();
        let mut left = [0.0_f32; 128];
        let mut right = [0.0_f32; 128];

        engine.process_block(ProcessBlock {
            frame_count: 128,
            note_events: &[],
            controller_events: &[],
            macro_state: None,
            output: Some(StereoBlockMut::new(&mut left, &mut right)),
        });

        let snapshot = engine.snapshot();
        assert!(left.iter().all(|sample| sample.is_finite()));
        assert!(right.iter().all(|sample| sample.is_finite()));
        assert_eq!(snapshot.output_safety.safety_limiter_hits, 0);
        assert_eq!(snapshot.output_safety.max_safety_reduction, 0.0);
        assert_eq!(snapshot.output_safety.tiny_flush_events, 0);
        assert!(
            snapshot.output_safety.post_safety_peak <= MASTER_SAFETY_CEILING,
            "snapshot={snapshot:?}"
        );
        assert_eq!(
            snapshot.output_safety.pre_safety_peak.to_bits(),
            snapshot.output_safety.post_safety_peak.to_bits()
        );
        assert_eq!(
            snapshot.peak_output.to_bits(),
            snapshot.output_safety.post_safety_peak.to_bits()
        );
        assert!(!snapshot.clip_detected);
    }

    #[test]
    fn output_safety_snapshot_counts_tiny_flush_and_limiter_work() {
        let mut tiny = OutputSafetySnapshot::default();
        tiny.observe_channel(DENORMAL_FLUSH_ABS * 0.5, 0.0);

        assert_eq!(tiny.tiny_flush_events, 1);
        assert_eq!(tiny.safety_limiter_hits, 0);
        assert_eq!(tiny.max_safety_reduction, 0.0);

        let hot_pre = 1.40_f32;
        let hot_post = master_safety_limit(hot_pre);
        let mut hot = OutputSafetySnapshot::default();
        hot.observe_channel(hot_pre, hot_post);

        assert_eq!(hot.safety_limiter_hits, 1);
        assert!(hot.max_safety_reduction > 0.0);
        assert!(hot_post <= MASTER_SAFETY_CEILING);
    }

    #[test]
    fn hot_render_reports_output_safety_limiter_telemetry() {
        let mut engine = fixture_engine();
        let mut left = [0.0_f32; 2048];
        let mut right = [0.0_f32; 2048];
        let note_events = [
            Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: 36,
                    velocity: 1.0,
                },
            },
            Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: 43,
                    velocity: 1.0,
                },
            },
            Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: 48,
                    velocity: 1.0,
                },
            },
            Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: 55,
                    velocity: 1.0,
                },
            },
            Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: 60,
                    velocity: 1.0,
                },
            },
            Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: 67,
                    velocity: 1.0,
                },
            },
        ];

        engine.process_block(ProcessBlock {
            frame_count: 2048,
            note_events: &note_events,
            controller_events: &[Scheduled {
                frame_offset: 0,
                event: ControllerEvent::DirectParam {
                    id: ParamId::FinalStageOutputTrimDb,
                    value: 12.0,
                },
            }],
            macro_state: Some(MacroState {
                gravitacija: 1.0,
                bloom: 0.7,
                heat: 1.0,
                ruin: 1.0,
                swarm: 0.75,
            }),
            output: Some(StereoBlockMut::new(&mut left, &mut right)),
        });

        let snapshot = engine.snapshot();
        assert!(left.iter().all(|sample| sample.is_finite()));
        assert!(right.iter().all(|sample| sample.is_finite()));
        assert!(
            snapshot.output_safety.pre_safety_peak > MASTER_SAFETY_CEILING,
            "snapshot={snapshot:?}"
        );
        assert!(
            snapshot.output_safety.post_safety_peak <= MASTER_SAFETY_CEILING,
            "snapshot={snapshot:?}"
        );
        assert!(
            snapshot.output_safety.safety_limiter_hits > 0,
            "snapshot={snapshot:?}"
        );
        assert!(
            snapshot.output_safety.max_safety_reduction > 0.0,
            "snapshot={snapshot:?}"
        );
        assert_eq!(
            snapshot.peak_output.to_bits(),
            snapshot.output_safety.post_safety_peak.to_bits()
        );
    }

    #[test]
    fn dry_path_remains_non_silent_with_fx_disabled() {
        let mut patch = load_patch_toml(MOLTEN_HORIZON).expect("fixture must parse");
        patch.engine.fx.chorus.enabled = false;
        patch.engine.fx.reverb.enabled = false;
        let mut engine =
            Engine::new(EngineConfig::default(), patch).expect("fixture must validate");
        let mut left = [0.0_f32; 512];
        let mut right = [0.0_f32; 512];

        engine.process_block(ProcessBlock {
            frame_count: 512,
            note_events: &[Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: 43,
                    velocity: 0.88,
                },
            }],
            controller_events: &[],
            macro_state: None,
            output: Some(StereoBlockMut::new(&mut left, &mut right)),
        });

        let peak = left
            .iter()
            .zip(right.iter())
            .map(|(left, right)| left.abs().max(right.abs()))
            .fold(0.0, f32::max);
        assert!(peak > 0.0001);
        assert!(left.iter().all(|sample| sample.is_finite()));
        assert!(right.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn dense_chord_playback_with_fx_stays_finite() {
        let mut engine = fixture_engine();
        let mut left = [0.0_f32; 1024];
        let mut right = [0.0_f32; 1024];
        let note_events = [
            Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: 48,
                    velocity: 0.82,
                },
            },
            Scheduled {
                frame_offset: 32,
                event: NoteEvent::NoteOn {
                    note: 55,
                    velocity: 0.84,
                },
            },
            Scheduled {
                frame_offset: 64,
                event: NoteEvent::NoteOn {
                    note: 60,
                    velocity: 0.88,
                },
            },
            Scheduled {
                frame_offset: 96,
                event: NoteEvent::NoteOn {
                    note: 67,
                    velocity: 0.90,
                },
            },
        ];
        let controller_events = [
            Scheduled {
                frame_offset: 128,
                event: ControllerEvent::Macro {
                    id: MacroId::Gravitacija,
                    value: 0.82,
                },
            },
            Scheduled {
                frame_offset: 256,
                event: ControllerEvent::Macro {
                    id: MacroId::Heat,
                    value: 0.74,
                },
            },
            Scheduled {
                frame_offset: 384,
                event: ControllerEvent::Macro {
                    id: MacroId::Ruin,
                    value: 0.70,
                },
            },
        ];

        engine.process_block(ProcessBlock {
            frame_count: 1024,
            note_events: &note_events,
            controller_events: &controller_events,
            macro_state: None,
            output: Some(StereoBlockMut::new(&mut left, &mut right)),
        });

        assert!(left.iter().all(|sample| sample.is_finite()));
        assert!(right.iter().all(|sample| sample.is_finite()));
        assert!(
            left.iter()
                .zip(right.iter())
                .map(|(left, right)| left.abs().max(right.abs()))
                .fold(0.0, f32::max)
                > 0.0001
        );
    }

    #[test]
    fn snapshot_exposes_patch_metadata() {
        let engine = fixture_engine();
        let snapshot = engine.snapshot();
        assert_eq!(snapshot.patch_name, "Molten Horizon");
        assert!(snapshot.patch_description.is_some());
        assert!(snapshot.patch_tags.iter().any(|tag| tag == "monster"));
        assert!(snapshot.patch_favorite);
    }

    #[test]
    fn load_patch_resets_runtime_state() {
        let mut engine = fixture_engine();
        let mut left = [0.0_f32; 128];
        let mut right = [0.0_f32; 128];

        engine.process_block(ProcessBlock {
            frame_count: 128,
            note_events: &[Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: 60,
                    velocity: 0.9,
                },
            }],
            controller_events: &[Scheduled {
                frame_offset: 0,
                event: ControllerEvent::Macro {
                    id: MacroId::Gravitacija,
                    value: 0.92,
                },
            }],
            macro_state: None,
            output: Some(StereoBlockMut::new(&mut left, &mut right)),
        });
        assert!(engine.snapshot().active_voice_count > 0);

        let replacement =
            load_patch_toml(include_str!("../../../patches/factory/ember-vault.toml"))
                .expect("replacement patch parses");
        engine
            .load_patch(replacement)
            .expect("replacement patch loads");

        let snapshot = engine.snapshot();
        assert_eq!(snapshot.patch_name, "Ember Vault");
        assert_eq!(snapshot.active_voice_count, 0);
        assert!(!snapshot.sustain_down);
        assert!(snapshot.peak_output.abs() <= f32::EPSILON);
    }

    #[test]
    fn panic_clears_notes_and_controller_state() {
        let mut engine = fixture_engine();

        engine.process_block(ProcessBlock {
            frame_count: 64,
            note_events: &[Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: 60,
                    velocity: 0.95,
                },
            }],
            controller_events: &[
                Scheduled {
                    frame_offset: 0,
                    event: ControllerEvent::Sustain { down: true },
                },
                Scheduled {
                    frame_offset: 1,
                    event: ControllerEvent::ModWheel { amount: 0.8 },
                },
                Scheduled {
                    frame_offset: 2,
                    event: ControllerEvent::ChannelAftertouch { pressure: 0.7 },
                },
                Scheduled {
                    frame_offset: 3,
                    event: ControllerEvent::GfmLayerAmount { amount: 0.25 },
                },
            ],
            macro_state: None,
            output: None,
        });

        engine.panic();

        let snapshot = engine.snapshot();
        assert_eq!(snapshot.active_voice_count, 0);
        assert!(snapshot.held_notes.is_empty());
        assert!(!snapshot.sustain_down);
        assert_eq!(snapshot.gfm_layer.amount, 0.0);
        assert_eq!(snapshot.gfm_layer.pressure, 0.0);
        assert_eq!(snapshot.gfm_layer.effective_amount, 0.0);
        assert!(snapshot.live_macros == MacroState::from_defaults(&engine.patch.macros));
        assert!(!snapshot.clip_detected);
    }

    #[test]
    fn panic_clears_patch_switch_mute_window() {
        let mut engine = fixture_engine();
        let replacement =
            load_patch_toml(include_str!("../../../patches/factory/ember-vault.toml"))
                .expect("replacement patch parses");
        engine
            .load_patch(replacement)
            .expect("replacement patch loads");

        let (left_before, right_before) = engine.render_frame();
        assert_eq!((left_before, right_before), (0.0, 0.0));

        engine.panic();

        let (left_after, right_after) = engine.render_frame();
        assert!(left_after.is_finite());
        assert!(right_after.is_finite());
    }

    #[test]
    fn reset_controllers_releases_sustain_state_but_keeps_held_voice() {
        let mut engine = fixture_engine();

        engine.process_block(ProcessBlock {
            frame_count: 64,
            note_events: &[Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: 60,
                    velocity: 0.9,
                },
            }],
            controller_events: &[Scheduled {
                frame_offset: 0,
                event: ControllerEvent::Sustain { down: true },
            }],
            macro_state: None,
            output: None,
        });
        engine.process_block(ProcessBlock {
            frame_count: 64,
            note_events: &[Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOff { note: 60 },
            }],
            controller_events: &[Scheduled {
                frame_offset: 0,
                event: ControllerEvent::Macro {
                    id: MacroId::Ruin,
                    value: 0.85,
                },
            }],
            macro_state: None,
            output: None,
        });

        assert!(engine.snapshot().sustain_down);
        engine.reset_controllers();

        let snapshot = engine.snapshot();
        assert!(!snapshot.sustain_down);
        assert!(snapshot.held_notes.contains(&60));
        assert!(snapshot.live_macros == MacroState::from_defaults(&engine.patch.macros));
    }

    fn render_engine_layer_signature(
        patch: PatchFileV1,
        layer_seed: Option<u64>,
        frames: usize,
    ) -> (u64, EngineLayerRenderStats) {
        render_engine_layer_signature_at_rate(patch, layer_seed, ENGINE_LAYER_TEST_RATE_HZ, frames)
    }

    fn render_engine_layer_signature_at_rate(
        patch: PatchFileV1,
        layer_seed: Option<u64>,
        sample_rate_hz: u32,
        frames: usize,
    ) -> (u64, EngineLayerRenderStats) {
        let note_events =
            engine_layer_note_on_events(select_gfm_program_for_patch(&patch).program_id);
        let mut engine = Engine::new(
            EngineConfig {
                sample_rate_hz: sample_rate_hz as f32,
                max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
                voice_count: 6,
            },
            patch,
        )
        .expect("engine must validate");

        if let Some(seed) = layer_seed {
            engine.set_gfm_layer_mode(GfmLayerMode::Enabled { seed });
        }

        render_engine_layer_signature_with_notes(&mut engine, frames, note_events)
    }

    fn render_engine_layer_signature_for_engine(
        engine: &mut Engine,
        frames: usize,
    ) -> (u64, EngineLayerRenderStats) {
        let note_events =
            engine_layer_note_on_events(select_gfm_program_for_patch(&engine.patch).program_id);
        render_engine_layer_signature_with_notes(engine, frames, note_events)
    }

    fn render_engine_layer_signature_with_notes(
        engine: &mut Engine,
        frames: usize,
        note_events: [ScheduledNoteEvent; 3],
    ) -> (u64, EngineLayerRenderStats) {
        render_engine_layer_signature_with_notes_and_controllers(engine, frames, note_events, &[])
    }

    fn bcs_layer_playable_controller_events(amount: f32) -> [ScheduledControllerEvent; 2] {
        [
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::BcsLayerEnabled { enabled: true },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::BcsLayerAmount { amount },
            },
        ]
    }

    fn render_engine_layer_signature_with_notes_and_controllers(
        engine: &mut Engine,
        frames: usize,
        note_events: [ScheduledNoteEvent; 3],
        initial_controller_events: &[ScheduledControllerEvent],
    ) -> (u64, EngineLayerRenderStats) {
        let mut rendered = 0;
        let mut signature = 0xcbf2_9ce4_8422_2325_u64;
        let mut finite = true;
        let mut peak_abs = 0.0_f32;
        let mut sum_squares = 0.0_f32;
        let mut left = [0.0_f32; ENGINE_LAYER_BLOCK_FRAMES];
        let mut right = [0.0_f32; ENGINE_LAYER_BLOCK_FRAMES];

        while rendered < frames {
            let frame_count = (frames - rendered).min(ENGINE_LAYER_BLOCK_FRAMES);

            if rendered == 0 {
                engine.process_block(ProcessBlock {
                    frame_count,
                    note_events: &note_events,
                    controller_events: initial_controller_events,
                    macro_state: None,
                    output: Some(StereoBlockMut::new(
                        &mut left[..frame_count],
                        &mut right[..frame_count],
                    )),
                });
            } else {
                engine.process_block(ProcessBlock {
                    frame_count,
                    note_events: &[],
                    controller_events: &[],
                    macro_state: None,
                    output: Some(StereoBlockMut::new(
                        &mut left[..frame_count],
                        &mut right[..frame_count],
                    )),
                });
            }

            for index in 0..frame_count {
                let left_sample = left[index];
                let right_sample = right[index];
                finite &= left_sample.is_finite() && right_sample.is_finite();
                peak_abs = peak_abs.max(left_sample.abs().max(right_sample.abs()));
                sum_squares += left_sample * left_sample + right_sample * right_sample;
                for pcm in [sample_to_pcm16(left_sample), sample_to_pcm16(right_sample)] {
                    signature ^= pcm as u16 as u64;
                    signature = signature.wrapping_mul(0x100_0000_01b3);
                }
            }

            rendered += frame_count;
        }

        let rms = (sum_squares / (frames.max(1) * 2) as f32).sqrt();
        let gfm_layer = engine.snapshot().gfm_layer;
        (
            signature,
            EngineLayerRenderStats {
                finite,
                rms,
                peak_abs,
                gfm_selection: gfm_layer.selection,
                gfm_program_id: gfm_layer.active_program_id,
                gfm_diagnostics: gfm_layer.diagnostics,
            },
        )
    }

    fn engine_layer_note_on_events(program_id: Option<GfmProgramId>) -> [ScheduledNoteEvent; 3] {
        let [low, middle, high] = engine_layer_notes(program_id);
        [
            Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: low,
                    velocity: 0.78,
                },
            },
            Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: middle,
                    velocity: 0.70,
                },
            },
            Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: high,
                    velocity: 0.64,
                },
            },
        ]
    }

    fn engine_layer_notes(program_id: Option<GfmProgramId>) -> [u8; 3] {
        match program_id {
            Some(GfmProgramId::PecPerformance) => [60, 67, 72],
            Some(GfmProgramId::HorizontPerformance | GfmProgramId::BakljaPerformance) | None => {
                [48, 55, 60]
            }
        }
    }

    fn render_gfm_adapter_signature(program_id: GfmProgramId) -> (u64, GfmDryRunStats) {
        let mut voice =
            GfmFieldVoice::new(program_id, GFM_PERFORMANCE_BASELINE_SEED, GFM_TEST_RATE_HZ);
        let frames = voice.frames();
        let probe_position = voice.probe_position();
        let mut signature = 0xcbf2_9ce4_8422_2325_u64;
        let mut finite = true;
        let mut peak_abs = 0.0_f32;

        for _ in 0..frames {
            let sample = voice.next_sample();
            finite &= sample.is_finite();
            peak_abs = peak_abs.max(sample.abs());
            let pcm = sample_to_pcm16(sample);
            signature ^= pcm as u16 as u64;
            signature = signature.wrapping_mul(0x100_0000_01b3);
        }

        let diagnostics = voice.diagnostics();
        (
            signature,
            GfmDryRunStats {
                finite,
                peak_abs,
                max_rupture_count: diagnostics.max_rupture_count,
                health: diagnostics.health,
                probe_position,
                frames,
                final_frame_index: voice.frame_index(),
            },
        )
    }

    fn render_gfm_adapter_signature_with_live_pressure(
        program_id: GfmProgramId,
        pressure: f32,
        frames: usize,
    ) -> (u64, GfmDryRunStats) {
        let mut voice =
            GfmFieldVoice::new(program_id, GFM_PERFORMANCE_BASELINE_SEED, GFM_TEST_RATE_HZ);
        let probe_position = voice.probe_position();
        let mut signature = 0xcbf2_9ce4_8422_2325_u64;
        let mut finite = true;
        let mut peak_abs = 0.0_f32;

        for _ in 0..frames {
            let sample = voice.next_sample_with_live_pressure(pressure);
            finite &= sample.is_finite();
            peak_abs = peak_abs.max(sample.abs());
            let pcm = sample_to_pcm16(sample);
            signature ^= pcm as u16 as u64;
            signature = signature.wrapping_mul(0x100_0000_01b3);
        }

        let diagnostics = voice.diagnostics();
        (
            signature,
            GfmDryRunStats {
                finite,
                peak_abs,
                max_rupture_count: diagnostics.max_rupture_count,
                health: diagnostics.health,
                probe_position,
                frames,
                final_frame_index: voice.frame_index(),
            },
        )
    }

    fn render_gfm_block_signature(program_id: GfmProgramId) -> (u64, GfmDryRunStats) {
        let voice = GfmFieldVoice::new(program_id, GFM_PERFORMANCE_BASELINE_SEED, GFM_TEST_RATE_HZ);
        render_gfm_voice_block_signature(voice)
    }

    fn render_gfm_voice_block_signature(mut voice: GfmFieldVoice) -> (u64, GfmDryRunStats) {
        let frames = voice.frames();
        let probe_position = voice.probe_position();
        let mut signature = 0xcbf2_9ce4_8422_2325_u64;
        let mut finite = true;
        let mut peak_abs = 0.0_f32;
        let mut rendered = 0;
        let mut block = [0.0_f32; 512];

        while rendered < frames {
            let frame_count = (frames - rendered).min(block.len());
            voice.render_mono_block(&mut block[..frame_count]);
            for sample in &block[..frame_count] {
                finite &= sample.is_finite();
                peak_abs = peak_abs.max(sample.abs());
                let pcm = sample_to_pcm16(*sample);
                signature ^= pcm as u16 as u64;
                signature = signature.wrapping_mul(0x100_0000_01b3);
            }
            rendered += frame_count;
        }

        let diagnostics = voice.diagnostics();
        (
            signature,
            GfmDryRunStats {
                finite,
                peak_abs,
                max_rupture_count: diagnostics.max_rupture_count,
                health: diagnostics.health,
                probe_position,
                frames,
                final_frame_index: voice.frame_index(),
            },
        )
    }

    fn render_gfm_chunked_signature(
        program_id: GfmProgramId,
        chunk_size: usize,
    ) -> (u64, GfmDryRunStats) {
        let mut voice =
            GfmFieldVoice::new(program_id, GFM_PERFORMANCE_BASELINE_SEED, GFM_TEST_RATE_HZ);
        let frames = voice.frames();
        let probe_position = voice.probe_position();
        let mut signature = 0xcbf2_9ce4_8422_2325_u64;
        let mut finite = true;
        let mut peak_abs = 0.0_f32;
        let mut rendered = 0;
        let mut block = [0.0_f32; 251];
        let chunk_size = chunk_size.clamp(1, block.len());

        while rendered < frames {
            let frame_count = (frames - rendered).min(chunk_size);
            voice.render_mono_block(&mut block[..frame_count]);
            for sample in &block[..frame_count] {
                finite &= sample.is_finite();
                peak_abs = peak_abs.max(sample.abs());
                let pcm = sample_to_pcm16(*sample);
                signature ^= pcm as u16 as u64;
                signature = signature.wrapping_mul(0x100_0000_01b3);
            }
            rendered += frame_count;
        }

        let diagnostics = voice.diagnostics();
        (
            signature,
            GfmDryRunStats {
                finite,
                peak_abs,
                max_rupture_count: diagnostics.max_rupture_count,
                health: diagnostics.health,
                probe_position,
                frames,
                final_frame_index: voice.frame_index(),
            },
        )
    }

    fn render_gfm_selected_patch_signature(program_id: GfmProgramId) -> (u64, GfmDryRunStats) {
        render_gfm_block_signature(program_id)
    }

    fn gfm_patch_voice_test_config() -> GfmPatchVoiceConfig {
        GfmPatchVoiceConfig::new(GFM_PERFORMANCE_BASELINE_SEED, GFM_TEST_RATE_HZ)
    }

    fn low_score_gfm_patch() -> PatchFileV1 {
        let mut patch = load_patch_toml(MOLTEN_HORIZON).expect("fixture must parse");
        patch.meta.patch_name = "GFM Low Score Disabled".to_string();
        patch.macros.gravitacija = 0.0;
        patch.macros.bloom = 0.0;
        patch.macros.heat = 0.0;
        patch.macros.ruin = 0.0;
        patch.macros.swarm = 0.0;
        patch.identity_bias.horizont_bias = -1.0;
        patch.identity_bias.pec_bias = -1.0;
        patch.identity_bias.baklja_bias = -1.0;
        patch.identity_bias.gravitacija_pressure_bias = -1.0;
        patch.identity_bias.rupture_threshold_bias = 1.0;
        patch.identity_bias.body_focus_bias = -1.0;
        patch
    }

    fn render_direct_field_signature(program_id: GfmProgramId) -> (u64, GfmDryRunStats) {
        let program = GfmPerformanceProgram::new(program_id, GFM_TEST_RATE_HZ as f32);
        let gesture = GfmPerformanceGesture::v0_4();
        let mut lattice = GfmLattice16::new(GFM_PERFORMANCE_BASELINE_SEED, program.params());
        let frames = gesture.frames(GFM_TEST_RATE_HZ);
        let probe_position = lattice.probe_position();
        let mut signature = 0xcbf2_9ce4_8422_2325_u64;
        let mut finite = true;
        let mut peak_abs = 0.0_f32;

        for frame in 0..frames {
            let sample = lattice
                .next_sample_with_excitation(gesture.excitation_at_frame(frame, GFM_TEST_RATE_HZ));
            finite &= sample.is_finite();
            peak_abs = peak_abs.max(sample.abs());
            let pcm = sample_to_pcm16(sample);
            signature ^= pcm as u16 as u64;
            signature = signature.wrapping_mul(0x100_0000_01b3);
        }

        let diagnostics = lattice.diagnostics();
        (
            signature,
            GfmDryRunStats {
                finite,
                peak_abs,
                max_rupture_count: diagnostics.max_rupture_count,
                health: diagnostics.health,
                probe_position,
                frames,
                final_frame_index: frames,
            },
        )
    }
}
