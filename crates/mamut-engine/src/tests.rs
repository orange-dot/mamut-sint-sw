use super::*;
use mamut_dsp::{DENORMAL_FLUSH_ABS, MASTER_SAFETY_CEILING};
use mamut_field::{
    GFM_PERFORMANCE_BASELINE_SEED, GFM_PERFORMANCE_DURATION_SECONDS, GFM_V1_HEIGHT, GFM_V1_WIDTH,
    GfmHealthHistogram, GfmLattice16, GfmPerformanceGesture, GfmPerformanceProgram, GfmProgramId,
    sample_to_pcm16,
};
use mamut_patch::{
    CrossMixMode, NoiseColor, Osc2PitchMode, OscPhaseMode, SpectralTable, load_patch_toml,
};

const MOLTEN_HORIZON: &str = include_str!("../../../patches/factory/molten-horizon.toml");
const CATHEDRAL_BLOOM: &str = include_str!("../../../patches/factory/cathedral-bloom.toml");
const EMBER_VAULT: &str = include_str!("../../../patches/factory/ember-vault.toml");
const FURNACE_CHOIR: &str = include_str!("../../../patches/factory/furnace-choir.toml");
const GLASS_TIDE: &str = include_str!("../../../patches/factory/glass-tide.toml");
const GRANITE_PLAIN: &str = include_str!("../../../patches/factory/granite-plain.toml");
const GRAVITY_WAKE: &str = include_str!("../../../patches/factory/gravity-wake.toml");
const RAZOR_THAW: &str = include_str!("../../../patches/factory/razor-thaw.toml");
const SAWYER_REZZ: &str = include_str!("../../../patches/factory/sawyer-rezz.toml");
const GFM_TEST_RATE_HZ: u32 = 1_000;
const ENGINE_LAYER_TEST_RATE_HZ: u32 = 8_000;
const ENGINE_LAYER_RECOVERY_RATE_HZ: u32 = 48_000;
const ENGINE_LAYER_BLOCK_FRAMES: usize = 256;
const FACTORY_PATCHES: [(&str, &str); 9] = [
    ("cathedral-bloom", CATHEDRAL_BLOOM),
    ("ember-vault", EMBER_VAULT),
    ("furnace-choir", FURNACE_CHOIR),
    ("glass-tide", GLASS_TIDE),
    ("granite-plain", GRANITE_PLAIN),
    ("gravity-wake", GRAVITY_WAKE),
    ("molten-horizon", MOLTEN_HORIZON),
    ("razor-thaw", RAZOR_THAW),
    ("sawyer-rezz", SAWYER_REZZ),
];

#[derive(Clone, Copy)]
struct GfmDryRunStats {
    finite: bool,
    peak_abs: f32,
    max_rupture_count: usize,
    health: GfmHealthHistogram,
    probe_position: (usize, usize),
    frames: usize,
    final_frame_index: usize,
}

#[derive(Debug, Clone, Copy)]
struct EngineLayerRenderStats {
    finite: bool,
    rms: f32,
    peak_abs: f32,
    gfm_selection: GfmVoiceProgramSelection,
    gfm_program_id: Option<GfmProgramId>,
    gfm_diagnostics: Option<GfmDiagnostics>,
}

fn fixture_engine() -> Engine {
    let patch = load_patch_toml(MOLTEN_HORIZON).expect("fixture must parse");
    Engine::new(EngineConfig::default(), patch).expect("fixture must validate")
}

#[derive(Debug, Clone, Copy)]
struct OutputStats {
    left_mean: f64,
    right_mean: f64,
    peak_abs: f32,
}

fn render_post_warmup_stats(patch_source: &str, note: Option<u8>) -> OutputStats {
    const SAMPLE_RATE_HZ: usize = 48_000;
    const BLOCK_FRAMES: usize = 240;
    const TOTAL_FRAMES: usize = SAMPLE_RATE_HZ * 3;
    const WARMUP_FRAMES: usize = SAMPLE_RATE_HZ;

    let patch = load_patch_toml(patch_source).expect("factory patch must parse");
    let mut engine = Engine::new(
        EngineConfig {
            sample_rate_hz: SAMPLE_RATE_HZ as f32,
            max_block_frames: BLOCK_FRAMES,
            voice_count: 6,
        },
        patch,
    )
    .expect("engine must validate");
    let note_on = [Scheduled {
        frame_offset: 0,
        event: NoteEvent::NoteOn {
            note: note.unwrap_or(60),
            velocity: 0.9,
        },
    }];
    let mut left = [0.0_f32; BLOCK_FRAMES];
    let mut right = [0.0_f32; BLOCK_FRAMES];
    let mut frame_cursor = 0_usize;
    let mut left_sum = 0.0_f64;
    let mut right_sum = 0.0_f64;
    let mut peak_abs = 0.0_f32;
    let mut measured_frames = 0_usize;

    while frame_cursor < TOTAL_FRAMES {
        let frame_count = (TOTAL_FRAMES - frame_cursor).min(BLOCK_FRAMES);
        let note_events = if frame_cursor == 0 && note.is_some() {
            &note_on[..]
        } else {
            &[]
        };
        engine.process_block(ProcessBlock {
            frame_count,
            note_events,
            controller_events: &[],
            macro_state: None,
            output: Some(StereoBlockMut::new(&mut left, &mut right)),
        });

        for frame in 0..frame_count {
            if frame_cursor + frame >= WARMUP_FRAMES {
                let left_sample = left[frame];
                let right_sample = right[frame];
                assert!(left_sample.is_finite());
                assert!(right_sample.is_finite());
                left_sum += left_sample as f64;
                right_sum += right_sample as f64;
                peak_abs = peak_abs.max(left_sample.abs().max(right_sample.abs()));
                measured_frames += 1;
            }
        }

        frame_cursor += frame_count;
    }

    OutputStats {
        left_mean: left_sum / measured_frames as f64,
        right_mean: right_sum / measured_frames as f64,
        peak_abs,
    }
}

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
    assert_eq!(snapshot.gfm_layer.amount, 0.0);
    assert_eq!(snapshot.gfm_layer.pressure, 0.65);
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
    assert_eq!(snapshot.gfm_layer.pressure, 0.0);
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
    assert_eq!(before.gfm_layer.amount, 0.72);
    assert_eq!(before.gfm_layer.pressure, 1.0);
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
    assert_eq!(snapshot.gfm_layer.amount, 0.85);
    assert_eq!(snapshot.gfm_layer.pressure, 0.0);
    assert_eq!(snapshot.gfm_layer.effective_amount, 0.0);
    assert_eq!(
        snapshot.gfm_layer.active_program_id,
        Some(GfmProgramId::HorizontPerformance)
    );
    assert!(snapshot.gfm_layer.diagnostics.is_some());
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
    let frames = ENGINE_LAYER_RECOVERY_RATE_HZ as usize * GFM_PERFORMANCE_DURATION_SECONDS;
    let (_, stats) = render_engine_layer_signature_at_rate(
        patch,
        Some(GFM_PERFORMANCE_BASELINE_SEED),
        ENGINE_LAYER_RECOVERY_RATE_HZ,
        frames,
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

#[test]
fn allocator_prefers_idle_then_released_then_oldest_active() {
    let mut engine = fixture_engine();

    for note in 60..66 {
        engine.process_block(ProcessBlock {
            frame_count: 64,
            note_events: &[Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note,
                    velocity: 0.8,
                },
            }],
            controller_events: &[],
            macro_state: None,
            output: None,
        });
    }

    let full_snapshot = engine.snapshot();
    assert_eq!(full_snapshot.active_voice_count, 6);
    assert_eq!(full_snapshot.voices[0].note, Some(60));

    engine.process_block(ProcessBlock {
        frame_count: 64,
        note_events: &[
            Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOff { note: 60 },
            },
            Scheduled {
                frame_offset: 1,
                event: NoteEvent::NoteOn {
                    note: 72,
                    velocity: 0.8,
                },
            },
        ],
        controller_events: &[],
        macro_state: None,
        output: None,
    });

    let released_reused = engine.snapshot();
    assert!(released_reused.held_notes.contains(&72));

    engine.process_block(ProcessBlock {
        frame_count: 64,
        note_events: &[Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: 73,
                velocity: 0.8,
            },
        }],
        controller_events: &[],
        macro_state: None,
        output: None,
    });

    let stolen = engine.snapshot();
    assert!(stolen.held_notes.contains(&73));
    assert!(!stolen.held_notes.contains(&61));
}

#[test]
fn note_off_prefers_held_voice_for_repeated_pitch() {
    let mut engine = fixture_engine();

    engine.process_block(ProcessBlock {
        frame_count: 64,
        note_events: &[
            Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: 60,
                    velocity: 0.8,
                },
            },
            Scheduled {
                frame_offset: 1,
                event: NoteEvent::NoteOff { note: 60 },
            },
            Scheduled {
                frame_offset: 2,
                event: NoteEvent::NoteOn {
                    note: 60,
                    velocity: 0.7,
                },
            },
            Scheduled {
                frame_offset: 3,
                event: NoteEvent::NoteOff { note: 60 },
            },
        ],
        controller_events: &[],
        macro_state: None,
        output: None,
    });

    let snapshot = engine.snapshot();
    assert!(
        snapshot
            .voices
            .iter()
            .filter(|voice| voice.note == Some(60))
            .all(|voice| voice.phase != VoicePhase::Held)
    );
}

#[test]
fn sustain_state_transitions_are_stable() {
    let mut engine = fixture_engine();

    engine.process_block(ProcessBlock {
        frame_count: 64,
        note_events: &[Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: 60,
                velocity: 1.0,
            },
        }],
        controller_events: &[Scheduled {
            frame_offset: 0,
            event: ControllerEvent::Sustain { down: true },
        }],
        macro_state: None,
        output: None,
    });

    engine.process_block(ProcessBlock {
        frame_count: 64,
        note_events: &[Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOff { note: 60 },
        }],
        controller_events: &[],
        macro_state: None,
        output: None,
    });

    let sustained = engine.snapshot();
    assert_eq!(sustained.voices[0].phase, VoicePhase::SustainedReleased);

    engine.process_block(ProcessBlock {
        frame_count: 64,
        note_events: &[],
        controller_events: &[Scheduled {
            frame_offset: 0,
            event: ControllerEvent::Sustain { down: false },
        }],
        macro_state: None,
        output: None,
    });
    assert_eq!(engine.snapshot().voices[0].phase, VoicePhase::Released);
}

#[test]
fn rendered_audio_is_non_silent_and_finite() {
    let mut engine = fixture_engine();
    let mut left = [0.0_f32; 256];
    let mut right = [0.0_f32; 256];
    engine.process_block(ProcessBlock {
        frame_count: 256,
        note_events: &[Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: 60,
                velocity: 0.9,
            },
        }],
        controller_events: &[],
        macro_state: None,
        output: Some(StereoBlockMut::new(&mut left, &mut right)),
    });

    let peak = left
        .iter()
        .zip(right.iter())
        .map(|(left, right)| left.abs().max(right.abs()))
        .fold(0.0, f32::max);
    assert!(peak > 0.0001);
    assert!(left.iter().all(|sample| sample.is_finite()));
    assert!(right.iter().all(|sample| sample.is_finite()));
}

#[test]
fn idle_master_output_rejects_patch_dc_bias() {
    let stats = render_post_warmup_stats(RAZOR_THAW, None);

    assert!(stats.left_mean.abs() < 0.002, "stats={stats:?}");
    assert!(stats.right_mean.abs() < 0.002, "stats={stats:?}");
    assert!(stats.peak_abs < 0.01, "stats={stats:?}");
}

#[test]
fn sustained_gravity_wake_output_has_low_dc_mean() {
    let stats = render_post_warmup_stats(GRAVITY_WAKE, Some(60));

    assert!(stats.left_mean.abs() < 0.02, "stats={stats:?}");
    assert!(stats.right_mean.abs() < 0.02, "stats={stats:?}");
    assert!(stats.peak_abs > 0.05, "stats={stats:?}");
}

#[test]
fn process_block_clamps_to_output_capacity() {
    let mut engine = fixture_engine();
    let mut left = [0.0_f32; 64];
    let mut right = [0.0_f32; 32];

    engine.process_block(ProcessBlock {
        frame_count: 128,
        note_events: &[Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: 60,
                velocity: 0.9,
            },
        }],
        controller_events: &[],
        macro_state: None,
        output: Some(StereoBlockMut::new(&mut left, &mut right)),
    });

    assert_eq!(engine.snapshot().last_block_frames, 128);
    assert!(left[..32].iter().all(|sample| sample.is_finite()));
    assert!(right.iter().all(|sample| sample.is_finite()));
}

#[test]
fn macro_sweeps_remain_finite() {
    let mut engine = fixture_engine();
    let mut left = [0.0_f32; 512];
    let mut right = [0.0_f32; 512];

    engine.process_block(ProcessBlock {
        frame_count: 512,
        note_events: &[Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: 48,
                velocity: 0.92,
            },
        }],
        controller_events: &[
            Scheduled {
                frame_offset: 64,
                event: ControllerEvent::Macro {
                    id: MacroId::Gravitacija,
                    value: 0.84,
                },
            },
            Scheduled {
                frame_offset: 160,
                event: ControllerEvent::Macro {
                    id: MacroId::Ruin,
                    value: 0.76,
                },
            },
            Scheduled {
                frame_offset: 320,
                event: ControllerEvent::Macro {
                    id: MacroId::Bloom,
                    value: 0.72,
                },
            },
        ],
        macro_state: None,
        output: Some(StereoBlockMut::new(&mut left, &mut right)),
    });

    assert!(left.iter().all(|sample| sample.is_finite()));
    assert!(right.iter().all(|sample| sample.is_finite()));
    let peak = left
        .iter()
        .zip(right.iter())
        .map(|(left, right)| left.abs().max(right.abs()))
        .fold(0.0, f32::max);
    assert!(peak > 0.0001);
    assert!(peak <= MASTER_SAFETY_CEILING, "peak={peak}");
}

#[test]
fn output_safety_snapshot_tracks_neutral_low_level_block() {
    let mut engine = fixture_engine();
    let mut left = [0.0_f32; 128];
    let mut right = [0.0_f32; 128];

    engine.process_block(ProcessBlock {
        frame_count: 128,
        note_events: &[],
        controller_events: &[],
        macro_state: None,
        output: Some(StereoBlockMut::new(&mut left, &mut right)),
    });

    let snapshot = engine.snapshot();
    assert!(left.iter().all(|sample| sample.is_finite()));
    assert!(right.iter().all(|sample| sample.is_finite()));
    assert_eq!(snapshot.output_safety.safety_limiter_hits, 0);
    assert_eq!(snapshot.output_safety.max_safety_reduction, 0.0);
    assert_eq!(snapshot.output_safety.tiny_flush_events, 0);
    assert!(
        snapshot.output_safety.post_safety_peak <= MASTER_SAFETY_CEILING,
        "snapshot={snapshot:?}"
    );
    assert_eq!(
        snapshot.output_safety.pre_safety_peak.to_bits(),
        snapshot.output_safety.post_safety_peak.to_bits()
    );
    assert_eq!(
        snapshot.peak_output.to_bits(),
        snapshot.output_safety.post_safety_peak.to_bits()
    );
    assert!(!snapshot.clip_detected);
}

#[test]
fn output_safety_snapshot_counts_tiny_flush_and_limiter_work() {
    let mut tiny = OutputSafetySnapshot::default();
    tiny.observe_channel(DENORMAL_FLUSH_ABS * 0.5, 0.0);

    assert_eq!(tiny.tiny_flush_events, 1);
    assert_eq!(tiny.safety_limiter_hits, 0);
    assert_eq!(tiny.max_safety_reduction, 0.0);

    let hot_pre = 1.40_f32;
    let hot_post = master_safety_limit(hot_pre);
    let mut hot = OutputSafetySnapshot::default();
    hot.observe_channel(hot_pre, hot_post);

    assert_eq!(hot.safety_limiter_hits, 1);
    assert!(hot.max_safety_reduction > 0.0);
    assert!(hot_post <= MASTER_SAFETY_CEILING);
}

#[test]
fn hot_render_reports_output_safety_limiter_telemetry() {
    let mut engine = fixture_engine();
    let mut left = [0.0_f32; 2048];
    let mut right = [0.0_f32; 2048];
    let note_events = [
        Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: 36,
                velocity: 1.0,
            },
        },
        Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: 43,
                velocity: 1.0,
            },
        },
        Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: 48,
                velocity: 1.0,
            },
        },
        Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: 55,
                velocity: 1.0,
            },
        },
        Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: 60,
                velocity: 1.0,
            },
        },
        Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: 67,
                velocity: 1.0,
            },
        },
    ];

    engine.process_block(ProcessBlock {
        frame_count: 2048,
        note_events: &note_events,
        controller_events: &[Scheduled {
            frame_offset: 0,
            event: ControllerEvent::DirectParam {
                id: ParamId::FinalStageOutputTrimDb,
                value: 12.0,
            },
        }],
        macro_state: Some(MacroState {
            gravitacija: 1.0,
            bloom: 0.7,
            heat: 1.0,
            ruin: 1.0,
            swarm: 0.75,
        }),
        output: Some(StereoBlockMut::new(&mut left, &mut right)),
    });

    let snapshot = engine.snapshot();
    assert!(left.iter().all(|sample| sample.is_finite()));
    assert!(right.iter().all(|sample| sample.is_finite()));
    assert!(
        snapshot.output_safety.pre_safety_peak > MASTER_SAFETY_CEILING,
        "snapshot={snapshot:?}"
    );
    assert!(
        snapshot.output_safety.post_safety_peak <= MASTER_SAFETY_CEILING,
        "snapshot={snapshot:?}"
    );
    assert!(
        snapshot.output_safety.safety_limiter_hits > 0,
        "snapshot={snapshot:?}"
    );
    assert!(
        snapshot.output_safety.max_safety_reduction > 0.0,
        "snapshot={snapshot:?}"
    );
    assert_eq!(
        snapshot.peak_output.to_bits(),
        snapshot.output_safety.post_safety_peak.to_bits()
    );
}

#[test]
fn dry_path_remains_non_silent_with_fx_disabled() {
    let mut patch = load_patch_toml(MOLTEN_HORIZON).expect("fixture must parse");
    patch.engine.fx.chorus.enabled = false;
    patch.engine.fx.reverb.enabled = false;
    let mut engine = Engine::new(EngineConfig::default(), patch).expect("fixture must validate");
    let mut left = [0.0_f32; 512];
    let mut right = [0.0_f32; 512];

    engine.process_block(ProcessBlock {
        frame_count: 512,
        note_events: &[Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: 43,
                velocity: 0.88,
            },
        }],
        controller_events: &[],
        macro_state: None,
        output: Some(StereoBlockMut::new(&mut left, &mut right)),
    });

    let peak = left
        .iter()
        .zip(right.iter())
        .map(|(left, right)| left.abs().max(right.abs()))
        .fold(0.0, f32::max);
    assert!(peak > 0.0001);
    assert!(left.iter().all(|sample| sample.is_finite()));
    assert!(right.iter().all(|sample| sample.is_finite()));
}

#[test]
fn dense_chord_playback_with_fx_stays_finite() {
    let mut engine = fixture_engine();
    let mut left = [0.0_f32; 1024];
    let mut right = [0.0_f32; 1024];
    let note_events = [
        Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: 48,
                velocity: 0.82,
            },
        },
        Scheduled {
            frame_offset: 32,
            event: NoteEvent::NoteOn {
                note: 55,
                velocity: 0.84,
            },
        },
        Scheduled {
            frame_offset: 64,
            event: NoteEvent::NoteOn {
                note: 60,
                velocity: 0.88,
            },
        },
        Scheduled {
            frame_offset: 96,
            event: NoteEvent::NoteOn {
                note: 67,
                velocity: 0.90,
            },
        },
    ];
    let controller_events = [
        Scheduled {
            frame_offset: 128,
            event: ControllerEvent::Macro {
                id: MacroId::Gravitacija,
                value: 0.82,
            },
        },
        Scheduled {
            frame_offset: 256,
            event: ControllerEvent::Macro {
                id: MacroId::Heat,
                value: 0.74,
            },
        },
        Scheduled {
            frame_offset: 384,
            event: ControllerEvent::Macro {
                id: MacroId::Ruin,
                value: 0.70,
            },
        },
    ];

    engine.process_block(ProcessBlock {
        frame_count: 1024,
        note_events: &note_events,
        controller_events: &controller_events,
        macro_state: None,
        output: Some(StereoBlockMut::new(&mut left, &mut right)),
    });

    assert!(left.iter().all(|sample| sample.is_finite()));
    assert!(right.iter().all(|sample| sample.is_finite()));
    assert!(
        left.iter()
            .zip(right.iter())
            .map(|(left, right)| left.abs().max(right.abs()))
            .fold(0.0, f32::max)
            > 0.0001
    );
}

#[test]
fn snapshot_exposes_patch_metadata() {
    let engine = fixture_engine();
    let snapshot = engine.snapshot();
    assert_eq!(snapshot.patch_name, "Molten Horizon");
    assert!(snapshot.patch_description.is_some());
    assert!(snapshot.patch_tags.iter().any(|tag| tag == "monster"));
    assert!(snapshot.patch_favorite);
}

#[test]
fn performance_response_direct_params_update_snapshot_and_exported_patch() {
    let mut engine = fixture_engine();

    engine.process_block(ProcessBlock {
        frame_count: 1,
        note_events: &[],
        controller_events: &[Scheduled {
            frame_offset: 0,
            event: ControllerEvent::DirectParam {
                id: ParamId::PerformanceAftertouchToBaklja,
                value: 0.77,
            },
        }],
        macro_state: None,
        output: None,
    });

    let snapshot = engine.snapshot();
    assert!((snapshot.performance_response.aftertouch_to_baklja - 0.77).abs() < 0.0001);
    assert!(
        (engine
            .export_patch()
            .performance_response
            .aftertouch_to_baklja
            - 0.77)
            .abs()
            < 0.0001
    );
}

#[test]
fn source_expansion_direct_params_update_snapshot_and_exported_patch() {
    let mut engine = fixture_engine();

    engine.process_block(ProcessBlock {
        frame_count: 1,
        note_events: &[],
        controller_events: &[
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::DirectParam {
                    id: ParamId::Osc1PulseWidth,
                    value: 0.37,
                },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::DirectParam {
                    id: ParamId::Osc2Level,
                    value: 1.42,
                },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::DirectParam {
                    id: ParamId::Osc2PitchMode,
                    value: 1.0,
                },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::DirectParam {
                    id: ParamId::NoiseColor,
                    value: 2.0,
                },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::DirectParam {
                    id: ParamId::CrossMixMode,
                    value: 4.0,
                },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::DirectParam {
                    id: ParamId::SpectralLevel,
                    value: 0.38,
                },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::DirectParam {
                    id: ParamId::SpectralTable,
                    value: 2.0,
                },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::DirectParam {
                    id: ParamId::SpectralPosition,
                    value: 0.72,
                },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::DirectParam {
                    id: ParamId::SpectralMorph,
                    value: 0.44,
                },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::DirectParam {
                    id: ParamId::SpectralRatio,
                    value: 1.75,
                },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::DirectParam {
                    id: ParamId::SpectralFineTuneCents,
                    value: -11.0,
                },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::DirectParam {
                    id: ParamId::AdditiveLevel,
                    value: 0.31,
                },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::DirectParam {
                    id: ParamId::AdditivePartialCount,
                    value: 7.0,
                },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::DirectParam {
                    id: ParamId::AdditiveHarmonicSpread,
                    value: 0.42,
                },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::DirectParam {
                    id: ParamId::AdditiveOddEvenBalance,
                    value: -0.35,
                },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::DirectParam {
                    id: ParamId::AdditiveInharmonicity,
                    value: 0.27,
                },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::DirectParam {
                    id: ParamId::AdditiveSpectralTilt,
                    value: 0.58,
                },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::DirectParam {
                    id: ParamId::AdditiveRandomDetuneCents,
                    value: 14.0,
                },
            },
        ],
        macro_state: None,
        output: None,
    });

    let snapshot = engine.snapshot();
    assert!((snapshot.direct.osc1_pulse_width - 0.37).abs() < 0.0001);
    assert!((snapshot.direct.osc2_level - 1.42).abs() < 0.0001);
    assert_eq!(snapshot.direct.osc2_pitch_mode, 1.0);
    assert_eq!(snapshot.direct.noise_color, 2.0);
    assert_eq!(snapshot.direct.cross_mix_mode, 4.0);
    assert!((snapshot.direct.spectral_level - 0.38).abs() < 0.0001);
    assert_eq!(snapshot.direct.spectral_table, 2.0);
    assert!((snapshot.direct.spectral_position - 0.72).abs() < 0.0001);
    assert!((snapshot.direct.spectral_morph - 0.44).abs() < 0.0001);
    assert!((snapshot.direct.spectral_ratio - 1.75).abs() < 0.0001);
    assert!((snapshot.direct.spectral_fine_tune_cents + 11.0).abs() < 0.0001);
    assert!((snapshot.direct.additive_level - 0.31).abs() < 0.0001);
    assert_eq!(snapshot.direct.additive_partial_count, 7.0);
    assert!((snapshot.direct.additive_harmonic_spread - 0.42).abs() < 0.0001);
    assert!((snapshot.direct.additive_odd_even_balance + 0.35).abs() < 0.0001);
    assert!((snapshot.direct.additive_inharmonicity - 0.27).abs() < 0.0001);
    assert!((snapshot.direct.additive_spectral_tilt - 0.58).abs() < 0.0001);
    assert!((snapshot.direct.additive_random_detune_cents - 14.0).abs() < 0.0001);

    let exported = engine.export_patch();
    assert_eq!(exported.engine.osc1.pulse_width, 0.37);
    assert_eq!(exported.engine.osc2.level, 1.42);
    assert_eq!(exported.engine.osc2.pitch_mode, Osc2PitchMode::Ratio);
    assert_eq!(exported.engine.noise_color, NoiseColor::Dark);
    assert_eq!(exported.engine.cross_mix_mode, CrossMixMode::Difference);
    assert_eq!(exported.engine.spectral.level, 0.38);
    assert_eq!(exported.engine.spectral.table, SpectralTable::Metallic);
    assert_eq!(exported.engine.spectral.position, 0.72);
    assert_eq!(exported.engine.spectral.morph, 0.44);
    assert_eq!(exported.engine.spectral.ratio, 1.75);
    assert_eq!(exported.engine.spectral.fine_tune_cents, -11.0);
    assert_eq!(exported.engine.additive.level, 0.31);
    assert_eq!(exported.engine.additive.partial_count, 7);
    assert_eq!(exported.engine.additive.harmonic_spread, 0.42);
    assert_eq!(exported.engine.additive.odd_even_balance, -0.35);
    assert_eq!(exported.engine.additive.inharmonicity, 0.27);
    assert_eq!(exported.engine.additive.spectral_tilt, 0.58);
    assert_eq!(exported.engine.additive.random_detune_cents, 14.0);
}

#[test]
fn source_expansion_non_default_render_is_finite_and_changes_signature() {
    let mut patch = load_patch_toml(MOLTEN_HORIZON).expect("fixture must parse");
    let (baseline_signature, _) = render_engine_layer_signature(patch.clone(), None, 2048);

    patch.engine.osc1.pwm_depth = 0.42;
    patch.engine.osc1.saw_bend = 0.55;
    patch.engine.osc1.triangle_fold = 0.35;
    patch.engine.osc1.pulse_edge = 0.25;
    patch.engine.osc2.level = 1.45;
    patch.engine.osc2.pitch_mode = Osc2PitchMode::Ratio;
    patch.engine.osc2.ratio = 1.50;
    patch.engine.osc2.phase_mode = OscPhaseMode::Fixed;
    patch.engine.osc2.start_phase = 0.33;
    patch.engine.noise_filter_level = Some(0.20);
    patch.engine.noise_body_level = 0.16;
    patch.engine.noise_color = NoiseColor::Bright;
    patch.engine.fm_amount = 0.22;
    patch.engine.phase_mod_amount = 0.18;
    patch.engine.ring_mod_amount = 0.20;
    patch.engine.cross_mix_mode = CrossMixMode::Fold;
    patch.engine.cross_mix_amount = 0.35;
    patch.engine.spectral.level = 0.44;
    patch.engine.spectral.table = SpectralTable::Formant;
    patch.engine.spectral.position = 0.82;
    patch.engine.spectral.morph = 0.58;
    patch.engine.spectral.ratio = 1.25;
    patch.engine.spectral.fine_tune_cents = 7.0;
    patch.engine.additive.level = 0.30;
    patch.engine.additive.partial_count = 7;
    patch.engine.additive.harmonic_spread = 0.38;
    patch.engine.additive.odd_even_balance = 0.44;
    patch.engine.additive.inharmonicity = 0.25;
    patch.engine.additive.spectral_tilt = 0.52;
    patch.engine.additive.random_detune_cents = 9.5;

    let (source_signature, source_stats) = render_engine_layer_signature(patch, None, 2048);

    assert!(source_stats.finite);
    assert!(source_stats.peak_abs < 1.01);
    assert_ne!(baseline_signature, source_signature);
}

#[test]
fn additive_source_is_deterministic_and_changes_render_signature() {
    let mut patch = load_patch_toml(MOLTEN_HORIZON).expect("fixture must parse");
    let (baseline_signature, baseline_stats) =
        render_engine_layer_signature(patch.clone(), None, 2048);
    assert!(baseline_stats.finite);

    patch.engine.additive.level = 0.36;
    patch.engine.additive.partial_count = 6;
    patch.engine.additive.harmonic_spread = 0.32;
    patch.engine.additive.odd_even_balance = 0.50;
    patch.engine.additive.inharmonicity = 0.22;
    patch.engine.additive.spectral_tilt = -0.42;
    patch.engine.additive.random_detune_cents = 11.0;

    let (first_signature, first_stats) = render_engine_layer_signature(patch.clone(), None, 2048);
    let (second_signature, second_stats) = render_engine_layer_signature(patch, None, 2048);

    assert!(first_stats.finite);
    assert!(second_stats.finite);
    assert!(first_stats.peak_abs < 1.01);
    assert_eq!(first_signature, second_signature);
    assert_ne!(baseline_signature, first_signature);
}

#[test]
fn additive_extreme_valid_controls_stay_finite() {
    let mut patch = load_patch_toml(MOLTEN_HORIZON).expect("fixture must parse");
    patch.engine.additive.level = 1.0;
    patch.engine.additive.partial_count = 8;
    patch.engine.additive.harmonic_spread = 1.0;
    patch.engine.additive.odd_even_balance = -1.0;
    patch.engine.additive.inharmonicity = 1.0;
    patch.engine.additive.spectral_tilt = 1.0;
    patch.engine.additive.random_detune_cents = 35.0;

    let (_, stats) = render_engine_layer_signature(patch, None, 4096);

    assert!(stats.finite);
    assert!(stats.peak_abs <= 1.0);
}

#[test]
fn load_patch_resets_runtime_state() {
    let mut engine = fixture_engine();
    let mut left = [0.0_f32; 128];
    let mut right = [0.0_f32; 128];

    engine.process_block(ProcessBlock {
        frame_count: 128,
        note_events: &[Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: 60,
                velocity: 0.9,
            },
        }],
        controller_events: &[Scheduled {
            frame_offset: 0,
            event: ControllerEvent::Macro {
                id: MacroId::Gravitacija,
                value: 0.92,
            },
        }],
        macro_state: None,
        output: Some(StereoBlockMut::new(&mut left, &mut right)),
    });
    assert!(engine.snapshot().active_voice_count > 0);

    let replacement = load_patch_toml(include_str!("../../../patches/factory/ember-vault.toml"))
        .expect("replacement patch parses");
    engine
        .load_patch(replacement)
        .expect("replacement patch loads");

    let snapshot = engine.snapshot();
    assert_eq!(snapshot.patch_name, "Ember Vault");
    assert_eq!(snapshot.active_voice_count, 0);
    assert!(!snapshot.sustain_down);
    assert!(snapshot.peak_output.abs() <= f32::EPSILON);
}

#[test]
fn panic_clears_notes_and_controller_state() {
    let mut engine = fixture_engine();

    engine.process_block(ProcessBlock {
        frame_count: 64,
        note_events: &[Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: 60,
                velocity: 0.95,
            },
        }],
        controller_events: &[
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::Sustain { down: true },
            },
            Scheduled {
                frame_offset: 1,
                event: ControllerEvent::ModWheel { amount: 0.8 },
            },
            Scheduled {
                frame_offset: 2,
                event: ControllerEvent::ChannelAftertouch { pressure: 0.7 },
            },
            Scheduled {
                frame_offset: 3,
                event: ControllerEvent::GfmLayerAmount { amount: 0.25 },
            },
        ],
        macro_state: None,
        output: None,
    });

    engine.panic();

    let snapshot = engine.snapshot();
    assert_eq!(snapshot.active_voice_count, 0);
    assert!(snapshot.held_notes.is_empty());
    assert!(!snapshot.sustain_down);
    assert_eq!(snapshot.gfm_layer.amount, 0.0);
    assert_eq!(snapshot.gfm_layer.pressure, 0.0);
    assert_eq!(snapshot.gfm_layer.effective_amount, 0.0);
    assert!(snapshot.live_macros == MacroState::from_defaults(&engine.patch.macros));
    assert!(!snapshot.clip_detected);
}

#[test]
fn panic_clears_patch_switch_mute_window() {
    let mut engine = fixture_engine();
    let replacement = load_patch_toml(include_str!("../../../patches/factory/ember-vault.toml"))
        .expect("replacement patch parses");
    engine
        .load_patch(replacement)
        .expect("replacement patch loads");

    let (left_before, right_before) = engine.render_frame();
    assert_eq!((left_before, right_before), (0.0, 0.0));

    engine.panic();

    let (left_after, right_after) = engine.render_frame();
    assert!(left_after.is_finite());
    assert!(right_after.is_finite());
}

#[test]
fn reset_controllers_releases_sustain_state_but_keeps_held_voice() {
    let mut engine = fixture_engine();

    engine.process_block(ProcessBlock {
        frame_count: 64,
        note_events: &[Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: 60,
                velocity: 0.9,
            },
        }],
        controller_events: &[Scheduled {
            frame_offset: 0,
            event: ControllerEvent::Sustain { down: true },
        }],
        macro_state: None,
        output: None,
    });
    engine.process_block(ProcessBlock {
        frame_count: 64,
        note_events: &[Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOff { note: 60 },
        }],
        controller_events: &[Scheduled {
            frame_offset: 0,
            event: ControllerEvent::Macro {
                id: MacroId::Ruin,
                value: 0.85,
            },
        }],
        macro_state: None,
        output: None,
    });

    assert!(engine.snapshot().sustain_down);
    engine.reset_controllers();

    let snapshot = engine.snapshot();
    assert!(!snapshot.sustain_down);
    assert!(snapshot.held_notes.contains(&60));
    assert!(snapshot.live_macros == MacroState::from_defaults(&engine.patch.macros));
}

fn render_engine_layer_signature(
    patch: PatchFileV1,
    layer_seed: Option<u64>,
    frames: usize,
) -> (u64, EngineLayerRenderStats) {
    render_engine_layer_signature_at_rate(patch, layer_seed, ENGINE_LAYER_TEST_RATE_HZ, frames)
}

fn render_engine_layer_signature_at_rate(
    patch: PatchFileV1,
    layer_seed: Option<u64>,
    sample_rate_hz: u32,
    frames: usize,
) -> (u64, EngineLayerRenderStats) {
    let note_events = engine_layer_note_on_events(select_gfm_program_for_patch(&patch).program_id);
    let mut engine = Engine::new(
        EngineConfig {
            sample_rate_hz: sample_rate_hz as f32,
            max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
            voice_count: 6,
        },
        patch,
    )
    .expect("engine must validate");

    if let Some(seed) = layer_seed {
        engine.set_gfm_layer_mode(GfmLayerMode::Enabled { seed });
    }

    render_engine_layer_signature_with_notes(&mut engine, frames, note_events)
}

fn render_engine_layer_signature_for_engine(
    engine: &mut Engine,
    frames: usize,
) -> (u64, EngineLayerRenderStats) {
    let note_events =
        engine_layer_note_on_events(select_gfm_program_for_patch(&engine.patch).program_id);
    render_engine_layer_signature_with_notes(engine, frames, note_events)
}

fn render_engine_layer_signature_with_notes(
    engine: &mut Engine,
    frames: usize,
    note_events: [ScheduledNoteEvent; 3],
) -> (u64, EngineLayerRenderStats) {
    render_engine_layer_signature_with_notes_and_controllers(engine, frames, note_events, &[])
}

fn bcs_layer_playable_controller_events(amount: f32) -> [ScheduledControllerEvent; 2] {
    [
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::BcsLayerEnabled { enabled: true },
        },
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::BcsLayerAmount { amount },
        },
    ]
}

fn render_engine_layer_mono_samples(
    engine: &mut Engine,
    frames: usize,
    note_events: &[ScheduledNoteEvent],
    initial_controller_events: &[ScheduledControllerEvent],
) -> (Vec<f32>, EngineLayerRenderStats) {
    let mut rendered = 0;
    let mut finite = true;
    let mut peak_abs = 0.0_f32;
    let mut sum_squares = 0.0_f32;
    let mut mono = Vec::with_capacity(frames);
    let mut left = [0.0_f32; ENGINE_LAYER_BLOCK_FRAMES];
    let mut right = [0.0_f32; ENGINE_LAYER_BLOCK_FRAMES];

    while rendered < frames {
        let frame_count = (frames - rendered).min(ENGINE_LAYER_BLOCK_FRAMES);

        let (block_note_events, block_controller_events) = if rendered == 0 {
            (note_events, initial_controller_events)
        } else {
            (&[][..], &[][..])
        };
        engine.process_block(ProcessBlock {
            frame_count,
            note_events: block_note_events,
            controller_events: block_controller_events,
            macro_state: None,
            output: Some(StereoBlockMut::new(
                &mut left[..frame_count],
                &mut right[..frame_count],
            )),
        });

        for index in 0..frame_count {
            let left_sample = left[index];
            let right_sample = right[index];
            finite &= left_sample.is_finite() && right_sample.is_finite();
            peak_abs = peak_abs.max(left_sample.abs().max(right_sample.abs()));
            sum_squares += left_sample * left_sample + right_sample * right_sample;
            mono.push((left_sample + right_sample) * 0.5);
        }

        rendered += frame_count;
    }

    let rms = (sum_squares / (frames.max(1) * 2) as f32).sqrt();
    let gfm_layer = engine.snapshot().gfm_layer;
    (
        mono,
        EngineLayerRenderStats {
            finite,
            rms,
            peak_abs,
            gfm_selection: gfm_layer.selection,
            gfm_program_id: gfm_layer.active_program_id,
            gfm_diagnostics: gfm_layer.diagnostics,
        },
    )
}

fn rms_window(samples: &[f32]) -> f32 {
    let sum_squares = samples.iter().map(|sample| sample * sample).sum::<f32>();
    (sum_squares / samples.len().max(1) as f32).sqrt()
}

fn rms_delta_window(left: &[f32], right: &[f32]) -> f32 {
    let sum_squares = left
        .iter()
        .zip(right)
        .map(|(left, right)| {
            let delta = left - right;
            delta * delta
        })
        .sum::<f32>();
    (sum_squares / left.len().min(right.len()).max(1) as f32).sqrt()
}

fn amplitude_ratio_db(numerator: f32, denominator: f32) -> f32 {
    20.0 * (numerator.max(f32::MIN_POSITIVE) / denominator.max(f32::MIN_POSITIVE)).log10()
}

fn render_engine_layer_signature_with_notes_and_controllers(
    engine: &mut Engine,
    frames: usize,
    note_events: [ScheduledNoteEvent; 3],
    initial_controller_events: &[ScheduledControllerEvent],
) -> (u64, EngineLayerRenderStats) {
    let mut rendered = 0;
    let mut signature = 0xcbf2_9ce4_8422_2325_u64;
    let mut finite = true;
    let mut peak_abs = 0.0_f32;
    let mut sum_squares = 0.0_f32;
    let mut left = [0.0_f32; ENGINE_LAYER_BLOCK_FRAMES];
    let mut right = [0.0_f32; ENGINE_LAYER_BLOCK_FRAMES];

    while rendered < frames {
        let frame_count = (frames - rendered).min(ENGINE_LAYER_BLOCK_FRAMES);

        if rendered == 0 {
            engine.process_block(ProcessBlock {
                frame_count,
                note_events: &note_events,
                controller_events: initial_controller_events,
                macro_state: None,
                output: Some(StereoBlockMut::new(
                    &mut left[..frame_count],
                    &mut right[..frame_count],
                )),
            });
        } else {
            engine.process_block(ProcessBlock {
                frame_count,
                note_events: &[],
                controller_events: &[],
                macro_state: None,
                output: Some(StereoBlockMut::new(
                    &mut left[..frame_count],
                    &mut right[..frame_count],
                )),
            });
        }

        for index in 0..frame_count {
            let left_sample = left[index];
            let right_sample = right[index];
            finite &= left_sample.is_finite() && right_sample.is_finite();
            peak_abs = peak_abs.max(left_sample.abs().max(right_sample.abs()));
            sum_squares += left_sample * left_sample + right_sample * right_sample;
            for pcm in [sample_to_pcm16(left_sample), sample_to_pcm16(right_sample)] {
                signature ^= pcm as u16 as u64;
                signature = signature.wrapping_mul(0x100_0000_01b3);
            }
        }

        rendered += frame_count;
    }

    let rms = (sum_squares / (frames.max(1) * 2) as f32).sqrt();
    let gfm_layer = engine.snapshot().gfm_layer;
    (
        signature,
        EngineLayerRenderStats {
            finite,
            rms,
            peak_abs,
            gfm_selection: gfm_layer.selection,
            gfm_program_id: gfm_layer.active_program_id,
            gfm_diagnostics: gfm_layer.diagnostics,
        },
    )
}

fn engine_layer_note_on_events(program_id: Option<GfmProgramId>) -> [ScheduledNoteEvent; 3] {
    let [low, middle, high] = engine_layer_notes(program_id);
    [
        Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: low,
                velocity: 0.78,
            },
        },
        Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: middle,
                velocity: 0.70,
            },
        },
        Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: high,
                velocity: 0.64,
            },
        },
    ]
}

fn engine_layer_notes(program_id: Option<GfmProgramId>) -> [u8; 3] {
    match program_id {
        Some(GfmProgramId::PecPerformance) => [60, 67, 72],
        Some(GfmProgramId::HorizontPerformance | GfmProgramId::BakljaPerformance) | None => {
            [48, 55, 60]
        }
    }
}

fn render_gfm_adapter_signature(program_id: GfmProgramId) -> (u64, GfmDryRunStats) {
    let mut voice = GfmFieldVoice::new(program_id, GFM_PERFORMANCE_BASELINE_SEED, GFM_TEST_RATE_HZ);
    let frames = voice.frames();
    let probe_position = voice.probe_position();
    let mut signature = 0xcbf2_9ce4_8422_2325_u64;
    let mut finite = true;
    let mut peak_abs = 0.0_f32;

    for _ in 0..frames {
        let sample = voice.next_sample();
        finite &= sample.is_finite();
        peak_abs = peak_abs.max(sample.abs());
        let pcm = sample_to_pcm16(sample);
        signature ^= pcm as u16 as u64;
        signature = signature.wrapping_mul(0x100_0000_01b3);
    }

    let diagnostics = voice.diagnostics();
    (
        signature,
        GfmDryRunStats {
            finite,
            peak_abs,
            max_rupture_count: diagnostics.max_rupture_count,
            health: diagnostics.health,
            probe_position,
            frames,
            final_frame_index: voice.frame_index(),
        },
    )
}

fn render_gfm_adapter_signature_with_live_pressure(
    program_id: GfmProgramId,
    pressure: f32,
    frames: usize,
) -> (u64, GfmDryRunStats) {
    let mut voice = GfmFieldVoice::new(program_id, GFM_PERFORMANCE_BASELINE_SEED, GFM_TEST_RATE_HZ);
    let probe_position = voice.probe_position();
    let mut signature = 0xcbf2_9ce4_8422_2325_u64;
    let mut finite = true;
    let mut peak_abs = 0.0_f32;

    for _ in 0..frames {
        let sample = voice.next_sample_with_live_pressure(pressure);
        finite &= sample.is_finite();
        peak_abs = peak_abs.max(sample.abs());
        let pcm = sample_to_pcm16(sample);
        signature ^= pcm as u16 as u64;
        signature = signature.wrapping_mul(0x100_0000_01b3);
    }

    let diagnostics = voice.diagnostics();
    (
        signature,
        GfmDryRunStats {
            finite,
            peak_abs,
            max_rupture_count: diagnostics.max_rupture_count,
            health: diagnostics.health,
            probe_position,
            frames,
            final_frame_index: voice.frame_index(),
        },
    )
}

fn render_gfm_block_signature(program_id: GfmProgramId) -> (u64, GfmDryRunStats) {
    let voice = GfmFieldVoice::new(program_id, GFM_PERFORMANCE_BASELINE_SEED, GFM_TEST_RATE_HZ);
    render_gfm_voice_block_signature(voice)
}

fn render_gfm_voice_block_signature(mut voice: GfmFieldVoice) -> (u64, GfmDryRunStats) {
    let frames = voice.frames();
    let probe_position = voice.probe_position();
    let mut signature = 0xcbf2_9ce4_8422_2325_u64;
    let mut finite = true;
    let mut peak_abs = 0.0_f32;
    let mut rendered = 0;
    let mut block = [0.0_f32; 512];

    while rendered < frames {
        let frame_count = (frames - rendered).min(block.len());
        voice.render_mono_block(&mut block[..frame_count]);
        for sample in &block[..frame_count] {
            finite &= sample.is_finite();
            peak_abs = peak_abs.max(sample.abs());
            let pcm = sample_to_pcm16(*sample);
            signature ^= pcm as u16 as u64;
            signature = signature.wrapping_mul(0x100_0000_01b3);
        }
        rendered += frame_count;
    }

    let diagnostics = voice.diagnostics();
    (
        signature,
        GfmDryRunStats {
            finite,
            peak_abs,
            max_rupture_count: diagnostics.max_rupture_count,
            health: diagnostics.health,
            probe_position,
            frames,
            final_frame_index: voice.frame_index(),
        },
    )
}

fn render_gfm_chunked_signature(
    program_id: GfmProgramId,
    chunk_size: usize,
) -> (u64, GfmDryRunStats) {
    let mut voice = GfmFieldVoice::new(program_id, GFM_PERFORMANCE_BASELINE_SEED, GFM_TEST_RATE_HZ);
    let frames = voice.frames();
    let probe_position = voice.probe_position();
    let mut signature = 0xcbf2_9ce4_8422_2325_u64;
    let mut finite = true;
    let mut peak_abs = 0.0_f32;
    let mut rendered = 0;
    let mut block = [0.0_f32; 251];
    let chunk_size = chunk_size.clamp(1, block.len());

    while rendered < frames {
        let frame_count = (frames - rendered).min(chunk_size);
        voice.render_mono_block(&mut block[..frame_count]);
        for sample in &block[..frame_count] {
            finite &= sample.is_finite();
            peak_abs = peak_abs.max(sample.abs());
            let pcm = sample_to_pcm16(*sample);
            signature ^= pcm as u16 as u64;
            signature = signature.wrapping_mul(0x100_0000_01b3);
        }
        rendered += frame_count;
    }

    let diagnostics = voice.diagnostics();
    (
        signature,
        GfmDryRunStats {
            finite,
            peak_abs,
            max_rupture_count: diagnostics.max_rupture_count,
            health: diagnostics.health,
            probe_position,
            frames,
            final_frame_index: voice.frame_index(),
        },
    )
}

fn render_gfm_selected_patch_signature(program_id: GfmProgramId) -> (u64, GfmDryRunStats) {
    render_gfm_block_signature(program_id)
}

fn gfm_patch_voice_test_config() -> GfmPatchVoiceConfig {
    GfmPatchVoiceConfig::new(GFM_PERFORMANCE_BASELINE_SEED, GFM_TEST_RATE_HZ)
}

fn low_score_gfm_patch() -> PatchFileV1 {
    let mut patch = load_patch_toml(MOLTEN_HORIZON).expect("fixture must parse");
    patch.meta.patch_name = "GFM Low Score Disabled".to_string();
    patch.macros.gravitacija = 0.0;
    patch.macros.bloom = 0.0;
    patch.macros.heat = 0.0;
    patch.macros.ruin = 0.0;
    patch.macros.swarm = 0.0;
    patch.identity_bias.horizont_bias = -1.0;
    patch.identity_bias.pec_bias = -1.0;
    patch.identity_bias.baklja_bias = -1.0;
    patch.identity_bias.gravitacija_pressure_bias = -1.0;
    patch.identity_bias.rupture_threshold_bias = 1.0;
    patch.identity_bias.body_focus_bias = -1.0;
    patch
}

fn render_direct_field_signature(program_id: GfmProgramId) -> (u64, GfmDryRunStats) {
    let program = GfmPerformanceProgram::new(program_id, GFM_TEST_RATE_HZ as f32);
    let gesture = GfmPerformanceGesture::v0_4();
    let mut lattice = GfmLattice16::new(GFM_PERFORMANCE_BASELINE_SEED, program.params());
    let frames = gesture.frames(GFM_TEST_RATE_HZ);
    let probe_position = lattice.probe_position();
    let mut signature = 0xcbf2_9ce4_8422_2325_u64;
    let mut finite = true;
    let mut peak_abs = 0.0_f32;

    for frame in 0..frames {
        let sample = lattice
            .next_sample_with_excitation(gesture.excitation_at_frame(frame, GFM_TEST_RATE_HZ));
        finite &= sample.is_finite();
        peak_abs = peak_abs.max(sample.abs());
        let pcm = sample_to_pcm16(sample);
        signature ^= pcm as u16 as u64;
        signature = signature.wrapping_mul(0x100_0000_01b3);
    }

    let diagnostics = lattice.diagnostics();
    (
        signature,
        GfmDryRunStats {
            finite,
            peak_abs,
            max_rupture_count: diagnostics.max_rupture_count,
            health: diagnostics.health,
            probe_position,
            frames,
            final_frame_index: frames,
        },
    )
}
