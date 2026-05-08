use super::*;

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
