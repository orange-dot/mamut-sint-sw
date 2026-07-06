#![allow(clippy::expect_used, clippy::panic, clippy::module_inception)]

mod tests {
    use crate::midi::{self, MidiChannel};
    use crate::profile::Profile;
    use crate::scenario::errors::ScenarioError;
    use crate::scenario::{self};

    const TEST_PROFILE: &str = r#"
name = "test"
version = 1

[[binding]]
control = "K8 GFM Gate"
cc = 3
section = "knob"
index = 8
kind = "gfm_layer_amount"

[[binding]]
control = "S9 BCS Gain"
cc = 28
section = "slider"
index = 9
kind = "bcs_layer_gain"

[[binding]]
control = "SW1"
cc = 80
section = "switch"
index = 1
kind = "runtime_action"
action = "panic"

[[binding]]
control = "SW2"
cc = 81
section = "switch"
index = 2
kind = "runtime_action"
action = "reset_controllers"
"#;

    // Reference fixtures — the locked profile and the first shipped scenario.
    const REAL_PROFILE: &str = include_str!("../../../profiles/pc4-full.toml");
    const GFM_GATE_ARM: &str = include_str!("../../../scenarios/gfm-gate-arm.toml");

    // The full shipped scenario library (SET3-3).
    const SHIPPED_SCENARIOS: &[(&str, &str)] = &[
        ("gfm-gate-arm", GFM_GATE_ARM),
        (
            "gfm-mapping-sweep",
            include_str!("../../../scenarios/gfm-mapping-sweep.toml"),
        ),
        (
            "gfm-host-load",
            include_str!("../../../scenarios/gfm-host-load.toml"),
        ),
        (
            "bcs-layer-basic",
            include_str!("../../../scenarios/bcs-layer-basic.toml"),
        ),
        (
            "masnoca-listening",
            include_str!("../../../scenarios/masnoca-listening.toml"),
        ),
        (
            "factory-bank-pass",
            include_str!("../../../scenarios/factory-bank-pass.toml"),
        ),
    ];

    fn channel(number: u8) -> MidiChannel {
        MidiChannel::new(number).expect("valid channel")
    }

    fn test_profile() -> Profile {
        Profile::from_toml(TEST_PROFILE, "test").expect("test profile parses")
    }

    // ---- midi encoder --------------------------------------------------

    #[test]
    fn encodes_channel_voice_messages() {
        let ch2 = channel(2); // wire nibble 1
        assert_eq!(midi::note_on(ch2, 60, 100), [0x91, 60, 100]);
        assert_eq!(midi::note_off(ch2, 60), [0x81, 60, 0]);
        assert_eq!(midi::control_change(ch2, 3, 127), [0xB1, 3, 127]);
        assert_eq!(midi::channel_aftertouch(ch2, 64), [0xD1, 64]);
        assert_eq!(midi::program_change(ch2, 4), [0xC1, 4]);
        assert_eq!(midi::note_on(channel(1), 60, 100), [0x90, 60, 100]);
        assert_eq!(midi::note_on(channel(16), 60, 100), [0x9F, 60, 100]);
    }

    #[test]
    fn note_on_velocity_zero_stays_zero() {
        // A note-on with velocity 0 is a note-off by convention; we still emit
        // the literal bytes the caller asked for.
        assert_eq!(midi::note_on(channel(2), 60, 0), [0x91, 60, 0]);
    }

    #[test]
    fn pitch_bend_center_and_extremes() {
        let ch = channel(1);
        assert_eq!(
            midi::pitch_bend(ch, midi::PITCH_BEND_CENTER),
            [0xE0, 0x00, 0x40]
        );
        assert_eq!(midi::pitch_bend(ch, 0), [0xE0, 0x00, 0x00]);
        assert_eq!(
            midi::pitch_bend(ch, midi::PITCH_BEND_MAX),
            [0xE0, 0x7F, 0x7F]
        );
        // Over-range clamps to max.
        assert_eq!(midi::pitch_bend(ch, 0xFFFF), [0xE0, 0x7F, 0x7F]);
        // Normalized centre.
        assert_eq!(midi::pitch_bend_norm(ch, 0.0), [0xE0, 0x00, 0x40]);
        assert_eq!(midi::pitch_bend_norm(ch, 1.0), [0xE0, 0x7F, 0x7F]);
        assert_eq!(midi::pitch_bend_norm(ch, -1.0), [0xE0, 0x00, 0x00]);
    }

    #[test]
    fn clamp7_saturates() {
        assert_eq!(midi::clamp7(0.0), 0);
        assert_eq!(midi::clamp7(1.0), 127);
        assert_eq!(midi::clamp7(2.0), 127);
        assert_eq!(midi::clamp7(-1.0), 0);
        assert_eq!(midi::clamp7(0.5), 64);
    }

    // ---- profile mirror ------------------------------------------------

    #[test]
    fn profile_resolves_full_name_and_token() {
        let profile = test_profile();
        assert_eq!(profile.cc_for("K8 GFM Gate"), Some(3));
        assert_eq!(profile.cc_for("K8"), Some(3));
        assert_eq!(profile.cc_for("S9 BCS Gain"), Some(28));
        assert_eq!(profile.cc_for("S9"), Some(28));
        assert_eq!(profile.cc_for("nope"), None);
    }

    #[test]
    fn profile_extracts_panic_and_reset_controls() {
        let profile = test_profile();
        assert_eq!(profile.panic_cc(), Some(80));
        assert_eq!(profile.reset_controllers_cc(), Some(81));
    }

    #[test]
    fn locked_profile_parses_and_ignores_extra_fields() {
        let profile = Profile::from_toml(REAL_PROFILE, "profiles/pc4-full.toml")
            .expect("locked profile parses");
        assert_eq!(profile.cc_for("K8 GFM Gate"), Some(3));
        assert_eq!(profile.cc_for("S9"), Some(28));
        assert_eq!(profile.panic_cc(), Some(80));
        assert_eq!(profile.reset_controllers_cc(), Some(81));
    }

    // ---- scenario schema + expansion -----------------------------------

    const NOTE_SCENARIO: &str = r#"
schema_version = 1
name = "notes"

[[timeline]]
kind = "program_change"
program = 4

[[timeline]]
kind = "wait"
ms = 100

[[timeline]]
kind = "chord"
notes = [60, 64, 67]
velocity = 100
duration_ms = 500

[[timeline]]
kind = "cc"
control = "K8 GFM Gate"
value = 0.9
"#;

    #[test]
    fn expand_is_deterministic() {
        let profile = test_profile();
        let scenario = scenario::load_from_str(NOTE_SCENARIO).expect("parses");
        let first = scenario::expand(&scenario, &profile, channel(2)).expect("expands");
        let second = scenario::expand(&scenario, &profile, channel(2)).expect("expands");
        assert_eq!(first, second);
    }

    #[test]
    fn note_off_pairing_and_timing() {
        let profile = test_profile();
        let scenario = scenario::load_from_str(NOTE_SCENARIO).expect("parses");
        let events = scenario::expand(&scenario, &profile, channel(2)).expect("expands");

        // program_change at 0, chord note-ons at 100, note-offs at 600.
        let note_ons: Vec<_> = events
            .iter()
            .filter(|e| e.bytes.first() == Some(&0x91) && e.bytes.get(2) != Some(&0))
            .collect();
        let note_offs: Vec<_> = events
            .iter()
            .filter(|e| e.bytes.first() == Some(&0x81))
            .collect();
        assert_eq!(note_ons.len(), 3);
        assert_eq!(note_offs.len(), 3);
        assert!(note_ons.iter().all(|e| e.at_ms == 100));
        assert!(note_offs.iter().all(|e| e.at_ms == 600));

        // The CC lands at the chord cursor (100) with the resolved cc 3.
        assert!(events.iter().any(|e| e.at_ms == 100
            && e.bytes.first() == Some(&0xB1)
            && e.bytes.get(1) == Some(&3)));

        // Events are sorted by absolute time.
        assert!(events.windows(2).all(|w| w[0].at_ms <= w[1].at_ms));
    }

    #[test]
    fn cc_value_clamps() {
        let profile = test_profile();
        let over = r#"
schema_version = 1
name = "clamp"
[[timeline]]
kind = "cc"
control = "K8 GFM Gate"
value = 2.0
"#;
        let scenario = scenario::load_from_str(over).expect("parses");
        let events = scenario::expand(&scenario, &profile, channel(2)).expect("expands");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].bytes, vec![0xB1, 3, 127]);
    }

    #[test]
    fn mod_wheel_and_sustain_encode_standard_ccs() {
        let profile = test_profile();
        let scenario = scenario::load_from_str(
            r#"
schema_version = 1
name = "std-cc"
[[timeline]]
kind = "mod_wheel"
value = 1.0
[[timeline]]
kind = "sustain"
down = true
[[timeline]]
kind = "sustain"
down = false
"#,
        )
        .expect("parses");
        let events = scenario::expand(&scenario, &profile, channel(2)).expect("expands");
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].bytes, vec![0xB1, 1, 127]); // mod wheel CC1
        assert_eq!(events[1].bytes, vec![0xB1, 64, 127]); // sustain down CC64
        assert_eq!(events[2].bytes, vec![0xB1, 64, 0]); // sustain up CC64
    }

    #[test]
    fn loop_expands_count_times() {
        let profile = test_profile();
        let looped = r#"
schema_version = 1
name = "loop"

[[timeline]]
kind = "loop"
count = 3

[[timeline.steps]]
kind = "note_on"
note = 60

[[timeline.steps]]
kind = "wait"
ms = 10
"#;
        let scenario = scenario::load_from_str(looped).expect("parses");
        let events = scenario::expand(&scenario, &profile, channel(2)).expect("expands");
        let note_ons = events
            .iter()
            .filter(|e| e.bytes.first() == Some(&0x91))
            .count();
        assert_eq!(note_ons, 3);
        // Cursor advanced 10 ms per iteration: note-ons at 0, 10, 20.
        let times: Vec<u64> = events
            .iter()
            .filter(|e| e.bytes.first() == Some(&0x91))
            .map(|e| e.at_ms)
            .collect();
        assert_eq!(times, vec![0, 10, 20]);
    }

    #[test]
    fn unknown_control_is_a_hard_error() {
        let profile = test_profile();
        let bad = r#"
schema_version = 1
name = "bad"
[[timeline]]
kind = "cc"
control = "NOPE"
value = 0.5
"#;
        let scenario = scenario::load_from_str(bad).expect("parses");
        assert_eq!(
            scenario::expand(&scenario, &profile, channel(2)),
            Err(ScenarioError::UnknownControl {
                control: "NOPE".to_string(),
                profile: "test".to_string(),
            })
        );
    }

    #[test]
    fn rejects_wrong_schema_version() {
        let bad = r#"
schema_version = 2
name = "bad"
[[timeline]]
kind = "wait"
ms = 1
"#;
        let scenario = scenario::load_from_str(bad).expect("parses");
        assert_eq!(
            scenario::validate_scenario(&scenario),
            Err(ScenarioError::UnsupportedSchemaVersion { found: 2 })
        );
    }

    #[test]
    fn rejects_program_out_of_range() {
        let bad = r#"
schema_version = 1
name = "bad"
[[timeline]]
kind = "program_change"
program = 8
"#;
        let scenario = scenario::load_from_str(bad).expect("parses");
        assert_eq!(
            scenario::validate_scenario(&scenario),
            Err(ScenarioError::ProgramOutOfRange { value: 8 })
        );
    }

    #[test]
    fn rejects_unknown_step_field() {
        // `velocty` typo must be caught (deny_unknown_fields is not honored on
        // the tagged Step enum, so a post-parse key check does it).
        let bad = r#"
schema_version = 1
name = "typo"
[[timeline]]
kind = "note"
note = 60
velocty = 10
"#;
        let error = scenario::load_from_str(bad).expect_err("typo must be rejected");
        assert!(format!("{error}").contains("velocty"), "{error}");
    }

    #[test]
    fn rejects_non_finite_values() {
        let bad = r#"
schema_version = 1
name = "nan"
[[timeline]]
kind = "cc"
control = "K8 GFM Gate"
value = nan
"#;
        let scenario = scenario::load_from_str(bad).expect("parses");
        assert_eq!(
            scenario::validate_scenario(&scenario),
            Err(ScenarioError::NonFiniteValue { field: "cc value" })
        );
    }

    #[test]
    fn rejects_oversized_ramp_steps() {
        let bad = r#"
schema_version = 1
name = "big-ramp"
[[timeline]]
kind = "cc_ramp"
control = "K8 GFM Gate"
from = 0.0
to = 1.0
duration_ms = 100
steps = 100000
"#;
        let scenario = scenario::load_from_str(bad).expect("parses");
        assert!(matches!(
            scenario::validate_scenario(&scenario),
            Err(ScenarioError::RampStepsTooLarge { .. })
        ));
    }

    #[test]
    fn expansion_budget_bounds_runaway() {
        let profile = test_profile();
        // 200 max-width ramps => ~819k events; the 500k event cap trips first.
        let evil = r#"
schema_version = 1
name = "boom"
[[timeline]]
kind = "loop"
count = 200
[[timeline.steps]]
kind = "cc_ramp"
control = "K8 GFM Gate"
from = 0.0
to = 1.0
duration_ms = 10
steps = 4096
"#;
        let scenario = scenario::load_from_str(evil).expect("parses");
        scenario::validate_scenario(&scenario).expect("validates");
        assert!(matches!(
            scenario::expand(&scenario, &profile, channel(2)),
            Err(ScenarioError::ExpansionBudgetExceeded { .. })
        ));
    }

    #[test]
    fn aftertouch_envelope_has_no_seam_duplicate() {
        let profile = test_profile();
        let env = r#"
schema_version = 1
name = "env"
[[timeline]]
kind = "aftertouch_envelope"
peak = 1.0
attack_ms = 100
hold_ms = 100
release_ms = 100
steps = 4
"#;
        let scenario = scenario::load_from_str(env).expect("parses");
        let events = scenario::expand(&scenario, &profile, channel(2)).expect("expands");
        // attack (4) + hold (1) + release (4) = 9, no duplicate at the seam.
        assert_eq!(events.len(), 9);
        // The single peak sits at the attack/hold seam (t = attack_ms).
        let peaks: Vec<_> = events
            .iter()
            .filter(|e| e.bytes == vec![0xD1, 127])
            .collect();
        assert_eq!(peaks.len(), 1);
        assert_eq!(peaks[0].at_ms, 100);
    }

    #[test]
    fn rejects_empty_timeline() {
        let bad = r#"
schema_version = 1
name = "bad"
"#;
        let scenario = scenario::load_from_str(bad).expect("parses");
        assert_eq!(
            scenario::validate_scenario(&scenario),
            Err(ScenarioError::EmptyTimeline)
        );
    }

    // ---- reference scenario against the locked profile -----------------

    #[test]
    fn all_shipped_scenarios_validate_and_expand() {
        let profile = Profile::from_toml(REAL_PROFILE, "profiles/pc4-full.toml")
            .expect("locked profile parses");
        for (name, source) in SHIPPED_SCENARIOS {
            let scenario = scenario::load_from_str(source)
                .unwrap_or_else(|error| panic!("{name} must parse: {error}"));
            assert_eq!(scenario.name, *name, "scenario name should match file");
            scenario::validate_scenario(&scenario)
                .unwrap_or_else(|error| panic!("{name} must validate: {error}"));
            let events = scenario::expand(&scenario, &profile, channel(2))
                .unwrap_or_else(|error| panic!("{name} must expand: {error}"));
            assert!(!events.is_empty(), "{name} must produce events");
            // Every emitted program change stays within the locked 8-slot set.
            for event in &events {
                if event.bytes.first().is_some_and(|b| b & 0xF0 == 0xC0) {
                    assert!(
                        event.bytes.get(1).is_some_and(|p| *p <= 7),
                        "{name}: program change out of 0..=7"
                    );
                }
            }
        }
    }

    #[test]
    fn reference_scenario_expands_against_locked_profile() {
        let profile = Profile::from_toml(REAL_PROFILE, "profiles/pc4-full.toml")
            .expect("locked profile parses");
        let scenario = scenario::load_from_str(GFM_GATE_ARM).expect("scenario parses");
        scenario::validate_scenario(&scenario).expect("scenario validates");
        let events = scenario::expand(&scenario, &profile, channel(2)).expect("expands");
        assert!(!events.is_empty());
        // K8 GFM Gate resolves to CC 3 on channel 2 (0xB1).
        assert!(
            events
                .iter()
                .any(|e| e.bytes.first() == Some(&0xB1) && e.bytes.get(1) == Some(&3))
        );
        // Deterministic re-expansion.
        let again = scenario::expand(&scenario, &profile, channel(2)).expect("expands");
        assert_eq!(events, again);
    }
}
