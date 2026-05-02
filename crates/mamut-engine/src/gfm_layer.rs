use super::*;

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

pub(crate) fn sanitize_gfm_selection_score(value: f32) -> f32 {
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

pub(crate) fn gfm_live_pressure_excitation(base: GfmExcitation, pressure: f32) -> GfmExcitation {
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

pub(crate) fn gfm_momentary_layer_amount(amount: f32, pressure: f32) -> f32 {
    let amount = normalize_gfm_layer_control(amount);
    let pressure = normalize_gfm_layer_control(pressure);
    if amount <= f32::EPSILON || pressure <= f32::EPSILON {
        return 0.0;
    }
    let gate = expressive_amount(pressure, 0.70);
    (amount * gate).clamp(0.0, 1.0)
}

pub(crate) fn normalize_gfm_layer_control(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    if value <= GFM_LAYER_CONTROL_DEADZONE {
        0.0
    } else {
        value
    }
}

pub(crate) fn normalize_bcs_layer_control(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    if value <= BCS_LAYER_CONTROL_DEADZONE {
        0.0
    } else {
        value
    }
}
