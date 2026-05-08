use super::*;

#[test]
fn resolve_factory_patch_by_stem() {
    let path = resolve_patch_argument(Some("molten-horizon")).expect("factory patch resolves");
    assert!(path.ends_with("patches/factory/molten-horizon.toml"));
}

#[test]
fn resolve_factory_patch_by_patch_name_slug() {
    let path = resolve_patch_argument(Some("Molten Horizon")).expect("patch name resolves");
    assert!(path.ends_with("patches/factory/molten-horizon.toml"));
}

#[test]
fn parse_play_options_supports_device_selection() {
    let args = vec![
        "--demo".to_string(),
        "--headless".to_string(),
        "--audio-device".to_string(),
        "hw:4,0".to_string(),
        "--sample-rate".to_string(),
        "96000".to_string(),
        "--alsa-period-frames".to_string(),
        "256".to_string(),
        "--alsa-buffer-frames".to_string(),
        "1024".to_string(),
        "--alsa-start-threshold-frames".to_string(),
        "1024".to_string(),
        "--midi-device".to_string(),
        "Launchkey".to_string(),
        "--midi-channel".to_string(),
        "3".to_string(),
        "--controller-profile".to_string(),
        "profiles/pc4-full.toml".to_string(),
        "--trace-midi".to_string(),
        "--gfm-layer-seed".to_string(),
        "0x6A464D40".to_string(),
        "--bcs-layer-scenario".to_string(),
        "subharmonic-pressure".to_string(),
        "razor-thaw".to_string(),
    ];

    let options = parse_play_options(&args).expect("play options parse");
    assert!(options.force_demo);
    assert!(options.headless);
    assert!(!options.gui);
    assert!(options.trace_midi);
    assert_eq!(options.audio_selector.as_deref(), Some("hw:4,0"));
    assert_eq!(options.sample_rate_hz, 96_000);
    assert_eq!(options.alsa_period_frames, Some(256));
    assert_eq!(options.alsa_buffer_frames, Some(1024));
    assert_eq!(options.alsa_start_threshold_frames, Some(1024));
    assert_eq!(options.midi_selector.as_deref(), Some("Launchkey"));
    assert_eq!(options.midi_channel, Some(3));
    assert_eq!(options.gfm_layer_seed, Some(0x6A46_4D40));
    assert_eq!(
        options.bcs_layer_scenario,
        Some(BcsScenario::SubharmonicPressure)
    );
    assert!(
        options
            .controller_profile_path
            .as_ref()
            .expect("profile path set")
            .ends_with("profiles/pc4-full.toml")
    );
    assert!(
        options
            .patch_path
            .ends_with("patches/factory/razor-thaw.toml")
    );
}

#[test]
fn parse_play_options_supports_gui_opt_in() {
    let args = vec!["--gui".to_string(), "razor-thaw".to_string()];
    let options = parse_play_options(&args).expect("play options parse");

    assert!(options.gui);
    assert!(!options.headless);
    assert!(
        options
            .patch_path
            .ends_with("patches/factory/razor-thaw.toml")
    );
}

#[test]
fn parse_play_options_defaults_gfm_layer_disabled() {
    let args = vec!["razor-thaw".to_string()];
    let options = parse_play_options(&args).expect("play options parse");

    assert_eq!(options.sample_rate_hz, ALSA_PLAYBACK_SAMPLE_RATE_HZ);
    assert_eq!(options.gfm_layer_seed, None);
    assert_eq!(options.bcs_layer_scenario, None);
}

#[test]
fn parse_play_options_accepts_supported_sample_rates() {
    for sample_rate_hz in ALSA_PLAYBACK_SAMPLE_RATE_HZ_ALLOWED {
        let args = vec![
            "--sample-rate".to_string(),
            sample_rate_hz.to_string(),
            "razor-thaw".to_string(),
        ];
        let options = parse_play_options(&args).expect("sample rate parses");
        assert_eq!(options.sample_rate_hz, sample_rate_hz);
    }
}

#[test]
fn parse_play_options_rejects_unsupported_sample_rates() {
    for sample_rate_hz in ["0", "98000", "196000", "forty-eight"] {
        let args = vec![
            "--sample-rate".to_string(),
            sample_rate_hz.to_string(),
            "razor-thaw".to_string(),
        ];
        assert!(parse_play_options(&args).is_err());
    }
    assert!(parse_play_options(&["--sample-rate".to_string()]).is_err());
}

#[test]
fn parse_dry_run_options_supports_gfm_layer_seed() {
    let args = vec![
        "--gfm-layer-seed".to_string(),
        "1782992192".to_string(),
        "--bcs-layer-scenario".to_string(),
        "recovery_return".to_string(),
        "cathedral-bloom".to_string(),
    ];
    let options = parse_dry_run_options(&args).expect("dry-run options parse");

    assert_eq!(options.gfm_layer_seed, Some(0x6A46_4D40));
    assert_eq!(
        options.bcs_layer_scenario,
        Some(BcsScenario::RecoveryReturn)
    );
    assert!(
        options
            .patch_path
            .ends_with("patches/factory/cathedral-bloom.toml")
    );
}

#[test]
fn parse_gfm_layer_seed_accepts_decimal_and_hex() {
    assert_eq!(
        parse_gfm_layer_seed("1782992192").expect("decimal seed parses"),
        0x6A46_4D40
    );
    assert_eq!(
        parse_gfm_layer_seed("0x6A464D40").expect("hex seed parses"),
        0x6A46_4D40
    );
    assert_eq!(
        parse_gfm_layer_seed("0x6A46_4D40").expect("underscored hex seed parses"),
        0x6A46_4D40
    );
}

#[test]
fn parse_gfm_layer_seed_rejects_invalid_values() {
    assert!(parse_gfm_layer_seed("not-a-seed").is_err());
    assert!(parse_gfm_layer_seed("0x").is_err());
    assert!(parse_play_options(&["--gfm-layer-seed".to_string()]).is_err());
}

#[test]
fn parse_bcs_layer_scenario_accepts_aliases_and_off() {
    assert_eq!(
        parse_optional_bcs_layer_scenario("stable-anchor").expect("stable parses"),
        Some(BcsScenario::StableAnchor)
    );
    assert_eq!(
        parse_optional_bcs_layer_scenario("edge_sweep").expect("edge parses"),
        Some(BcsScenario::EdgeSweep)
    );
    assert_eq!(
        parse_optional_bcs_layer_scenario("subharmonic").expect("subharmonic parses"),
        Some(BcsScenario::SubharmonicPressure)
    );
    assert_eq!(
        parse_optional_bcs_layer_scenario("off").expect("off parses"),
        None
    );
    assert!(parse_optional_bcs_layer_scenario("not-a-scenario").is_err());
    assert!(parse_play_options(&["--bcs-layer-scenario".to_string()]).is_err());
}
