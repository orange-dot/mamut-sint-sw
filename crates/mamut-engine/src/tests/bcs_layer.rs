use super::*;

#[test]
fn bcs_layer_defaults_to_disabled_snapshot() {
    let engine = fixture_engine();
    let snapshot = engine.snapshot();

    assert_eq!(engine.bcs_layer_mode(), BcsLayerMode::Disabled);
    assert_eq!(snapshot.bcs_layer, BcsLayerSnapshot::default());
    assert!(!snapshot.bcs_layer.enabled);
    assert_eq!(snapshot.bcs_layer.amount, 0.0);
    assert_eq!(snapshot.bcs_layer.effective_amount, 0.0);
    assert_eq!(snapshot.bcs_layer.gain, 0.0);
    assert_eq!(snapshot.bcs_layer.effective_gain, 0.0);
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
    assert_eq!(layer_snapshot.gain, 0.0);
    assert_eq!(layer_snapshot.effective_gain, 0.0);

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
    assert_eq!(snapshot.bcs_layer.effective_gain, 0.0);
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
    assert_eq!(snapshot.bcs_layer.gain, 0.0);
    assert_eq!(snapshot.bcs_layer.effective_gain, 0.0);
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
    let (layer_signature, layer_stats) = render_engine_layer_signature_with_notes_and_controllers(
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
    assert!(snapshot.bcs_layer.gain > 0.70);
    assert!(snapshot.bcs_layer.effective_gain > 0.70);
    assert_eq!(snapshot.bcs_layer.pitch_note, Some(48));
    assert_eq!(snapshot.bcs_layer.unsafe_events, 0);
    assert!(!snapshot.bcs_layer.unsafe_state);
    assert!(snapshot.bcs_layer.max_state_abs < 32.0);
}

#[test]
fn bcs_layer_full_amount_leaves_audible_single_note_residual() {
    let frames = ENGINE_LAYER_TEST_RATE_HZ as usize * 3;
    let note_events = [Scheduled {
        frame_offset: 0,
        event: NoteEvent::NoteOn {
            note: 48,
            velocity: 0.78,
        },
    }];
    let patch = load_patch_toml(MOLTEN_HORIZON).expect("factory patch must parse");
    let mut baseline_engine = Engine::new(
        EngineConfig {
            sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
            max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
            voice_count: 6,
        },
        patch.clone(),
    )
    .expect("engine must validate");
    let (baseline, baseline_stats) =
        render_engine_layer_mono_samples(&mut baseline_engine, frames, &note_events, &[]);

    let mut bcs_engine = Engine::new(
        EngineConfig {
            sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
            max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
            voice_count: 6,
        },
        patch,
    )
    .expect("engine must validate");
    bcs_engine.set_bcs_layer_mode(BcsLayerMode::Enabled {
        scenario: BcsScenario::StableAnchor,
    });
    let (bcs, bcs_stats) = render_engine_layer_mono_samples(
        &mut bcs_engine,
        frames,
        &note_events,
        &bcs_layer_playable_controller_events(1.0),
    );
    let snapshot = bcs_engine.snapshot().bcs_layer;
    let analysis_start = ENGINE_LAYER_TEST_RATE_HZ as usize / 2;
    let baseline_rms = rms_window(&baseline[analysis_start..]);
    let residual_rms = rms_delta_window(&baseline[analysis_start..], &bcs[analysis_start..]);
    let residual_db = amplitude_ratio_db(residual_rms, baseline_rms);

    assert!(baseline_stats.finite);
    assert!(bcs_stats.finite);
    assert!(
        bcs_stats.peak_abs <= MASTER_SAFETY_CEILING,
        "stats={bcs_stats:?}"
    );
    assert_eq!(snapshot.unsafe_events, 0);
    assert!(!snapshot.unsafe_state);
    assert!(
        residual_db >= -14.0,
        "baseline_rms={baseline_rms} residual_rms={residual_rms} residual_db={residual_db}"
    );
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
