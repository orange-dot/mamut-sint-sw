use super::*;

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
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::DirectParam {
                    id: ParamId::Osc1Bandlimit,
                    value: 0.25,
                },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::DirectParam {
                    id: ParamId::Osc2Bandlimit,
                    value: 0.20,
                },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::DirectParam {
                    id: ParamId::FilterModel,
                    value: 2.0,
                },
            },
        ],
        macro_state: None,
        output: None,
    });

    let snapshot = engine.snapshot();
    assert!((snapshot.direct.osc1_pulse_width - 0.37).abs() < 0.0001);
    assert!((snapshot.direct.osc1_bandlimit - 0.25).abs() < 0.0001);
    assert!((snapshot.direct.osc2_level - 1.42).abs() < 0.0001);
    assert!((snapshot.direct.osc2_bandlimit - 0.20).abs() < 0.0001);
    assert_eq!(snapshot.direct.osc2_pitch_mode, 1.0);
    assert_eq!(snapshot.direct.noise_color, 2.0);
    assert_eq!(snapshot.direct.cross_mix_mode, 4.0);
    assert_eq!(snapshot.direct.filter_model, 2.0);
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
    assert_eq!(exported.engine.osc1.bandlimit, 0.25);
    assert_eq!(exported.engine.osc2.level, 1.42);
    assert_eq!(exported.engine.osc2.pitch_mode, Osc2PitchMode::Ratio);
    assert_eq!(exported.engine.osc2.bandlimit, 0.20);
    assert_eq!(exported.engine.noise_color, NoiseColor::Dark);
    assert_eq!(exported.engine.cross_mix_mode, CrossMixMode::Difference);
    assert_eq!(exported.engine.filter.model, FilterModel::MatterDriven);
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
