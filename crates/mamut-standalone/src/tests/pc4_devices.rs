use super::*;

#[test]
fn pc4_control_display_exposes_passive_visual_meter_values() {
    let profile = pc4_full_profile();
    let snapshot = test_snapshot();
    let s1 = profile.binding_for_cc(12).expect("s1 binding");
    let k3 = profile.binding_for_cc(72).expect("k3 binding");
    let sw5 = profile.binding_for_cc(85).expect("sw5 binding");

    let s1_display = pc4_control_display(&snapshot, s1);
    assert!((0.0..=1.0).contains(&s1_display.normalized));
    assert!(s1_display.short_action.contains("Osc1 Saw"));

    let k3_display = pc4_control_display(&snapshot, k3);
    assert!((0.0..=1.0).contains(&k3_display.normalized));
    assert!(k3_display.value.contains("ms"));

    let sw5_display = pc4_control_display(&snapshot, sw5);
    assert!((0.0..=1.0).contains(&sw5_display.normalized));
    assert!(matches!(sw5_display.value.as_str(), "on" | "off"));
}

#[test]
fn pc4_knob_angle_uses_seven_to_five_clock_sweep() {
    let epsilon = 0.0001;

    assert!((pc4_knob_angle(0.0) - (2.0 * std::f32::consts::PI / 3.0)).abs() < epsilon);
    assert!((pc4_knob_angle(0.5) - (3.0 * std::f32::consts::PI / 2.0)).abs() < epsilon);
    assert!((pc4_knob_angle(1.0) - (7.0 * std::f32::consts::PI / 3.0)).abs() < epsilon);
}

#[test]
fn normalize_alsa_hw_selector_rejects_non_hw_paths() {
    assert_eq!(
        normalize_alsa_hw_selector("hw:4,0").expect("hw selector normalizes"),
        "hw:4,0"
    );
    assert!(normalize_alsa_hw_selector("default").is_err());
    assert!(normalize_alsa_hw_selector("pipewire").is_err());
    assert!(normalize_alsa_hw_selector("plughw:4,0").is_err());
}

#[test]
fn same_hw_selector_matches_normalized_hw_pairs() {
    assert!(same_hw_selector(Some("hw:04,0"), "hw:4,0"));
    assert!(!same_hw_selector(Some("hw:4,0"), "hw:5,0"));
    assert!(!same_hw_selector(Some("default"), "hw:4,0"));
}

#[test]
fn alsa_tuning_rejects_invalid_relationships() {
    assert!(
        AlsaPlaybackTuning {
            period_frames: 512,
            buffer_frames: 256,
            start_threshold_frames: 256,
        }
        .validate()
        .is_err()
    );
    assert!(
        AlsaPlaybackTuning {
            period_frames: 256,
            buffer_frames: 1024,
            start_threshold_frames: 2048,
        }
        .validate()
        .is_err()
    );
}

#[test]
fn parse_alsa_proc_lines_extract_expected_fields() {
    assert_eq!(
        parse_alsa_card_line(" 4 [AG06AG03       ]: USB-Audio - AG06/AG03"),
        Some((4, "AG06/AG03".to_string()))
    );
    assert_eq!(
        parse_alsa_pcm_line("04-00: USB Audio : USB Audio : playback 1 : capture 1"),
        Some((4, 0, "USB Audio".to_string(), true))
    );
}

#[test]
fn parse_runtime_ui_command_supports_patch_and_macro_commands() {
    assert_eq!(
        parse_runtime_ui_command("patch Cathedral Bloom").expect("patch command parses"),
        RuntimeUiCommand::Patch("Cathedral Bloom".to_string())
    );
    assert_eq!(
        parse_runtime_ui_command("favorite 3").expect("favorite command parses"),
        RuntimeUiCommand::Favorite(3)
    );
    assert_eq!(
        parse_runtime_ui_command("macro gravitacija 0.74").expect("macro command parses"),
        RuntimeUiCommand::Macro(MacroId::Gravitacija, 0.74)
    );
    assert_eq!(
        parse_runtime_ui_command("panic").expect("panic parses"),
        RuntimeUiCommand::Panic
    );
    assert_eq!(
        parse_runtime_ui_command("bcs edge-sweep").expect("bcs parses"),
        RuntimeUiCommand::BcsLayer(Some(BcsScenario::EdgeSweep))
    );
    assert_eq!(
        parse_runtime_ui_command("bcs off").expect("bcs off parses"),
        RuntimeUiCommand::BcsLayer(None)
    );
    assert_eq!(
        parse_runtime_ui_command("audio Scarlett").expect("audio command parses"),
        RuntimeUiCommand::AudioSelect("Scarlett".to_string())
    );
    assert_eq!(
        parse_runtime_ui_command("midi 1").expect("midi command parses"),
        RuntimeUiCommand::MidiSelect("1".to_string())
    );
    assert_eq!(
        parse_runtime_ui_command("next").expect("next parses"),
        RuntimeUiCommand::NextFavorite
    );
    assert_eq!(
        parse_runtime_ui_command("demo-patch").expect("demo patch parses"),
        RuntimeUiCommand::DemoPatch
    );
    assert_eq!(
        parse_runtime_ui_command("record 10").expect("record command parses"),
        RuntimeUiCommand::Record {
            seconds: 10,
            path: None,
        }
    );
    assert_eq!(
        parse_runtime_ui_command("record 70 /tmp/mamut-take.wav")
            .expect("record command with path parses"),
        RuntimeUiCommand::Record {
            seconds: 70,
            path: Some(PathBuf::from("/tmp/mamut-take.wav")),
        }
    );
    assert_eq!(
        parse_runtime_ui_command("record-stop").expect("record stop parses"),
        RuntimeUiCommand::RecordStop
    );
    assert!(parse_runtime_ui_command("record 0").is_err());
}
