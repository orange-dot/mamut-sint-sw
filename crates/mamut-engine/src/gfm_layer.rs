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
    let mut selection = select_gfm_program(resolve_identity(patch, &macros));
    if let Some(program_id) = pinned_gfm_program(patch.engine.gfm.program) {
        selection.program_id = Some(program_id);
    }
    selection
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GfmExcitationMode {
    OfflineGesture,
    Live,
}

#[derive(Debug, Clone)]
pub struct GfmPatchVoiceDecision {
    config: GfmPatchVoiceConfig,
    selection: GfmVoiceProgramSelection,
    controls: GfmPerformanceControls,
    voice: Option<GfmFieldVoice>,
}

impl GfmPatchVoiceDecision {
    pub const fn config(&self) -> GfmPatchVoiceConfig {
        self.config
    }

    pub const fn selection(&self) -> GfmVoiceProgramSelection {
        self.selection
    }

    pub const fn controls(&self) -> GfmPerformanceControls {
        self.controls
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
    controls: GfmPerformanceControls,
    mode: GfmExcitationMode,
    lattice: GfmLattice16,
    sample_rate_hz: u32,
    frame_index: usize,
}

impl GfmFieldVoice {
    pub fn new(program_id: GfmProgramId, seed: u64, sample_rate_hz: u32) -> Self {
        Self::new_offline_gesture_with_controls(
            program_id,
            seed,
            sample_rate_hz,
            GfmPerformanceControls::DEFAULT,
        )
    }

    pub fn new_live(
        program_id: GfmProgramId,
        seed: u64,
        sample_rate_hz: u32,
        controls: GfmPerformanceControls,
    ) -> Self {
        Self::new_with_mode(
            program_id,
            seed,
            sample_rate_hz,
            controls,
            GfmExcitationMode::Live,
        )
    }

    pub fn new_offline_gesture_with_controls(
        program_id: GfmProgramId,
        seed: u64,
        sample_rate_hz: u32,
        controls: GfmPerformanceControls,
    ) -> Self {
        Self::new_with_mode(
            program_id,
            seed,
            sample_rate_hz,
            controls,
            GfmExcitationMode::OfflineGesture,
        )
    }

    fn new_with_mode(
        program_id: GfmProgramId,
        seed: u64,
        sample_rate_hz: u32,
        controls: GfmPerformanceControls,
        mode: GfmExcitationMode,
    ) -> Self {
        let sample_rate_hz = sample_rate_hz.max(1);
        let controls = controls.sanitized();
        let program =
            GfmPerformanceProgram::with_controls(program_id, sample_rate_hz as f32, controls);
        let params = gfm_live_lattice_params(program_id, program.params(), controls, mode);
        Self {
            program,
            gesture: GfmPerformanceGesture::v0_4(),
            controls,
            mode,
            lattice: GfmLattice16::new(seed, params),
            sample_rate_hz,
            frame_index: 0,
        }
    }

    pub fn from_patch(patch: &PatchFileV1, config: GfmPatchVoiceConfig) -> GfmPatchVoiceDecision {
        let config = config.normalized();
        let selection = select_gfm_program_for_patch(patch);
        let controls = gfm_controls_from_patch(patch.engine.gfm);
        let voice = selection.program_id.map(|program_id| {
            Self::new_live(program_id, config.seed, config.sample_rate_hz, controls)
        });

        GfmPatchVoiceDecision {
            config,
            selection,
            controls,
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

    pub const fn controls(&self) -> GfmPerformanceControls {
        self.controls
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
        self.next_sample_with_live_control(pressure, pressure)
    }

    pub fn next_sample_with_live_control(&mut self, pressure: f32, amount: f32) -> f32 {
        let excitation = match self.mode {
            GfmExcitationMode::OfflineGesture => {
                let excitation = self
                    .gesture
                    .excitation_at_frame(self.frame_index, self.sample_rate_hz);
                gfm_live_pressure_excitation_with_controls(excitation, pressure, self.controls)
            }
            GfmExcitationMode::Live => {
                gfm_live_excitation(self.program.id(), pressure, amount, self.controls)
            }
        };
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

fn gfm_live_lattice_params(
    program_id: GfmProgramId,
    mut params: mamut_field::GfmParams,
    controls: GfmPerformanceControls,
    mode: GfmExcitationMode,
) -> mamut_field::GfmParams {
    if !matches!(mode, GfmExcitationMode::Live) {
        return params;
    }

    let controls = controls.sanitized();
    match program_id {
        GfmProgramId::HorizontPerformance | GfmProgramId::PecPerformance => {
            params.rupture_threshold = params.rupture_threshold.max(1.20);
            params.rupture_response *= 0.35;
        }
        GfmProgramId::BakljaPerformance => {
            params.ruin =
                (0.42 + controls.rupture * 0.18 + controls.brightness * 0.08).clamp(0.0, 1.0);
            params.rupture_threshold =
                (0.40 - controls.rupture * 0.04 - controls.brightness * 0.02).clamp(0.30, 1.20);
            params.rupture_response =
                (0.34 + controls.rupture * 0.32 + controls.brightness * 0.12).clamp(0.0, 1.0);
            params.rupture_quorum = params.rupture_quorum.max(3);
        }
    }
    params
}

fn gfm_live_pressure_excitation_with_controls(
    base: GfmExcitation,
    pressure: f32,
    controls: GfmPerformanceControls,
) -> GfmExcitation {
    let pressure = normalize_gfm_layer_control(pressure);
    if pressure <= f32::EPSILON {
        return base;
    }
    let controls = controls.sanitized();
    let depth_scale = 0.58 + controls.depth * 0.84;
    let heat_scale = 0.28 + controls.heat * 0.78;
    let rupture_scale = 0.22 + controls.rupture * 0.92;

    GfmExcitation {
        pressure: (base.pressure + pressure * 0.22 * depth_scale).clamp(0.0, 1.0),
        heat: (base.heat + pressure * 0.10 * heat_scale).clamp(0.0, 1.0),
        rupture_bias: (base.rupture_bias + pressure * 0.16 * rupture_scale).clamp(0.0, 1.0),
    }
}

fn gfm_live_excitation(
    program_id: GfmProgramId,
    pressure: f32,
    amount: f32,
    controls: GfmPerformanceControls,
) -> GfmExcitation {
    let pressure = normalize_gfm_layer_control(pressure);
    let amount = normalize_gfm_layer_control(amount);
    let controls = controls.sanitized();
    if pressure <= f32::EPSILON && amount <= f32::EPSILON {
        return GfmExcitation::none();
    }

    match program_id {
        GfmProgramId::HorizontPerformance => {
            gfm_horizont_live_excitation(pressure, amount, controls)
        }
        GfmProgramId::PecPerformance => gfm_pec_live_excitation(pressure, amount, controls),
        GfmProgramId::BakljaPerformance => gfm_baklja_live_excitation(pressure, amount, controls),
    }
}

fn gfm_horizont_live_excitation(
    pressure: f32,
    amount: f32,
    controls: GfmPerformanceControls,
) -> GfmExcitation {
    let expression = (pressure * 0.78 + amount * 0.22).clamp(0.0, 1.0);
    let open = expressive_amount(expression, 0.82);
    let motion = expressive_amount(pressure, 0.74);
    let depth_drive =
        open * (0.10 + controls.depth * 0.42 + controls.spread * 0.20 + controls.motion * 0.08);
    let heat_drive = motion * (0.015 + controls.heat * 0.14 + controls.brightness * 0.08);

    GfmExcitation {
        pressure: depth_drive.clamp(0.0, 1.0),
        heat: heat_drive.clamp(0.0, 1.0),
        rupture_bias: 0.0,
    }
}

fn gfm_pec_live_excitation(
    pressure: f32,
    amount: f32,
    controls: GfmPerformanceControls,
) -> GfmExcitation {
    let expression = (pressure * 0.60 + amount * 0.40).clamp(0.0, 1.0);
    let thermal = expressive_amount(expression, 0.76);
    let pressure_drive = thermal * (0.08 + controls.body * 0.34 + controls.depth * 0.16);
    let heat_drive = thermal
        * (0.18 + controls.heat * 0.58 + controls.brightness * 0.16 + controls.motion * 0.06);

    GfmExcitation {
        pressure: pressure_drive.clamp(0.0, 1.0),
        heat: heat_drive.clamp(0.0, 1.0),
        rupture_bias: 0.0,
    }
}

fn gfm_baklja_live_excitation(
    pressure: f32,
    amount: f32,
    controls: GfmPerformanceControls,
) -> GfmExcitation {
    let expression = (pressure * 0.66 + amount * 0.34).clamp(0.0, 1.0);
    let body = expressive_amount(expression, 0.82);
    let heat = expressive_amount((pressure * 0.54 + amount * 0.46).clamp(0.0, 1.0), 0.72);
    let edge = (pressure * 0.72 + amount * 0.28).clamp(0.0, 1.0);
    let threshold = (0.62 - controls.rupture * 0.22 - controls.brightness * 0.10).clamp(0.28, 0.68);
    let rupture_gate = ((edge - threshold) / (1.0 - threshold)).clamp(0.0, 1.0);
    let rupture_drive = expressive_amount(rupture_gate, 1.26)
        * (0.38 + controls.rupture * 0.82 + controls.brightness * 0.30);

    GfmExcitation {
        pressure: (body * (0.18 + controls.depth * 0.62 + controls.body * 0.20)).clamp(0.0, 1.0),
        heat: (heat * (0.12 + controls.heat * 0.42 + controls.brightness * 0.24)).clamp(0.0, 1.0),
        rupture_bias: rupture_drive.clamp(0.0, 1.0),
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

pub(crate) fn pinned_gfm_program(program: GfmPatchProgram) -> Option<GfmProgramId> {
    match program {
        GfmPatchProgram::Auto => None,
        GfmPatchProgram::Horizont => Some(GfmProgramId::HorizontPerformance),
        GfmPatchProgram::Pec => Some(GfmProgramId::PecPerformance),
        GfmPatchProgram::Baklja => Some(GfmProgramId::BakljaPerformance),
    }
}

pub(crate) fn gfm_controls_from_patch(gfm: GfmPatch) -> GfmPerformanceControls {
    GfmPerformanceControls {
        depth: gfm.depth,
        heat: gfm.heat,
        spread: gfm.spread,
        rupture: gfm.rupture,
        recovery: gfm.recovery,
        motion: gfm.motion,
        body: gfm.body,
        brightness: gfm.brightness,
    }
    .sanitized()
}
