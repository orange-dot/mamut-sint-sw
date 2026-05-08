use super::*;

pub type PatchExtensions = toml::Table;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatchFileV1 {
    pub meta: PatchMeta,
    pub engine: EnginePatchDefaults,
    pub macros: MacroDefaults,
    pub macro_response: MacroResponseSet,
    pub identity_bias: IdentityBiasProfile,
    pub performance_response: PerformanceResponseProfile,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui: Option<UiPatchHints>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<PatchExtensions>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatchMeta {
    pub schema_version: u32,
    pub patch_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnginePatchDefaults {
    pub osc1: Osc1Patch,
    pub osc2: Osc2Patch,
    #[serde(default)]
    pub spectral: SpectralPatch,
    #[serde(default)]
    pub additive: AdditivePatch,
    #[serde(default = "default_source_pwm_rate_hz")]
    pub source_pwm_rate_hz: f32,
    #[serde(default)]
    pub noise_color: NoiseColor,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub noise_filter_level: Option<f32>,
    #[serde(default)]
    pub noise_body_level: f32,
    #[serde(default)]
    pub analog_drift: f32,
    #[serde(default)]
    pub micro_jitter: f32,
    #[serde(default)]
    pub fm_amount: f32,
    #[serde(default)]
    pub fm_direction: ModDirection,
    #[serde(default)]
    pub phase_mod_amount: f32,
    #[serde(default)]
    pub phase_mod_direction: ModDirection,
    #[serde(default)]
    pub ring_mod_amount: f32,
    #[serde(default)]
    pub am_amount: f32,
    #[serde(default)]
    pub sync_direction: ModDirection,
    #[serde(default)]
    pub sync_softness: f32,
    #[serde(default)]
    pub cross_mix_mode: CrossMixMode,
    #[serde(default)]
    pub cross_mix_amount: f32,
    pub sub: SubPatch,
    pub mixer: MixerPatch,
    pub filter: FilterPatch,
    pub amp_env: AdsrPatch,
    pub filter_env: FilterEnvPatch,
    pub voice: VoicePatch,
    pub final_stage: FinalStagePatch,
    pub fx: FxPatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Osc1Patch {
    pub saw_level: f32,
    pub pulse_level: f32,
    pub triangle_level: f32,
    pub noise_level: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fine_tune_cents: Option<f32>,
    #[serde(default = "default_half")]
    pub pulse_width: f32,
    #[serde(default)]
    pub pwm_depth: f32,
    #[serde(default)]
    pub phase_mode: OscPhaseMode,
    #[serde(default)]
    pub start_phase: f32,
    #[serde(default)]
    pub saw_bend: f32,
    #[serde(default)]
    pub triangle_fold: f32,
    #[serde(default)]
    pub pulse_edge: f32,
    #[serde(default)]
    pub bandlimit: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Osc2Patch {
    pub saw_level: f32,
    pub pulse_level: f32,
    pub triangle_level: f32,
    pub interval_semitones: i8,
    pub fine_tune_cents: f32,
    pub sync_amount: f32,
    pub crossmod_amount: f32,
    #[serde(default = "default_half")]
    pub pulse_width: f32,
    #[serde(default)]
    pub pwm_depth: f32,
    #[serde(default)]
    pub phase_mode: OscPhaseMode,
    #[serde(default)]
    pub start_phase: f32,
    #[serde(default = "default_one")]
    pub level: f32,
    #[serde(default)]
    pub pitch_mode: Osc2PitchMode,
    #[serde(default = "default_one")]
    pub ratio: f32,
    #[serde(default)]
    pub saw_bend: f32,
    #[serde(default)]
    pub triangle_fold: f32,
    #[serde(default)]
    pub pulse_edge: f32,
    #[serde(default)]
    pub bandlimit: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OscPhaseMode {
    #[default]
    Deterministic,
    Fixed,
    FreeRun,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Osc2PitchMode {
    #[default]
    Semitone,
    Ratio,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpectralPatch {
    #[serde(default)]
    pub level: f32,
    #[serde(default)]
    pub table: SpectralTable,
    #[serde(default)]
    pub position: f32,
    #[serde(default)]
    pub morph: f32,
    #[serde(default = "default_one")]
    pub ratio: f32,
    #[serde(default)]
    pub fine_tune_cents: f32,
}

impl Default for SpectralPatch {
    fn default() -> Self {
        Self {
            level: 0.0,
            table: SpectralTable::default(),
            position: 0.0,
            morph: 0.0,
            ratio: 1.0,
            fine_tune_cents: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SpectralTable {
    #[default]
    Sineish,
    Vocalish,
    Metallic,
    Hollow,
    Formant,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdditivePatch {
    #[serde(default)]
    pub level: f32,
    #[serde(default = "default_additive_partial_count")]
    pub partial_count: u8,
    #[serde(default)]
    pub harmonic_spread: f32,
    #[serde(default)]
    pub odd_even_balance: f32,
    #[serde(default)]
    pub inharmonicity: f32,
    #[serde(default)]
    pub spectral_tilt: f32,
    #[serde(default)]
    pub random_detune_cents: f32,
}

impl Default for AdditivePatch {
    fn default() -> Self {
        Self {
            level: 0.0,
            partial_count: default_additive_partial_count(),
            harmonic_spread: 0.0,
            odd_even_balance: 0.0,
            inharmonicity: 0.0,
            spectral_tilt: 0.0,
            random_detune_cents: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NoiseColor {
    #[default]
    White,
    Pinkish,
    Dark,
    Bright,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ModDirection {
    #[default]
    Osc1ToOsc2,
    Osc2ToOsc1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CrossMixMode {
    #[default]
    Sum,
    Multiply,
    Fold,
    Max,
    Difference,
}

fn default_half() -> f32 {
    0.5
}

fn default_one() -> f32 {
    1.0
}

fn default_source_pwm_rate_hz() -> f32 {
    0.35
}

fn default_additive_partial_count() -> u8 {
    6
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubPatch {
    pub level: f32,
    pub octave_offset: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MixerPatch {
    pub pre_filter_drive: f32,
    pub body_mix: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilterPatch {
    #[serde(default)]
    pub model: FilterModel,
    pub cutoff_hz: f32,
    pub resonance: f32,
    pub drive: f32,
    pub keytrack: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FilterModel {
    #[default]
    Legacy,
    TptClean,
    MatterDriven,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdsrPatch {
    pub attack_ms: f32,
    pub decay_ms: f32,
    pub sustain: f32,
    pub release_ms: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilterEnvPatch {
    #[serde(flatten)]
    pub adsr: AdsrPatch,
    pub depth: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoicePatch {
    pub stereo_width: f32,
    pub detune_spread_cents: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub velocity_to_level: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub velocity_to_filter: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinalStagePatch {
    pub body_drive: f32,
    pub asymmetry: f32,
    pub low_mid_emphasis: f32,
    pub output_trim_db: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FxPatch {
    pub chorus: ChorusPatch,
    pub reverb: ReverbPatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChorusPatch {
    pub enabled: bool,
    pub mix: f32,
    pub depth: f32,
    pub rate_hz: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReverbPatch {
    pub enabled: bool,
    pub mix: f32,
    pub size: f32,
    pub damping: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MacroDefaults {
    pub gravitacija: f32,
    pub bloom: f32,
    pub heat: f32,
    pub ruin: f32,
    pub swarm: f32,
}

impl MacroDefaults {
    pub fn get(&self, id: MacroId) -> f32 {
        match id {
            MacroId::Gravitacija => self.gravitacija,
            MacroId::Bloom => self.bloom,
            MacroId::Heat => self.heat,
            MacroId::Ruin => self.ruin,
            MacroId::Swarm => self.swarm,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MacroResponseCurve {
    Linear,
    SoftPlus,
    LateRise,
    EarlyRise,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MacroResponseSpec {
    pub sensitivity: f32,
    pub curve: MacroResponseCurve,
    pub softness: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MacroResponseSet {
    pub gravitacija: MacroResponseSpec,
    pub bloom: MacroResponseSpec,
    pub heat: MacroResponseSpec,
    pub ruin: MacroResponseSpec,
    pub swarm: MacroResponseSpec,
}

impl MacroResponseSet {
    pub fn spec(&self, id: MacroId) -> MacroResponseSpec {
        match id {
            MacroId::Gravitacija => self.gravitacija,
            MacroId::Bloom => self.bloom,
            MacroId::Heat => self.heat,
            MacroId::Ruin => self.ruin,
            MacroId::Swarm => self.swarm,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityBiasProfile {
    pub horizont_bias: f32,
    pub pec_bias: f32,
    pub baklja_bias: f32,
    pub gravitacija_pressure_bias: f32,
    pub rupture_threshold_bias: f32,
    pub body_focus_bias: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerformanceResponseProfile {
    pub velocity_to_level: f32,
    pub velocity_to_filter: f32,
    pub aftertouch_to_gravitacija: f32,
    pub aftertouch_to_baklja: f32,
    pub mod_wheel_to_bloom: f32,
    pub mod_wheel_to_swarm: f32,
    pub bend_range_semitones: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiPreferredPage {
    Macros,
    Advanced,
    Fx,
    Diagnostics,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UiPatchHints {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_page: Option<UiPreferredPage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub favorite: Option<bool>,
}
