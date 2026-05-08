#![allow(clippy::expect_used, clippy::module_inception)]

use super::*;

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
    fn old_factory_patch_loads_neutral_source_expansion_defaults() {
        let patch = load_patch_toml(MOLTEN_HORIZON).expect("fixture must parse");

        assert_eq!(patch.engine.osc1.pulse_width, 0.5);
        assert_eq!(patch.engine.osc1.pwm_depth, 0.0);
        assert_eq!(patch.engine.osc1.phase_mode, OscPhaseMode::Deterministic);
        assert_eq!(patch.engine.osc2.level, 1.0);
        assert_eq!(patch.engine.osc2.pitch_mode, Osc2PitchMode::Semitone);
        assert_eq!(patch.engine.noise_color, NoiseColor::White);
        assert_eq!(patch.engine.noise_filter_level, None);
        assert_eq!(patch.engine.noise_body_level, 0.0);
        assert_eq!(patch.engine.cross_mix_mode, CrossMixMode::Sum);
        assert_eq!(patch.engine.spectral, SpectralPatch::default());
        assert_eq!(patch.engine.additive, AdditivePatch::default());
    }

    #[test]
    fn source_expansion_fields_validate_and_round_trip() {
        let mut patch = load_patch_toml(MOLTEN_HORIZON).expect("fixture must parse");
        patch.engine.osc1.pulse_width = 0.37;
        patch.engine.osc1.pwm_depth = 0.42;
        patch.engine.osc1.phase_mode = OscPhaseMode::Fixed;
        patch.engine.osc1.start_phase = 0.25;
        patch.engine.osc1.saw_bend = -0.35;
        patch.engine.osc1.triangle_fold = 0.40;
        patch.engine.osc1.pulse_edge = 0.30;
        patch.engine.osc2.level = 1.35;
        patch.engine.osc2.pitch_mode = Osc2PitchMode::Ratio;
        patch.engine.osc2.ratio = 1.50;
        patch.engine.noise_color = NoiseColor::Pinkish;
        patch.engine.noise_filter_level = Some(0.18);
        patch.engine.noise_body_level = 0.22;
        patch.engine.fm_direction = ModDirection::Osc2ToOsc1;
        patch.engine.phase_mod_amount = 0.25;
        patch.engine.cross_mix_mode = CrossMixMode::Difference;
        patch.engine.cross_mix_amount = 0.45;
        patch.engine.spectral.level = 0.38;
        patch.engine.spectral.table = SpectralTable::Metallic;
        patch.engine.spectral.position = 0.72;
        patch.engine.spectral.morph = 0.44;
        patch.engine.spectral.ratio = 1.75;
        patch.engine.spectral.fine_tune_cents = -11.0;
        patch.engine.additive.level = 0.26;
        patch.engine.additive.partial_count = 8;
        patch.engine.additive.harmonic_spread = 0.35;
        patch.engine.additive.odd_even_balance = -0.45;
        patch.engine.additive.inharmonicity = 0.28;
        patch.engine.additive.spectral_tilt = 0.62;
        patch.engine.additive.random_detune_cents = 12.5;

        validate_patch_v1(&patch).expect("source controls validate");
        let serialized = save_patch_toml(&patch).expect("source controls serialize");
        let reparsed = load_patch_toml(&serialized).expect("source controls parse");

        assert_eq!(patch, reparsed);
    }

    #[test]
    fn rejects_invalid_source_expansion_enum_and_range() {
        let invalid_enum = MOLTEN_HORIZON.replace(
            "[engine.osc1]",
            "[engine]\nnoise_color = \"infrared\"\n\n[engine.osc1]",
        );
        assert!(load_patch_toml(&invalid_enum).is_err());
        let invalid_spectral_enum = MOLTEN_HORIZON.replace(
            "[engine.osc1]",
            "[engine.spectral]\ntable = \"glass\"\n\n[engine.osc1]",
        );
        assert!(load_patch_toml(&invalid_spectral_enum).is_err());

        let mut invalid_range = load_patch_toml(MOLTEN_HORIZON).expect("fixture must parse");
        invalid_range.engine.osc2.ratio = 0.05;
        assert!(validate_patch_v1(&invalid_range).is_err());

        let mut invalid_spectral_range =
            load_patch_toml(MOLTEN_HORIZON).expect("fixture must parse");
        invalid_spectral_range.engine.spectral.ratio = 8.0;
        assert!(validate_patch_v1(&invalid_spectral_range).is_err());

        let mut invalid_partial_count =
            load_patch_toml(MOLTEN_HORIZON).expect("fixture must parse");
        invalid_partial_count.engine.additive.partial_count = 3;
        assert!(validate_patch_v1(&invalid_partial_count).is_err());

        invalid_partial_count.engine.additive.partial_count = 9;
        assert!(validate_patch_v1(&invalid_partial_count).is_err());

        let mut invalid_odd_even = load_patch_toml(MOLTEN_HORIZON).expect("fixture must parse");
        invalid_odd_even.engine.additive.odd_even_balance = 1.5;
        assert!(validate_patch_v1(&invalid_odd_even).is_err());

        let mut invalid_random_detune =
            load_patch_toml(MOLTEN_HORIZON).expect("fixture must parse");
        invalid_random_detune.engine.additive.random_detune_cents = 36.0;
        assert!(validate_patch_v1(&invalid_random_detune).is_err());
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
