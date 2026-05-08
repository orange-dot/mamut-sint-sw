use super::*;

#[test]
fn gfm_field_voice_matches_direct_field_pcm() {
    for program_id in GfmProgramId::ALL_PERFORMANCE {
        let (adapter_signature, adapter_stats) = render_gfm_adapter_signature(program_id);
        let (direct_signature, direct_stats) = render_direct_field_signature(program_id);

        assert_eq!(adapter_signature, direct_signature);
        assert_eq!(adapter_stats.probe_position, direct_stats.probe_position);
        assert_eq!(adapter_stats.frames, direct_stats.frames);
        assert_eq!(adapter_stats.final_frame_index, adapter_stats.frames);
        assert_eq!(
            adapter_stats.max_rupture_count,
            direct_stats.max_rupture_count
        );
        assert_eq!(adapter_stats.health, direct_stats.health);
    }
}

#[test]
fn gfm_field_voice_dry_run_is_finite_bounded_and_recovery_safe() {
    for program_id in GfmProgramId::ALL_PERFORMANCE {
        let (_, stats) = render_gfm_adapter_signature(program_id);

        assert!(stats.finite);
        assert!(stats.peak_abs <= 1.0);
        match program_id {
            GfmProgramId::HorizontPerformance | GfmProgramId::PecPerformance => {
                assert_eq!(stats.max_rupture_count, 0);
                assert_eq!(stats.health.suspect, 0);
                assert_eq!(stats.health.quarantined, 0);
            }
            GfmProgramId::BakljaPerformance => {
                let recovered = stats.health.healthy + stats.health.recovering;
                assert!(stats.max_rupture_count > 0);
                assert!(stats.max_rupture_count <= (GFM_V1_WIDTH * GFM_V1_HEIGHT * 2) / 5);
                assert!(recovered >= (GFM_V1_WIDTH * GFM_V1_HEIGHT * 3) / 4);
            }
        }
    }
}

#[test]
fn gfm_field_voice_live_pressure_changes_excitation_and_stays_bounded() {
    for program_id in GfmProgramId::ALL_PERFORMANCE {
        let (plain_signature, plain_stats) =
            render_gfm_adapter_signature_with_live_pressure(program_id, 0.0, 4_096);
        let (pressured_signature, pressured_stats) =
            render_gfm_adapter_signature_with_live_pressure(program_id, 0.85, 4_096);

        assert_ne!(pressured_signature, plain_signature);
        assert!(plain_stats.finite);
        assert!(pressured_stats.finite);
        assert!(pressured_stats.peak_abs <= 1.0);
    }
}

#[test]
fn gfm_field_voice_mono_block_matches_sample_step_and_direct_field() {
    for program_id in GfmProgramId::ALL_PERFORMANCE {
        let (sample_signature, sample_stats) = render_gfm_adapter_signature(program_id);
        let (block_signature, block_stats) = render_gfm_block_signature(program_id);
        let (direct_signature, direct_stats) = render_direct_field_signature(program_id);

        assert_eq!(block_signature, sample_signature);
        assert_eq!(block_signature, direct_signature);
        assert_eq!(block_stats.probe_position, sample_stats.probe_position);
        assert_eq!(block_stats.probe_position, direct_stats.probe_position);
        assert_eq!(block_stats.final_frame_index, block_stats.frames);
        assert_eq!(
            block_stats.max_rupture_count,
            sample_stats.max_rupture_count
        );
        assert_eq!(block_stats.health, direct_stats.health);
    }
}

#[test]
fn gfm_field_voice_stereo_block_is_dual_mono() {
    let mut voice = GfmFieldVoice::new(
        GfmProgramId::BakljaPerformance,
        GFM_PERFORMANCE_BASELINE_SEED,
        GFM_TEST_RATE_HZ,
    );
    let mut left = [0.0_f32; 257];
    let mut right = [0.0_f32; 257];

    voice.render_stereo_block(&mut left, &mut right);

    assert_eq!(voice.frame_index(), 257);
    for (left, right) in left.iter().zip(right.iter()) {
        assert_eq!(left.to_bits(), right.to_bits());
        assert!(left.is_finite());
    }
}

#[test]
fn gfm_field_voice_chunking_is_invariant() {
    for program_id in GfmProgramId::ALL_PERFORMANCE {
        let (one_block_signature, one_block_stats) = render_gfm_block_signature(program_id);

        for chunk_size in [17, 64, 251] {
            let (chunked_signature, chunked_stats) =
                render_gfm_chunked_signature(program_id, chunk_size);
            assert_eq!(chunked_signature, one_block_signature);
            assert_eq!(chunked_stats.frames, one_block_stats.frames);
            assert_eq!(
                chunked_stats.final_frame_index,
                one_block_stats.final_frame_index
            );
            assert_eq!(
                chunked_stats.max_rupture_count,
                one_block_stats.max_rupture_count
            );
            assert_eq!(chunked_stats.health, one_block_stats.health);
        }
    }
}

#[test]
fn gfm_program_selection_is_deterministic_for_factory_patches() {
    let mut saw_horizont = false;
    let mut saw_pec = false;
    let mut saw_baklja = false;

    for (_, patch_source) in FACTORY_PATCHES {
        let patch = load_patch_toml(patch_source).expect("factory patch must parse");
        let left = select_gfm_program_for_patch(&patch);
        let right = select_gfm_program_for_patch(&patch);

        assert_eq!(left, right);
        assert!(left.best_score() >= 0.15);
        match left
            .program_id
            .expect("factory patch should select a GFM program")
        {
            GfmProgramId::HorizontPerformance => saw_horizont = true,
            GfmProgramId::PecPerformance => saw_pec = true,
            GfmProgramId::BakljaPerformance => saw_baklja = true,
        }
    }

    assert!(saw_horizont);
    assert!(saw_pec);
    assert!(saw_baklja);
}

#[test]
fn selected_factory_gfm_programs_render_finite_and_recovery_safe() {
    for (_, patch_source) in FACTORY_PATCHES {
        let patch = load_patch_toml(patch_source).expect("factory patch must parse");
        let decision = GfmFieldVoice::from_patch(&patch, gfm_patch_voice_test_config());
        let selection = decision.selection();
        let program_id = selection
            .program_id
            .expect("factory patch should select a GFM program");
        let (_, stats) = render_gfm_voice_block_signature(
            decision
                .into_voice()
                .expect("selected patch should build a GFM voice"),
        );

        assert!(stats.finite);
        assert!(stats.peak_abs <= 1.0);
        match program_id {
            GfmProgramId::HorizontPerformance | GfmProgramId::PecPerformance => {
                assert_eq!(stats.max_rupture_count, 0);
            }
            GfmProgramId::BakljaPerformance => {
                let recovered = stats.health.healthy + stats.health.recovering;
                assert!(stats.max_rupture_count > 0);
                assert!(recovered >= (GFM_V1_WIDTH * GFM_V1_HEIGHT * 3) / 4);
            }
        }
    }
}

#[test]
fn molten_horizon_renders_finite_at_96khz() {
    let patch = load_patch_toml(MOLTEN_HORIZON).expect("factory patch must parse");
    let note_events = engine_layer_note_on_events(select_gfm_program_for_patch(&patch).program_id);
    let mut engine = Engine::new(
        EngineConfig {
            sample_rate_hz: 96_000.0,
            max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
            voice_count: 6,
        },
        patch,
    )
    .expect("engine must validate");
    let controllers = [Scheduled {
        frame_offset: 0,
        event: ControllerEvent::Macro {
            id: MacroId::Gravitacija,
            value: 0.72,
        },
    }];

    let (_, stats) = render_engine_layer_signature_with_notes_and_controllers(
        &mut engine,
        96_000,
        note_events,
        &controllers,
    );
    let snapshot = engine.snapshot();

    assert_eq!(snapshot.sample_rate_hz, 96_000.0);
    assert!(stats.finite, "stats={stats:?}");
    assert!(stats.rms > 0.001, "stats={stats:?}");
    assert!(stats.peak_abs <= MASTER_SAFETY_CEILING, "stats={stats:?}");
    assert!(!snapshot.clip_detected, "snapshot={snapshot:?}");
}

#[test]
fn gfm_patch_voice_factory_matches_direct_selection_render() {
    for (_, patch_source) in FACTORY_PATCHES {
        let patch = load_patch_toml(patch_source).expect("factory patch must parse");
        let selection = select_gfm_program_for_patch(&patch);
        let program_id = selection
            .program_id
            .expect("factory patch should select a GFM program");
        let decision = GfmFieldVoice::from_patch(&patch, gfm_patch_voice_test_config());
        let (factory_signature, factory_stats) = render_gfm_voice_block_signature(
            decision
                .into_voice()
                .expect("selected patch should build a GFM voice"),
        );
        let (direct_signature, direct_stats) = render_gfm_selected_patch_signature(program_id);

        assert_eq!(factory_signature, direct_signature);
        assert_eq!(factory_stats.probe_position, direct_stats.probe_position);
        assert_eq!(factory_stats.frames, direct_stats.frames);
        assert_eq!(
            factory_stats.final_frame_index,
            direct_stats.final_frame_index
        );
        assert_eq!(
            factory_stats.max_rupture_count,
            direct_stats.max_rupture_count
        );
        assert_eq!(factory_stats.health, direct_stats.health);
    }
}

#[test]
fn gfm_patch_voice_factory_is_deterministic_for_patch_seed_and_rate() {
    for (_, patch_source) in FACTORY_PATCHES {
        let patch = load_patch_toml(patch_source).expect("factory patch must parse");
        let left = GfmFieldVoice::from_patch(&patch, gfm_patch_voice_test_config());
        let right = GfmFieldVoice::from_patch(&patch, gfm_patch_voice_test_config());

        assert_eq!(left.config(), right.config());
        assert_eq!(left.selection(), right.selection());

        let (left_signature, left_stats) = render_gfm_voice_block_signature(
            left.into_voice()
                .expect("selected patch should build a GFM voice"),
        );
        let (right_signature, right_stats) = render_gfm_voice_block_signature(
            right
                .into_voice()
                .expect("selected patch should build a GFM voice"),
        );

        assert_eq!(left_signature, right_signature);
        assert_eq!(left_stats.frames, right_stats.frames);
        assert_eq!(left_stats.final_frame_index, right_stats.final_frame_index);
        assert_eq!(left_stats.max_rupture_count, right_stats.max_rupture_count);
        assert_eq!(left_stats.health, right_stats.health);
    }
}

#[test]
fn gfm_patch_voice_factory_disables_low_score_patch() {
    let patch = low_score_gfm_patch();
    let decision = GfmFieldVoice::from_patch(&patch, gfm_patch_voice_test_config());

    assert!(!decision.is_enabled());
    assert_eq!(decision.selection().program_id, None);
    assert!(decision.selection().best_score() < GfmVoiceProgramSelection::MIN_PROGRAM_SCORE);
    assert!(decision.into_voice().is_none());
}
