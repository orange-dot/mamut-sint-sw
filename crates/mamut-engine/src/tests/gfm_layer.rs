use super::*;

#[test]
fn gfm_layer_defaults_to_disabled_snapshot() {
    let engine = fixture_engine();
    let snapshot = engine.snapshot();

    assert_eq!(engine.gfm_layer_mode(), GfmLayerMode::Disabled);
    assert_eq!(snapshot.gfm_layer.mode, GfmLayerMode::Disabled);
    assert_eq!(
        snapshot.gfm_layer.selection,
        GfmVoiceProgramSelection::default()
    );
    assert_eq!(snapshot.gfm_layer.active_program_id, None);
    assert!(snapshot.gfm_layer.diagnostics.is_none());
    assert_eq!(snapshot.gfm_layer.amount, 0.0);
    assert_eq!(snapshot.gfm_layer.pressure, 0.0);
    assert_eq!(snapshot.gfm_layer.effective_amount, 0.0);
    assert!(engine.gfm_layer_diagnostics().is_none());
}

#[test]
fn gfm_layer_gate_without_aftertouch_does_not_auto_arm() {
    let patch = load_patch_toml(CATHEDRAL_BLOOM).expect("factory patch must parse");
    let mut engine = Engine::new(
        EngineConfig {
            sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
            max_block_frames: ENGINE_LAYER_TEST_RATE_HZ as usize,
            voice_count: 6,
        },
        patch,
    )
    .expect("engine must validate");

    engine.process_block(ProcessBlock {
        frame_count: 8,
        note_events: &[],
        controller_events: &[Scheduled {
            frame_offset: 0,
            event: ControllerEvent::GfmLayerAmount { amount: 0.65 },
        }],
        macro_state: None,
        output: None,
    });
    let snapshot = engine.snapshot();

    assert_eq!(snapshot.gfm_layer.mode, GfmLayerMode::Disabled);
    assert_eq!(snapshot.gfm_layer.amount, 0.65);
    assert_eq!(snapshot.gfm_layer.pressure, 0.0);
    assert_eq!(snapshot.gfm_layer.effective_amount, 0.0);
    assert_eq!(snapshot.gfm_layer.active_program_id, None);
    assert!(snapshot.gfm_layer.diagnostics.is_none());
}

#[test]
fn gfm_layer_gate_zero_disarms_auto_armed_layer_after_release() {
    let patch = load_patch_toml(CATHEDRAL_BLOOM).expect("factory patch must parse");
    let mut engine = Engine::new(
        EngineConfig {
            sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
            max_block_frames: ENGINE_LAYER_TEST_RATE_HZ as usize,
            voice_count: 6,
        },
        patch,
    )
    .expect("engine must validate");

    engine.process_block(ProcessBlock {
        frame_count: 8,
        note_events: &[],
        controller_events: &[
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::GfmLayerAmount { amount: 0.65 },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::ChannelAftertouch { pressure: 0.72 },
            },
        ],
        macro_state: None,
        output: None,
    });
    assert!(matches!(
        engine.snapshot().gfm_layer.mode,
        GfmLayerMode::Enabled { .. }
    ));

    engine.process_block(ProcessBlock {
        frame_count: 8,
        note_events: &[],
        controller_events: &[Scheduled {
            frame_offset: 0,
            event: ControllerEvent::GfmLayerAmount { amount: 0.0 },
        }],
        macro_state: None,
        output: None,
    });
    engine.process_block(ProcessBlock {
        frame_count: smoothing_sample_count(
            ENGINE_LAYER_TEST_RATE_HZ as f32,
            GFM_LAYER_MIX_RELEASE_MS,
        ) + 8,
        note_events: &[],
        controller_events: &[],
        macro_state: None,
        output: None,
    });
    let snapshot = engine.snapshot();

    assert_eq!(snapshot.gfm_layer.mode, GfmLayerMode::Disabled);
    assert_eq!(snapshot.gfm_layer.amount, 0.0);
    assert_eq!(snapshot.gfm_layer.pressure, 0.72);
    assert_eq!(snapshot.gfm_layer.effective_amount, 0.0);
    assert_eq!(snapshot.gfm_layer.active_program_id, None);
    assert!(snapshot.gfm_layer.diagnostics.is_none());
}

#[test]
fn gfm_layer_amount_zero_keeps_manual_gui_enabled_layer_ready() {
    let patch = load_patch_toml(CATHEDRAL_BLOOM).expect("factory patch must parse");
    let mut engine = Engine::new(
        EngineConfig {
            sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
            max_block_frames: ENGINE_LAYER_TEST_RATE_HZ as usize,
            voice_count: 6,
        },
        patch,
    )
    .expect("engine must validate");
    engine.set_gfm_layer_mode(GfmLayerMode::Enabled {
        seed: GFM_PERFORMANCE_BASELINE_SEED,
    });

    engine.process_block(ProcessBlock {
        frame_count: 8,
        note_events: &[],
        controller_events: &[Scheduled {
            frame_offset: 0,
            event: ControllerEvent::GfmLayerAmount { amount: 0.0 },
        }],
        macro_state: None,
        output: None,
    });
    let snapshot = engine.snapshot();

    assert_eq!(
        snapshot.gfm_layer.mode,
        GfmLayerMode::Enabled {
            seed: GFM_PERFORMANCE_BASELINE_SEED
        }
    );
    assert_eq!(snapshot.gfm_layer.amount, 0.0);
    assert_eq!(snapshot.gfm_layer.effective_amount, 0.0);
    assert!(snapshot.gfm_layer.active_program_id.is_some());
    assert!(snapshot.gfm_layer.diagnostics.is_some());
}

#[test]
fn gfm_layer_momentary_gate_smooths_in_and_out() {
    let patch = load_patch_toml(CATHEDRAL_BLOOM).expect("factory patch must parse");
    let mut engine = Engine::new(
        EngineConfig {
            sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
            max_block_frames: ENGINE_LAYER_TEST_RATE_HZ as usize,
            voice_count: 6,
        },
        patch,
    )
    .expect("engine must validate");
    let controller_events = [
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::GfmLayerAmount { amount: 1.0 },
        },
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::ChannelAftertouch { pressure: 1.0 },
        },
    ];

    engine.process_block(ProcessBlock {
        frame_count: 1,
        note_events: &[],
        controller_events: &controller_events,
        macro_state: None,
        output: None,
    });
    let first = engine.snapshot();
    assert!(first.gfm_layer.effective_amount > 0.0);
    assert!(
        first.gfm_layer.effective_amount < 0.01,
        "first smoothed GFM amount should be tiny, got {}",
        first.gfm_layer.effective_amount
    );

    engine.process_block(ProcessBlock {
        frame_count: smoothing_sample_count(
            ENGINE_LAYER_TEST_RATE_HZ as f32,
            GFM_LAYER_MIX_ATTACK_MS,
        ),
        note_events: &[],
        controller_events: &[],
        macro_state: None,
        output: None,
    });
    let open = engine.snapshot();
    assert!(open.gfm_layer.effective_amount > 0.95);

    engine.process_block(ProcessBlock {
        frame_count: 1,
        note_events: &[],
        controller_events: &[Scheduled {
            frame_offset: 0,
            event: ControllerEvent::GfmLayerAmount { amount: 0.0 },
        }],
        macro_state: None,
        output: None,
    });
    let first_release = engine.snapshot();
    assert!(matches!(
        first_release.gfm_layer.mode,
        GfmLayerMode::Enabled { .. }
    ));
    assert!(first_release.gfm_layer.effective_amount > 0.90);

    engine.process_block(ProcessBlock {
        frame_count: smoothing_sample_count(
            ENGINE_LAYER_TEST_RATE_HZ as f32,
            GFM_LAYER_MIX_RELEASE_MS,
        ) + 8,
        note_events: &[],
        controller_events: &[],
        macro_state: None,
        output: None,
    });
    let closed = engine.snapshot();
    assert_eq!(closed.gfm_layer.mode, GfmLayerMode::Disabled);
    assert_eq!(closed.gfm_layer.effective_amount, 0.0);
}

#[test]
fn gfm_layer_midi_controls_can_reintroduce_layer_without_gui_enable() {
    let patch = load_patch_toml(CATHEDRAL_BLOOM).expect("factory patch must parse");
    let frames = ENGINE_LAYER_TEST_RATE_HZ as usize * 2;
    let note_events = engine_layer_note_on_events(select_gfm_program_for_patch(&patch).program_id);
    let aftertouch_only = [
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::GfmLayerAmount { amount: 0.0 },
        },
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::ChannelAftertouch { pressure: 0.72 },
        },
    ];
    let momentary_gfm = [
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::GfmLayerAmount { amount: 1.0 },
        },
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::ChannelAftertouch { pressure: 0.72 },
        },
    ];
    let mut baseline_engine = Engine::new(
        EngineConfig {
            sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
            max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
            voice_count: 6,
        },
        patch.clone(),
    )
    .expect("engine must validate");
    let (baseline_signature, _) = render_engine_layer_signature_with_notes_and_controllers(
        &mut baseline_engine,
        frames,
        note_events,
        &aftertouch_only,
    );
    let mut engine = Engine::new(
        EngineConfig {
            sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
            max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
            voice_count: 6,
        },
        patch,
    )
    .expect("engine must validate");

    let (layer_signature, layer_stats) = render_engine_layer_signature_with_notes_and_controllers(
        &mut engine,
        frames,
        note_events,
        &momentary_gfm,
    );
    let snapshot = engine.snapshot();

    assert_ne!(layer_signature, baseline_signature);
    assert_eq!(
        snapshot.gfm_layer.mode,
        GfmLayerMode::Enabled {
            seed: DEFAULT_GFM_LAYER_SEED
        }
    );
    assert!(snapshot.gfm_layer.effective_amount > 0.0);
    assert!(layer_stats.finite);
    assert!(layer_stats.gfm_program_id.is_some());
}

#[test]
fn gfm_layer_low_score_enabled_patch_matches_baseline_engine_render() {
    let patch = low_score_gfm_patch();
    let frames = ENGINE_LAYER_TEST_RATE_HZ as usize * 2;
    let (baseline_signature, baseline_stats) =
        render_engine_layer_signature(patch.clone(), None, frames);
    let (layer_signature, layer_stats) =
        render_engine_layer_signature(patch, Some(GFM_PERFORMANCE_BASELINE_SEED), frames);

    assert_eq!(layer_signature, baseline_signature);
    assert_eq!(layer_stats.gfm_selection.program_id, None);
    assert_eq!(layer_stats.gfm_program_id, None);
    assert!(layer_stats.gfm_diagnostics.is_none());
    assert_eq!(layer_stats.finite, baseline_stats.finite);
    assert_eq!(
        layer_stats.peak_abs.to_bits(),
        baseline_stats.peak_abs.to_bits()
    );
}

#[test]
fn gfm_layer_enabled_without_momentary_controls_matches_baseline_engine_render() {
    for patch_source in [CATHEDRAL_BLOOM, EMBER_VAULT, RAZOR_THAW] {
        let patch = load_patch_toml(patch_source).expect("factory patch must parse");
        let frames = ENGINE_LAYER_TEST_RATE_HZ as usize * 2;
        let (baseline_signature, baseline_stats) =
            render_engine_layer_signature(patch.clone(), None, frames);
        let (armed_signature, armed_stats) =
            render_engine_layer_signature(patch, Some(GFM_PERFORMANCE_BASELINE_SEED), frames);

        assert_eq!(armed_signature, baseline_signature);
        assert_eq!(
            armed_stats.peak_abs.to_bits(),
            baseline_stats.peak_abs.to_bits()
        );
        assert!(armed_stats.gfm_program_id.is_some());
        assert!(armed_stats.gfm_diagnostics.is_some());
    }
}

#[test]
fn gfm_layer_enable_clears_stale_momentary_controls_without_clearing_aftertouch_macros() {
    let patch = load_patch_toml(CATHEDRAL_BLOOM).expect("factory patch must parse");
    let mut engine = Engine::new(
        EngineConfig {
            sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
            max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
            voice_count: 6,
        },
        patch,
    )
    .expect("engine must validate");
    let controller_events = [
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::GfmLayerAmount { amount: 1.0 },
        },
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::ChannelAftertouch { pressure: 0.72 },
        },
    ];

    engine.process_block(ProcessBlock {
        frame_count: 8,
        note_events: &[],
        controller_events: &controller_events,
        macro_state: None,
        output: None,
    });
    let before = engine.snapshot();
    assert_eq!(before.gfm_layer.amount, 1.0);
    assert_eq!(before.gfm_layer.pressure, 0.72);
    assert!(before.effective_macros.gravitacija > before.live_macros.gravitacija);

    engine.set_gfm_layer_mode(GfmLayerMode::Enabled {
        seed: GFM_PERFORMANCE_BASELINE_SEED,
    });
    let after = engine.snapshot();

    assert_eq!(after.gfm_layer.amount, 0.0);
    assert_eq!(after.gfm_layer.pressure, 0.0);
    assert_eq!(after.gfm_layer.effective_amount, 0.0);
    assert_eq!(
        after.effective_macros.gravitacija.to_bits(),
        before.effective_macros.gravitacija.to_bits()
    );
}

#[test]
fn gfm_layer_changes_enabled_patch_when_momentary_pressure_is_present() {
    for patch_source in [CATHEDRAL_BLOOM, EMBER_VAULT, RAZOR_THAW] {
        let patch = load_patch_toml(patch_source).expect("factory patch must parse");
        let frames = ENGINE_LAYER_TEST_RATE_HZ as usize * 2;
        let (baseline_signature, _) = render_engine_layer_signature(patch.clone(), None, frames);
        let note_events =
            engine_layer_note_on_events(select_gfm_program_for_patch(&patch).program_id);
        let mut engine = Engine::new(
            EngineConfig {
                sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
                max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
                voice_count: 6,
            },
            patch,
        )
        .expect("engine must validate");
        engine.set_gfm_layer_mode(GfmLayerMode::Enabled {
            seed: GFM_PERFORMANCE_BASELINE_SEED,
        });
        let controller_events = [
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::GfmLayerAmount { amount: 1.0 },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::ChannelAftertouch { pressure: 0.72 },
            },
        ];
        let (layer_signature, layer_stats) =
            render_engine_layer_signature_with_notes_and_controllers(
                &mut engine,
                frames,
                note_events,
                &controller_events,
            );

        assert_ne!(layer_signature, baseline_signature);
        assert!(layer_stats.finite);
        assert!(layer_stats.rms > 0.0);
        assert!(
            layer_stats.peak_abs <= MASTER_SAFETY_CEILING,
            "stats={layer_stats:?}"
        );
        assert!(layer_stats.gfm_program_id.is_some());
    }
}

#[test]
fn gfm_layer_disabled_after_enabled_matches_baseline_engine_render() {
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

    engine.set_gfm_layer_mode(GfmLayerMode::Enabled {
        seed: GFM_PERFORMANCE_BASELINE_SEED,
    });
    engine.set_gfm_layer_mode(GfmLayerMode::Disabled);
    let (disabled_signature, disabled_stats) =
        render_engine_layer_signature_for_engine(&mut engine, frames);

    assert_eq!(disabled_signature, baseline_signature);
    assert_eq!(disabled_stats.gfm_selection, baseline_stats.gfm_selection);
    assert_eq!(disabled_stats.gfm_program_id, None);
    assert!(disabled_stats.gfm_diagnostics.is_none());
    assert_eq!(
        disabled_stats.peak_abs.to_bits(),
        baseline_stats.peak_abs.to_bits()
    );
}

#[test]
fn gfm_layer_amount_zero_mutes_layer_without_disabling_voice() {
    let patch = load_patch_toml(CATHEDRAL_BLOOM).expect("factory patch must parse");
    let frames = ENGINE_LAYER_TEST_RATE_HZ as usize * 2;
    let note_events = engine_layer_note_on_events(select_gfm_program_for_patch(&patch).program_id);
    let controller_events = [
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::GfmLayerAmount { amount: 0.0 },
        },
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::ChannelAftertouch { pressure: 0.85 },
        },
    ];
    let mut baseline_engine = Engine::new(
        EngineConfig {
            sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
            max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
            voice_count: 6,
        },
        patch.clone(),
    )
    .expect("engine must validate");
    let (baseline_signature, baseline_stats) =
        render_engine_layer_signature_with_notes_and_controllers(
            &mut baseline_engine,
            frames,
            note_events,
            &controller_events,
        );
    let mut engine = Engine::new(
        EngineConfig {
            sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
            max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
            voice_count: 6,
        },
        patch,
    )
    .expect("engine must validate");
    engine.set_gfm_layer_mode(GfmLayerMode::Enabled {
        seed: GFM_PERFORMANCE_BASELINE_SEED,
    });

    let (muted_signature, muted_stats) = render_engine_layer_signature_with_notes_and_controllers(
        &mut engine,
        frames,
        note_events,
        &controller_events,
    );
    let snapshot = engine.snapshot();

    assert_eq!(muted_signature, baseline_signature);
    assert_eq!(
        muted_stats.peak_abs.to_bits(),
        baseline_stats.peak_abs.to_bits()
    );
    assert_eq!(snapshot.gfm_layer.amount, 0.0);
    assert_eq!(snapshot.gfm_layer.pressure, 0.85);
    assert_eq!(snapshot.gfm_layer.effective_amount, 0.0);
    assert_eq!(
        snapshot.gfm_layer.active_program_id,
        Some(GfmProgramId::HorizontPerformance)
    );
    assert!(snapshot.gfm_layer.diagnostics.is_some());
}

#[derive(Clone, Copy)]
struct GfmLiveSweepScenario {
    amount: f32,
    pressure: f32,
}

const GFM_LIVE_LOW: GfmLiveSweepScenario = GfmLiveSweepScenario {
    amount: 0.34,
    pressure: 0.32,
};
const GFM_LIVE_MID: GfmLiveSweepScenario = GfmLiveSweepScenario {
    amount: 0.68,
    pressure: 0.62,
};
const GFM_LIVE_HIGH: GfmLiveSweepScenario = GfmLiveSweepScenario {
    amount: 1.0,
    pressure: 0.90,
};

fn render_gfm_layer_live_sweep(
    patch_source: &str,
    scenario: GfmLiveSweepScenario,
) -> (u64, EngineLayerRenderStats) {
    let patch = load_patch_toml(patch_source).expect("factory patch must parse");
    let note_events = engine_layer_note_on_events(select_gfm_program_for_patch(&patch).program_id);
    let mut engine = Engine::new(
        EngineConfig {
            sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
            max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
            voice_count: 6,
        },
        patch,
    )
    .expect("engine must validate");
    engine.set_gfm_layer_mode(GfmLayerMode::Enabled {
        seed: GFM_PERFORMANCE_BASELINE_SEED,
    });
    let controller_events = [
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::GfmLayerAmount {
                amount: scenario.amount,
            },
        },
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::ChannelAftertouch {
                pressure: scenario.pressure,
            },
        },
    ];

    render_engine_layer_signature_with_notes_and_controllers(
        &mut engine,
        ENGINE_LAYER_TEST_RATE_HZ as usize * 3,
        note_events,
        &controller_events,
    )
}

fn gfm_live_rupture_profile(stats: EngineLayerRenderStats) -> usize {
    let diagnostics = stats
        .gfm_diagnostics
        .expect("GFM diagnostics must be present");
    diagnostics.max_rupture_count
        + diagnostics.health.suspect
        + diagnostics.health.recovering
        + diagnostics.health.quarantined
}

#[test]
fn gfm_layer_live_sweep_separates_horizont_without_rupture() {
    let (low_signature, low_stats) = render_gfm_layer_live_sweep(CATHEDRAL_BLOOM, GFM_LIVE_LOW);
    let (mid_signature, mid_stats) = render_gfm_layer_live_sweep(CATHEDRAL_BLOOM, GFM_LIVE_MID);
    let (high_signature, high_stats) = render_gfm_layer_live_sweep(CATHEDRAL_BLOOM, GFM_LIVE_HIGH);

    assert_ne!(low_signature, mid_signature);
    assert_ne!(mid_signature, high_signature);
    assert!(
        (high_stats.rms - low_stats.rms).abs() > 0.0001,
        "low={low_stats:?} high={high_stats:?}"
    );
    for stats in [low_stats, mid_stats, high_stats] {
        let diagnostics = stats
            .gfm_diagnostics
            .expect("GFM diagnostics must be present");
        assert!(stats.finite, "stats={stats:?}");
        assert!(stats.peak_abs <= MASTER_SAFETY_CEILING, "stats={stats:?}");
        assert_eq!(
            stats.gfm_program_id,
            Some(GfmProgramId::HorizontPerformance)
        );
        assert_eq!(diagnostics.max_rupture_count, 0);
    }
}

#[test]
fn gfm_layer_live_sweep_separates_pec_without_rupture() {
    let (low_signature, low_stats) = render_gfm_layer_live_sweep(EMBER_VAULT, GFM_LIVE_LOW);
    let (mid_signature, mid_stats) = render_gfm_layer_live_sweep(EMBER_VAULT, GFM_LIVE_MID);
    let (high_signature, high_stats) = render_gfm_layer_live_sweep(EMBER_VAULT, GFM_LIVE_HIGH);

    assert_ne!(low_signature, mid_signature);
    assert_ne!(mid_signature, high_signature);
    assert!(
        high_stats.rms > low_stats.rms,
        "low={low_stats:?} high={high_stats:?}"
    );
    for stats in [low_stats, mid_stats, high_stats] {
        let diagnostics = stats
            .gfm_diagnostics
            .expect("GFM diagnostics must be present");
        assert!(stats.finite, "stats={stats:?}");
        assert!(stats.peak_abs <= MASTER_SAFETY_CEILING, "stats={stats:?}");
        assert_eq!(stats.gfm_program_id, Some(GfmProgramId::PecPerformance));
        assert_eq!(diagnostics.max_rupture_count, 0);
    }
}

#[test]
fn gfm_layer_live_sweep_scales_baklja_rupture() {
    let (low_signature, low_stats) = render_gfm_layer_live_sweep(RAZOR_THAW, GFM_LIVE_LOW);
    let (mid_signature, mid_stats) = render_gfm_layer_live_sweep(RAZOR_THAW, GFM_LIVE_MID);
    let (high_signature, high_stats) = render_gfm_layer_live_sweep(RAZOR_THAW, GFM_LIVE_HIGH);
    let low_ruptures = low_stats
        .gfm_diagnostics
        .expect("GFM diagnostics must be present")
        .max_rupture_count;
    let mid_ruptures = mid_stats
        .gfm_diagnostics
        .expect("GFM diagnostics must be present")
        .max_rupture_count;
    let high_ruptures = high_stats
        .gfm_diagnostics
        .expect("GFM diagnostics must be present")
        .max_rupture_count;

    assert_ne!(low_signature, mid_signature);
    assert_ne!(mid_signature, high_signature);
    assert!(high_ruptures > 0);
    assert!(low_ruptures != mid_ruptures || mid_ruptures != high_ruptures);
    assert!(gfm_live_rupture_profile(high_stats) > gfm_live_rupture_profile(low_stats));
    assert!(gfm_live_rupture_profile(high_stats) > gfm_live_rupture_profile(mid_stats));
    for stats in [low_stats, mid_stats, high_stats] {
        assert!(stats.finite, "stats={stats:?}");
        assert!(stats.peak_abs <= MASTER_SAFETY_CEILING, "stats={stats:?}");
        assert_eq!(stats.gfm_program_id, Some(GfmProgramId::BakljaPerformance));
    }
}

#[test]
fn gfm_layer_is_finite_bounded_and_recovery_safe() {
    let short_frames = ENGINE_LAYER_TEST_RATE_HZ as usize * 2;

    for patch_source in [CATHEDRAL_BLOOM, EMBER_VAULT] {
        let patch = load_patch_toml(patch_source).expect("factory patch must parse");
        let (_, stats) =
            render_engine_layer_signature(patch, Some(GFM_PERFORMANCE_BASELINE_SEED), short_frames);
        let diagnostics = stats
            .gfm_diagnostics
            .expect("selected patch should keep GFM diagnostics");

        assert!(stats.finite);
        assert!(stats.peak_abs <= MASTER_SAFETY_CEILING, "stats={stats:?}");
        assert_eq!(diagnostics.max_rupture_count, 0);
    }

    let patch = load_patch_toml(RAZOR_THAW).expect("factory patch must parse");
    let note_events = engine_layer_note_on_events(select_gfm_program_for_patch(&patch).program_id);
    let mut engine = Engine::new(
        EngineConfig {
            sample_rate_hz: ENGINE_LAYER_RECOVERY_RATE_HZ as f32,
            max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
            voice_count: 6,
        },
        patch,
    )
    .expect("engine must validate");
    engine.set_gfm_layer_mode(GfmLayerMode::Enabled {
        seed: GFM_PERFORMANCE_BASELINE_SEED,
    });
    let frames = ENGINE_LAYER_RECOVERY_RATE_HZ as usize * GFM_PERFORMANCE_DURATION_SECONDS;
    let controllers = [
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::GfmLayerAmount { amount: 1.0 },
        },
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::ChannelAftertouch { pressure: 0.9 },
        },
    ];
    let (_, stats) = render_engine_layer_signature_with_notes_and_controllers(
        &mut engine,
        frames,
        note_events,
        &controllers,
    );
    let diagnostics = stats
        .gfm_diagnostics
        .expect("selected patch should keep GFM diagnostics");
    let recovered = diagnostics.health.healthy + diagnostics.health.recovering;

    assert!(stats.finite);
    assert!(stats.peak_abs <= MASTER_SAFETY_CEILING, "stats={stats:?}");
    assert_eq!(stats.gfm_program_id, Some(GfmProgramId::BakljaPerformance));
    assert!(diagnostics.max_rupture_count > 0);
    assert!(
        recovered >= (GFM_V1_WIDTH * GFM_V1_HEIGHT * 3) / 4,
        "diagnostics={diagnostics:?}"
    );
}

#[test]
fn gfm_layer_snapshot_exposes_mode_selection_program_and_diagnostics() {
    let patch = load_patch_toml(EMBER_VAULT).expect("factory patch must parse");
    let mut engine = Engine::new(
        EngineConfig {
            sample_rate_hz: ENGINE_LAYER_TEST_RATE_HZ as f32,
            max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
            voice_count: 6,
        },
        patch,
    )
    .expect("engine must validate");
    let mode = GfmLayerMode::Enabled {
        seed: GFM_PERFORMANCE_BASELINE_SEED,
    };

    let selection = engine.set_gfm_layer_mode(mode);
    let snapshot = engine.snapshot();

    assert_eq!(engine.gfm_layer_mode(), mode);
    assert_eq!(snapshot.gfm_layer.mode, mode);
    assert_eq!(snapshot.gfm_layer.selection, selection);
    assert_eq!(
        snapshot.gfm_layer.active_program_id,
        Some(GfmProgramId::PecPerformance)
    );
    assert_eq!(
        snapshot.gfm_layer.controls,
        Some(crate::gfm_layer::gfm_controls_from_patch(
            engine.patch.engine.gfm
        ))
    );
    assert!(snapshot.gfm_layer.diagnostics.is_some());
    assert_eq!(snapshot.gfm_layer.amount, 0.0);
    assert_eq!(snapshot.gfm_layer.pressure, 0.0);
    assert_eq!(snapshot.gfm_layer.effective_amount, 0.0);
    assert_eq!(
        snapshot.gfm_layer.diagnostics,
        engine.gfm_layer_diagnostics()
    );
}

#[test]
fn gfm_layer_rebuilds_when_patch_loads() {
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

    let first_selection = engine.set_gfm_layer_mode(GfmLayerMode::Enabled {
        seed: GFM_PERFORMANCE_BASELINE_SEED,
    });
    assert_eq!(
        first_selection.program_id,
        Some(GfmProgramId::HorizontPerformance)
    );

    engine
        .load_patch(second_patch)
        .expect("patch must validate");

    assert_eq!(
        engine.snapshot().gfm_layer.selection.program_id,
        Some(GfmProgramId::BakljaPerformance)
    );
    assert_eq!(
        engine.snapshot().gfm_layer.active_program_id,
        Some(GfmProgramId::BakljaPerformance)
    );
}
