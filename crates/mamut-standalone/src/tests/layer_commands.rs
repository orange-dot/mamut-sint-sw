use super::*;

#[test]
fn gfm_layer_seed_from_ui_text_maps_enabled_disabled_and_invalid() {
    assert_eq!(
        gfm_layer_seed_from_ui_text(false, "not-a-seed").expect("disabled ignores text"),
        None
    );
    assert_eq!(
        gfm_layer_seed_from_ui_text(true, "0x6A46_4D40").expect("enabled seed parses"),
        Some(0x6A46_4D40)
    );
    assert!(gfm_layer_seed_from_ui_text(true, "not-a-seed").is_err());
}

#[test]
fn gfm_layer_status_line_reports_disabled_and_enabled_modes() {
    let disabled = gfm_layer_status_line(&test_snapshot());
    assert_eq!(disabled, "gfm: mode=disabled");

    let path = resolve_patch_argument(Some("ember-vault")).expect("factory patch resolves");
    let patch = load_patch_from_path(&path).expect("patch loads");
    let mut engine = Engine::new(EngineConfig::default(), patch).expect("engine builds");
    engine.set_gfm_layer_mode(GfmLayerMode::Enabled { seed: 0x6A46_4D40 });
    let enabled = gfm_layer_status_line(&engine.snapshot());

    assert!(enabled.contains("mode=enabled"));
    assert!(enabled.contains("seed=0x6A464D40"));
    assert!(enabled.contains("selected=PecPerformance"));
    assert!(enabled.contains("active=PecPerformance"));
}

#[test]
fn bcs_layer_status_line_reports_disabled_and_enabled_modes() {
    let disabled = bcs_layer_status_line(&test_snapshot());
    assert_eq!(
        disabled,
        "bcs: mode=disabled playable=off knob=0.000 gain=0.000 effective_gain=0.000"
    );

    let path = resolve_patch_argument(Some("ember-vault")).expect("factory patch resolves");
    let patch = load_patch_from_path(&path).expect("patch loads");
    let mut engine = Engine::new(EngineConfig::default(), patch).expect("engine builds");
    engine.set_bcs_layer_mode(BcsLayerMode::Enabled {
        scenario: BcsScenario::SubharmonicPressure,
    });
    let enabled = bcs_layer_status_line(&engine.snapshot());

    assert!(enabled.contains("mode=enabled"));
    assert!(enabled.contains("scenario=subharmonic-pressure"));
    assert!(enabled.contains("active=subharmonic-pressure"));
    assert!(enabled.contains("playable=off"));
    assert!(enabled.contains("knob=0.000"));
    assert!(enabled.contains("gain=0.000"));
    assert!(enabled.contains("effective_gain=0.000"));
    assert!(enabled.contains("unsafe_events=0"));
}

#[test]
fn engine_command_sets_gfm_layer_mode_enabled_and_disabled() {
    let mut state = test_engine_thread_state_for_patch("ember-vault");

    let (reply_tx, reply_rx) = mpsc::channel();
    assert!(state.handle_command(EngineCommand::SetGfmLayerMode(
        GfmLayerMode::Enabled { seed: 0x6A46_4D40 },
        reply_tx,
    )));
    let selection = reply_rx
        .recv_timeout(Duration::from_millis(50))
        .expect("enabled reply arrives")
        .expect("enabled command succeeds");
    assert_eq!(
        format_optional_program_id(selection.program_id),
        "PecPerformance"
    );

    let snapshot = state.engine.snapshot();
    assert_eq!(
        snapshot.gfm_layer.mode,
        GfmLayerMode::Enabled { seed: 0x6A46_4D40 }
    );
    assert_eq!(
        format_optional_program_id(snapshot.gfm_layer.active_program_id),
        "PecPerformance"
    );
    assert!(snapshot.gfm_layer.diagnostics.is_some());

    let (reply_tx, reply_rx) = mpsc::channel();
    assert!(state.handle_command(EngineCommand::SetGfmLayerMode(
        GfmLayerMode::Disabled,
        reply_tx,
    )));
    let selection = reply_rx
        .recv_timeout(Duration::from_millis(50))
        .expect("disabled reply arrives")
        .expect("disabled command succeeds");
    assert_eq!(format_optional_program_id(selection.program_id), "none");

    let snapshot = state.engine.snapshot();
    assert_eq!(snapshot.gfm_layer.mode, GfmLayerMode::Disabled);
    assert_eq!(
        format_optional_program_id(snapshot.gfm_layer.active_program_id),
        "none"
    );
    assert!(snapshot.gfm_layer.diagnostics.is_none());
}

#[test]
fn engine_command_sets_bcs_layer_mode_enabled_and_disabled() {
    let mut state = test_engine_thread_state_for_patch("ember-vault");

    let (reply_tx, reply_rx) = mpsc::channel();
    assert!(state.handle_command(EngineCommand::SetBcsLayerMode(
        BcsLayerMode::Enabled {
            scenario: BcsScenario::RecoveryReturn
        },
        reply_tx,
    )));
    let layer = reply_rx
        .recv_timeout(Duration::from_millis(50))
        .expect("enabled reply arrives")
        .expect("enabled command succeeds");
    assert_eq!(layer.active_scenario, Some(BcsScenario::RecoveryReturn));
    assert_eq!(layer.sample_rate_hz, Some(ALSA_PLAYBACK_SAMPLE_RATE_HZ));

    let snapshot = state.engine.snapshot();
    assert_eq!(
        snapshot.bcs_layer.mode,
        BcsLayerMode::Enabled {
            scenario: BcsScenario::RecoveryReturn
        }
    );
    assert_eq!(
        snapshot.bcs_layer.active_scenario,
        Some(BcsScenario::RecoveryReturn)
    );

    let (reply_tx, reply_rx) = mpsc::channel();
    assert!(state.handle_command(EngineCommand::SetBcsLayerMode(
        BcsLayerMode::Disabled,
        reply_tx,
    )));
    let layer = reply_rx
        .recv_timeout(Duration::from_millis(50))
        .expect("disabled reply arrives")
        .expect("disabled command succeeds");
    assert_eq!(layer, BcsLayerSnapshot::default());
    assert_eq!(
        state.engine.snapshot().bcs_layer,
        BcsLayerSnapshot::default()
    );
}
