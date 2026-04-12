use mamut_params::MacroId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
    pub cutoff_hz: f32,
    pub resonance: f32,
    pub drive: f32,
    pub keytrack: f32,
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

#[derive(Debug, Error)]
pub enum PatchLoadError {
    #[error("failed to parse patch TOML: {0}")]
    Parse(#[from] toml::de::Error),
}

#[derive(Debug, Error)]
pub enum PatchSaveError {
    #[error("failed to serialize patch TOML: {0}")]
    Serialize(#[from] toml::ser::Error),
}

#[derive(Debug, Error, PartialEq)]
pub enum PatchValidationError {
    #[error("schema_version must be 1, got {found}")]
    UnsupportedSchemaVersion { found: u32 },
    #[error("patch_name must not be empty")]
    EmptyPatchName,
    #[error("tag at index {index} must not be empty")]
    EmptyTag { index: usize },
    #[error("{field} must be in range {min}..={max}, got {value}")]
    RangeViolation {
        field: &'static str,
        min: f32,
        max: f32,
        value: f32,
    },
    #[error("{field} must be in integer range {min}..={max}, got {value}")]
    IntegerRangeViolation {
        field: &'static str,
        min: i32,
        max: i32,
        value: i32,
    },
    #[error("{field} must be positive, got {value}")]
    PositiveRequired { field: &'static str, value: f32 },
    #[error("{0}")]
    PolicyViolation(&'static str),
}

pub fn load_patch_toml(input: &str) -> Result<PatchFileV1, PatchLoadError> {
    toml::from_str(input).map_err(PatchLoadError::from)
}

pub fn save_patch_toml(patch: &PatchFileV1) -> Result<String, PatchSaveError> {
    toml::to_string_pretty(patch).map_err(PatchSaveError::from)
}

pub fn validate_patch_v1(patch: &PatchFileV1) -> Result<(), PatchValidationError> {
    if patch.meta.schema_version != 1 {
        return Err(PatchValidationError::UnsupportedSchemaVersion {
            found: patch.meta.schema_version,
        });
    }

    if patch.meta.patch_name.trim().is_empty() {
        return Err(PatchValidationError::EmptyPatchName);
    }

    if let Some(tags) = &patch.meta.tags {
        for (index, tag) in tags.iter().enumerate() {
            if tag.trim().is_empty() {
                return Err(PatchValidationError::EmptyTag { index });
            }
        }
    }

    validate_osc1(&patch.engine.osc1)?;
    validate_osc2(&patch.engine.osc2)?;
    validate_sub(&patch.engine.sub)?;
    validate_mixer(&patch.engine.mixer)?;
    validate_filter(&patch.engine.filter)?;
    validate_adsr("engine.amp_env", &patch.engine.amp_env)?;
    validate_filter_env(&patch.engine.filter_env)?;
    validate_voice(&patch.engine.voice)?;
    validate_final_stage(&patch.engine.final_stage)?;
    validate_fx(&patch.engine.fx)?;
    validate_macro_defaults(&patch.macros)?;
    validate_macro_response_set(&patch.macro_response)?;
    validate_identity_bias(&patch.identity_bias)?;
    validate_performance_response(&patch.performance_response)?;
    validate_extensions(&patch.x)?;

    Ok(())
}

fn validate_osc1(osc1: &Osc1Patch) -> Result<(), PatchValidationError> {
    check_range("engine.osc1.saw_level", osc1.saw_level, 0.0, 1.0)?;
    check_range("engine.osc1.pulse_level", osc1.pulse_level, 0.0, 1.0)?;
    check_range("engine.osc1.triangle_level", osc1.triangle_level, 0.0, 1.0)?;
    check_range("engine.osc1.noise_level", osc1.noise_level, 0.0, 1.0)?;
    if let Some(fine_tune_cents) = osc1.fine_tune_cents {
        check_range(
            "engine.osc1.fine_tune_cents",
            fine_tune_cents,
            -100.0,
            100.0,
        )?;
    }

    Ok(())
}

fn validate_osc2(osc2: &Osc2Patch) -> Result<(), PatchValidationError> {
    check_range("engine.osc2.saw_level", osc2.saw_level, 0.0, 1.0)?;
    check_range("engine.osc2.pulse_level", osc2.pulse_level, 0.0, 1.0)?;
    check_range("engine.osc2.triangle_level", osc2.triangle_level, 0.0, 1.0)?;
    check_integer_range(
        "engine.osc2.interval_semitones",
        osc2.interval_semitones as i32,
        -24,
        24,
    )?;
    check_range(
        "engine.osc2.fine_tune_cents",
        osc2.fine_tune_cents,
        -100.0,
        100.0,
    )?;
    check_range("engine.osc2.sync_amount", osc2.sync_amount, 0.0, 1.0)?;
    check_range(
        "engine.osc2.crossmod_amount",
        osc2.crossmod_amount,
        0.0,
        1.0,
    )?;

    Ok(())
}

fn validate_sub(sub: &SubPatch) -> Result<(), PatchValidationError> {
    check_range("engine.sub.level", sub.level, 0.0, 1.0)?;
    check_integer_range("engine.sub.octave_offset", sub.octave_offset as i32, -2, 0)?;
    Ok(())
}

fn validate_mixer(mixer: &MixerPatch) -> Result<(), PatchValidationError> {
    check_range(
        "engine.mixer.pre_filter_drive",
        mixer.pre_filter_drive,
        0.0,
        1.0,
    )?;
    check_range("engine.mixer.body_mix", mixer.body_mix, 0.0, 1.0)?;
    Ok(())
}

fn validate_filter(filter: &FilterPatch) -> Result<(), PatchValidationError> {
    check_range("engine.filter.cutoff_hz", filter.cutoff_hz, 20.0, 20_000.0)?;
    check_range("engine.filter.resonance", filter.resonance, 0.0, 1.0)?;
    check_range("engine.filter.drive", filter.drive, 0.0, 1.0)?;
    check_range("engine.filter.keytrack", filter.keytrack, 0.0, 1.0)?;
    Ok(())
}

fn validate_adsr(prefix: &'static str, adsr: &AdsrPatch) -> Result<(), PatchValidationError> {
    check_positive(&format_field(prefix, "attack_ms"), adsr.attack_ms)?;
    check_positive(&format_field(prefix, "decay_ms"), adsr.decay_ms)?;
    check_range(&format_field(prefix, "sustain"), adsr.sustain, 0.0, 1.0)?;
    check_positive(&format_field(prefix, "release_ms"), adsr.release_ms)?;
    Ok(())
}

fn validate_filter_env(filter_env: &FilterEnvPatch) -> Result<(), PatchValidationError> {
    validate_adsr("engine.filter_env", &filter_env.adsr)?;
    check_range("engine.filter_env.depth", filter_env.depth, 0.0, 1.0)?;
    Ok(())
}

fn validate_voice(voice: &VoicePatch) -> Result<(), PatchValidationError> {
    check_range("engine.voice.stereo_width", voice.stereo_width, 0.0, 1.0)?;
    check_range(
        "engine.voice.detune_spread_cents",
        voice.detune_spread_cents,
        0.0,
        50.0,
    )?;
    if let Some(value) = voice.velocity_to_level {
        check_range("engine.voice.velocity_to_level", value, 0.0, 1.0)?;
    }
    if let Some(value) = voice.velocity_to_filter {
        check_range("engine.voice.velocity_to_filter", value, 0.0, 1.0)?;
    }
    Ok(())
}

fn validate_final_stage(final_stage: &FinalStagePatch) -> Result<(), PatchValidationError> {
    check_range(
        "engine.final_stage.body_drive",
        final_stage.body_drive,
        0.0,
        1.0,
    )?;
    check_range(
        "engine.final_stage.asymmetry",
        final_stage.asymmetry,
        0.0,
        1.0,
    )?;
    check_range(
        "engine.final_stage.low_mid_emphasis",
        final_stage.low_mid_emphasis,
        0.0,
        1.0,
    )?;
    check_range(
        "engine.final_stage.output_trim_db",
        final_stage.output_trim_db,
        -24.0,
        12.0,
    )?;
    Ok(())
}

fn validate_fx(fx: &FxPatch) -> Result<(), PatchValidationError> {
    check_range("engine.fx.chorus.mix", fx.chorus.mix, 0.0, 1.0)?;
    check_range("engine.fx.chorus.depth", fx.chorus.depth, 0.0, 1.0)?;
    check_positive("engine.fx.chorus.rate_hz", fx.chorus.rate_hz)?;
    check_range("engine.fx.reverb.mix", fx.reverb.mix, 0.0, 1.0)?;
    check_range("engine.fx.reverb.size", fx.reverb.size, 0.0, 1.0)?;
    check_range("engine.fx.reverb.damping", fx.reverb.damping, 0.0, 1.0)?;
    Ok(())
}

fn validate_macro_defaults(defaults: &MacroDefaults) -> Result<(), PatchValidationError> {
    for macro_id in MacroId::ALL {
        check_range(macro_id.key(), defaults.get(macro_id), 0.0, 1.0)?;
    }
    Ok(())
}

fn validate_macro_response_set(
    response_set: &MacroResponseSet,
) -> Result<(), PatchValidationError> {
    for macro_id in MacroId::ALL {
        let spec = response_set.spec(macro_id);
        check_range("macro_response.sensitivity", spec.sensitivity, 0.0, 2.0)?;
        check_range("macro_response.softness", spec.softness, 0.0, 1.0)?;
    }
    Ok(())
}

fn validate_identity_bias(identity_bias: &IdentityBiasProfile) -> Result<(), PatchValidationError> {
    check_range(
        "identity_bias.horizont_bias",
        identity_bias.horizont_bias,
        -1.0,
        1.0,
    )?;
    check_range("identity_bias.pec_bias", identity_bias.pec_bias, -1.0, 1.0)?;
    check_range(
        "identity_bias.baklja_bias",
        identity_bias.baklja_bias,
        -1.0,
        1.0,
    )?;
    check_range(
        "identity_bias.gravitacija_pressure_bias",
        identity_bias.gravitacija_pressure_bias,
        -1.0,
        1.0,
    )?;
    check_range(
        "identity_bias.rupture_threshold_bias",
        identity_bias.rupture_threshold_bias,
        -1.0,
        1.0,
    )?;
    check_range(
        "identity_bias.body_focus_bias",
        identity_bias.body_focus_bias,
        -1.0,
        1.0,
    )?;
    Ok(())
}

fn validate_performance_response(
    performance_response: &PerformanceResponseProfile,
) -> Result<(), PatchValidationError> {
    check_range(
        "performance_response.velocity_to_level",
        performance_response.velocity_to_level,
        0.0,
        1.0,
    )?;
    check_range(
        "performance_response.velocity_to_filter",
        performance_response.velocity_to_filter,
        0.0,
        1.0,
    )?;
    check_range(
        "performance_response.aftertouch_to_gravitacija",
        performance_response.aftertouch_to_gravitacija,
        0.0,
        1.0,
    )?;
    check_range(
        "performance_response.aftertouch_to_baklja",
        performance_response.aftertouch_to_baklja,
        0.0,
        1.0,
    )?;
    check_range(
        "performance_response.mod_wheel_to_bloom",
        performance_response.mod_wheel_to_bloom,
        0.0,
        1.0,
    )?;
    check_range(
        "performance_response.mod_wheel_to_swarm",
        performance_response.mod_wheel_to_swarm,
        0.0,
        1.0,
    )?;
    check_integer_range(
        "performance_response.bend_range_semitones",
        performance_response.bend_range_semitones as i32,
        0,
        24,
    )?;
    Ok(())
}

fn validate_extensions(extensions: &Option<PatchExtensions>) -> Result<(), PatchValidationError> {
    if let Some(extensions) = extensions {
        for key in extensions.keys() {
            match key.as_str() {
                "macro_remap"
                | "macro_aliases"
                | "instrument_ontology"
                | "final_stage_bypass"
                | "gravitacija_mode" => {
                    return Err(PatchValidationError::PolicyViolation(
                        "x must not redefine macro meaning or instrument ontology",
                    ));
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn check_range(
    field: &'static str,
    value: f32,
    min: f32,
    max: f32,
) -> Result<(), PatchValidationError> {
    if (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(PatchValidationError::RangeViolation {
            field,
            min,
            max,
            value,
        })
    }
}

fn check_integer_range(
    field: &'static str,
    value: i32,
    min: i32,
    max: i32,
) -> Result<(), PatchValidationError> {
    if (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(PatchValidationError::IntegerRangeViolation {
            field,
            min,
            max,
            value,
        })
    }
}

fn check_positive(field: &'static str, value: f32) -> Result<(), PatchValidationError> {
    if value > 0.0 {
        Ok(())
    } else {
        Err(PatchValidationError::PositiveRequired { field, value })
    }
}

fn format_field(prefix: &'static str, suffix: &'static str) -> &'static str {
    match (prefix, suffix) {
        ("engine.amp_env", "attack_ms") => "engine.amp_env.attack_ms",
        ("engine.amp_env", "decay_ms") => "engine.amp_env.decay_ms",
        ("engine.amp_env", "sustain") => "engine.amp_env.sustain",
        ("engine.amp_env", "release_ms") => "engine.amp_env.release_ms",
        ("engine.filter_env", "attack_ms") => "engine.filter_env.attack_ms",
        ("engine.filter_env", "decay_ms") => "engine.filter_env.decay_ms",
        ("engine.filter_env", "sustain") => "engine.filter_env.sustain",
        ("engine.filter_env", "release_ms") => "engine.filter_env.release_ms",
        _ => "unknown_field",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOLTEN_HORIZON: &str = include_str!("../../../patches/factory/molten-horizon.toml");

    #[test]
    fn molten_horizon_round_trips() {
        let patch = load_patch_toml(MOLTEN_HORIZON).expect("fixture must parse");
        validate_patch_v1(&patch).expect("fixture must validate");

        let serialized = save_patch_toml(&patch).expect("fixture must serialize");
        let reparsed = load_patch_toml(&serialized).expect("round trip must parse");
        validate_patch_v1(&reparsed).expect("round trip must validate");

        assert_eq!(patch, reparsed);
    }

    #[test]
    fn rejects_unknown_field_in_strict_table() {
        let invalid = r#"
[meta]
schema_version = 1
patch_name = "Broken"

[engine.osc1]
saw_level = 0.5
pulse_level = 0.1
triangle_level = 0.1
noise_level = 0.0
bad_key = 2

[engine.osc2]
saw_level = 0.5
pulse_level = 0.1
triangle_level = 0.0
interval_semitones = 0
fine_tune_cents = 0.0
sync_amount = 0.0
crossmod_amount = 0.0

[engine.sub]
level = 0.3
octave_offset = -1

[engine.mixer]
pre_filter_drive = 0.2
body_mix = 0.2

[engine.filter]
cutoff_hz = 1000.0
resonance = 0.2
drive = 0.2
keytrack = 0.2

[engine.amp_env]
attack_ms = 10.0
decay_ms = 100.0
sustain = 0.8
release_ms = 300.0

[engine.filter_env]
attack_ms = 10.0
decay_ms = 100.0
sustain = 0.5
release_ms = 300.0
depth = 0.5

[engine.voice]
stereo_width = 0.5
detune_spread_cents = 4.0

[engine.final_stage]
body_drive = 0.1
asymmetry = 0.1
low_mid_emphasis = 0.2
output_trim_db = 0.0

[engine.fx.chorus]
enabled = true
mix = 0.2
depth = 0.2
rate_hz = 0.3

[engine.fx.reverb]
enabled = true
mix = 0.2
size = 0.2
damping = 0.2

[macros]
gravitacija = 0.1
bloom = 0.2
heat = 0.3
ruin = 0.4
swarm = 0.5

[macro_response.gravitacija]
sensitivity = 1.0
curve = "linear"
softness = 0.5

[macro_response.bloom]
sensitivity = 1.0
curve = "linear"
softness = 0.5

[macro_response.heat]
sensitivity = 1.0
curve = "linear"
softness = 0.5

[macro_response.ruin]
sensitivity = 1.0
curve = "linear"
softness = 0.5

[macro_response.swarm]
sensitivity = 1.0
curve = "linear"
softness = 0.5

[identity_bias]
horizont_bias = 0.0
pec_bias = 0.0
baklja_bias = 0.0
gravitacija_pressure_bias = 0.0
rupture_threshold_bias = 0.0
body_focus_bias = 0.0

[performance_response]
velocity_to_level = 0.2
velocity_to_filter = 0.2
aftertouch_to_gravitacija = 0.2
aftertouch_to_baklja = 0.2
mod_wheel_to_bloom = 0.2
mod_wheel_to_swarm = 0.2
bend_range_semitones = 2
"#;

        let error = load_patch_toml(invalid).expect_err("strict tables must reject unknown fields");
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn rejects_invalid_schema_version() {
        let mut patch = load_patch_toml(MOLTEN_HORIZON).expect("fixture must parse");
        patch.meta.schema_version = 2;

        let error = validate_patch_v1(&patch).expect_err("wrong schema version must fail");
        assert_eq!(
            error,
            PatchValidationError::UnsupportedSchemaVersion { found: 2 }
        );
    }

    #[test]
    fn rejects_out_of_range_values() {
        let mut patch = load_patch_toml(MOLTEN_HORIZON).expect("fixture must parse");
        patch.engine.filter.cutoff_hz = 25_000.0;

        let error = validate_patch_v1(&patch).expect_err("out of range cutoff must fail");
        assert_eq!(
            error,
            PatchValidationError::RangeViolation {
                field: "engine.filter.cutoff_hz",
                min: 20.0,
                max: 20_000.0,
                value: 25_000.0,
            }
        );
    }
}
