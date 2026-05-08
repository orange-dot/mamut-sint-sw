use super::*;

#[test]
fn tagged_capture_preview_uses_audio_capture_name_contract() {
    let preview = tagged_output_capture_preview(
        Path::new("patches/factory/Cathedral Bloom.toml"),
        "Gravitacija Take!",
        30,
    );

    assert_eq!(
        preview.file_name().and_then(|name| name.to_str()),
        Some("cathedral-bloom-gravitacija-take-30s-<timestamp>.wav")
    );
}

#[test]
fn output_recording_midi_log_path_uses_wav_stem_sidecar() {
    assert_eq!(
        output_recording_midi_log_path(Path::new("/tmp/mamut-take.wav")),
        PathBuf::from("/tmp/mamut-take.midi.log")
    );
    assert_eq!(
        output_recording_midi_log_path(Path::new("/tmp/mamut-take")),
        PathBuf::from("/tmp/mamut-take.midi.log")
    );
}

#[test]
fn user_patch_filename_uses_sound_lab_export_contract() {
    assert_eq!(
        generated_user_patch_filename_at("My Brass Lab", 0),
        "my-brass-lab-19700101-000000.toml"
    );
}

#[test]
fn performance_tabs_include_sound_lab() {
    assert_eq!(
        PerformanceTab::ALL,
        [
            PerformanceTab::Live,
            PerformanceTab::SoundLab,
            PerformanceTab::Engine,
            PerformanceTab::Pc4,
            PerformanceTab::Debug
        ]
    );
}

#[test]
fn request_patch_command_returns_current_engine_patch() {
    let mut state = test_engine_thread_state();
    let (reply_tx, reply_rx) = mpsc::channel();

    assert!(state.handle_command(EngineCommand::RequestPatch(reply_tx)));

    let patch = reply_rx
        .recv_timeout(Duration::from_millis(50))
        .expect("patch reply arrives");
    assert_eq!(patch.meta.patch_name, "Molten Horizon");
}

#[test]
fn direct_param_raw_value_exposes_sound_lab_values() {
    let snapshot = test_snapshot();

    assert_eq!(
        direct_param_raw_value(&snapshot, ParamId::GravitacijaMacro),
        Some(snapshot.live_macros.gravitacija)
    );
    assert_eq!(
        direct_param_raw_value(&snapshot, ParamId::Osc2SyncAmount),
        Some(snapshot.direct.sync_amount)
    );
    assert_eq!(
        direct_param_raw_value(&snapshot, ParamId::ChorusEnabled),
        Some(if snapshot.direct.chorus_enabled {
            1.0
        } else {
            0.0
        })
    );
    assert_eq!(
        direct_param_raw_value(&snapshot, ParamId::PerformanceAftertouchToBaklja),
        Some(snapshot.performance_response.aftertouch_to_baklja)
    );
    assert_eq!(
        direct_param_raw_value(&snapshot, ParamId::Osc1PulseWidth),
        Some(snapshot.direct.osc1_pulse_width)
    );
    assert_eq!(
        direct_param_raw_value(&snapshot, ParamId::NoiseColor),
        Some(snapshot.direct.noise_color)
    );
    assert_eq!(
        direct_param_raw_value(&snapshot, ParamId::CrossMixMode),
        Some(snapshot.direct.cross_mix_mode)
    );
    assert_eq!(
        direct_param_raw_value(&snapshot, ParamId::SpectralLevel),
        Some(snapshot.direct.spectral_level)
    );
    assert_eq!(
        direct_param_raw_value(&snapshot, ParamId::SpectralTable),
        Some(snapshot.direct.spectral_table)
    );
    assert_eq!(
        direct_param_raw_value(&snapshot, ParamId::SpectralPosition),
        Some(snapshot.direct.spectral_position)
    );
    assert_eq!(
        direct_param_raw_value(&snapshot, ParamId::AdditiveLevel),
        Some(snapshot.direct.additive_level)
    );
    assert_eq!(
        direct_param_raw_value(&snapshot, ParamId::AdditiveRandomDetuneCents),
        Some(snapshot.direct.additive_random_detune_cents)
    );
}

#[test]
fn sound_lab_page_midi_maps_are_compact_and_leave_layer_controls_global() {
    for page in SoundLabPage::ALL {
        let knobs = sound_lab_page_knob_bindings(page);
        let sliders = sound_lab_page_slider_bindings(page);
        assert!(knobs.len() <= 9, "{page:?} has too many knob bindings");
        assert!(sliders.len() <= 9, "{page:?} has too many slider bindings");
        assert!(
            !knobs
                .iter()
                .any(|binding| { matches!(binding.source, SoundLabMidiSource::Knob(8)) })
        );
        assert!(
            !sliders
                .iter()
                .any(|binding| { matches!(binding.source, SoundLabMidiSource::Slider(9)) })
        );
    }
}

#[test]
fn sound_lab_pc4_source_detection_matches_profile_surface() {
    let profile = pc4_full_profile();

    assert_eq!(
        sound_lab_source_for_cc(71, Some(&profile)),
        Some(SoundLabMidiSource::Knob(1))
    );
    assert_eq!(
        sound_lab_source_for_cc(18, Some(&profile)),
        Some(SoundLabMidiSource::Knob(7))
    );
    assert_eq!(
        sound_lab_source_for_cc(23, Some(&profile)),
        Some(SoundLabMidiSource::Slider(4))
    );
    assert_eq!(
        sound_lab_source_for_cc(1, Some(&profile)),
        Some(SoundLabMidiSource::ModWheel)
    );
    assert_eq!(sound_lab_source_for_cc(3, Some(&profile)), None);
    assert_eq!(sound_lab_source_for_cc(28, Some(&profile)), None);
    assert_eq!(sound_lab_source_for_cc(90, Some(&profile)), None);
    assert_eq!(sound_lab_source_for_cc(64, Some(&profile)), None);
}

#[test]
fn sound_lab_interactive_mapping_uses_param_units() {
    assert_eq!(
        sound_lab_param_value_from_normalized(ParamId::SpectralTable, 0.99),
        4.0
    );
    assert_eq!(
        sound_lab_param_value_from_normalized(ParamId::NoiseColor, 0.60),
        2.0
    );
    assert_eq!(
        sound_lab_param_value_from_normalized(ParamId::ChorusEnabled, 0.49),
        0.0
    );
    assert_eq!(
        sound_lab_param_value_from_normalized(ParamId::ChorusEnabled, 0.50),
        1.0
    );
    assert_eq!(
        sound_lab_param_value_from_normalized(ParamId::Osc2IntervalSemitones, 0.50),
        0.0
    );
    assert_eq!(
        sound_lab_param_value_from_normalized(ParamId::Osc2IntervalSemitones, 0.0),
        param_spec(ParamId::Osc2IntervalSemitones).min
    );
    assert!(
        (sound_lab_param_value_from_normalized(ParamId::FilterCutoffHz, 1.0)
            - param_spec(ParamId::FilterCutoffHz).max)
            .abs()
            < 0.01
    );
}

#[test]
fn sound_lab_surface_bindings_are_snapshot_visible() {
    let snapshot = test_snapshot();

    for page in SoundLabPage::ALL {
        for binding in sound_lab_page_knob_bindings(page)
            .iter()
            .chain(sound_lab_page_slider_bindings(page).iter())
        {
            assert!(
                direct_param_raw_value(&snapshot, binding.id).is_some(),
                "{page:?} {:?} should be visible",
                binding.id
            );
        }
        for id in sound_lab_page_mod_wheel_params(page) {
            assert!(
                direct_param_raw_value(&snapshot, *id).is_some(),
                "{page:?} {id:?} should be visible"
            );
        }
    }
}

#[test]
fn sound_lab_overlay_remaps_profile_controls_when_focus_is_active() {
    let profile = pc4_full_profile();
    let overlay =
        sound_lab_midi_overlay_events(&[0xB0, 71, 127], Some(&profile), Some(SoundLabPage::Osc1))
            .expect("K1 overlays Osc1 page");

    assert_eq!(overlay.page, SoundLabPage::Osc1);
    assert_eq!(overlay.source, SoundLabMidiSource::Knob(1));
    let events = overlay.iter().collect::<Vec<_>>();
    assert_eq!(events.len(), 1);
    match events[0] {
        ControllerEvent::DirectParam { id, value } => {
            assert_eq!(id, ParamId::Osc1SawLevel);
            assert!((value - 1.0).abs() < 0.0001);
        }
        other => panic!("unexpected overlay event: {other:?}"),
    }
}

#[test]
fn sound_lab_overlay_is_inactive_without_focus() {
    let profile = pc4_full_profile();
    assert_eq!(
        sound_lab_midi_overlay_events(&[0xB0, 71, 127], Some(&profile), None),
        None
    );
}

#[test]
fn sound_lab_overlay_leaves_global_controls_unclaimed() {
    let profile = pc4_full_profile();
    for message in [
        [0xB0, 3, 127],  // K8 GFM gate
        [0xB0, 28, 127], // S9 BCS gain
        [0xB0, 90, 127], // SW9 BCS enable
        [0xB0, 64, 127], // sustain
        [0x90, 60, 100], // note on
    ] {
        assert_eq!(
            sound_lab_midi_overlay_events(&message, Some(&profile), Some(SoundLabPage::Osc1)),
            None
        );
    }
}

#[test]
fn sound_lab_mod_wheel_maps_to_page_macro_params() {
    let profile = pc4_full_profile();
    let overlay = sound_lab_midi_overlay_events(
        &[0xB0, 1, 64],
        Some(&profile),
        Some(SoundLabPage::Relations),
    )
    .expect("mod wheel overlays Relations page");

    assert_eq!(overlay.source, SoundLabMidiSource::ModWheel);
    let ids = overlay
        .iter()
        .map(|event| match event {
            ControllerEvent::DirectParam { id, .. } => id,
            other => panic!("unexpected overlay event: {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec![
            ParamId::FmAmount,
            ParamId::PhaseModAmount,
            ParamId::RingModAmount,
            ParamId::CrossMixAmount,
        ]
    );
}

#[test]
fn sound_lab_spectral_page_maps_knobs_and_mod_wheel() {
    let profile = pc4_full_profile();
    let overlay = sound_lab_midi_overlay_events(
        &[0xB0, 71, 127],
        Some(&profile),
        Some(SoundLabPage::Spectral),
    )
    .expect("K1 overlays Spectral page");
    let events = overlay.iter().collect::<Vec<_>>();
    match events[0] {
        ControllerEvent::DirectParam { id, value } => {
            assert_eq!(id, ParamId::SpectralLevel);
            assert!((value - 1.0).abs() < 0.0001);
        }
        other => panic!("unexpected overlay event: {other:?}"),
    }

    let table_overlay = sound_lab_midi_overlay_events(
        &[0xB0, 73, 127],
        Some(&profile),
        Some(SoundLabPage::Spectral),
    )
    .expect("K2 overlays Spectral table");
    match table_overlay.iter().next().expect("one event") {
        ControllerEvent::DirectParam { id, value } => {
            assert_eq!(id, ParamId::SpectralTable);
            assert_eq!(value, 4.0);
        }
        other => panic!("unexpected overlay event: {other:?}"),
    }

    let mw_overlay =
        sound_lab_midi_overlay_events(&[0xB0, 1, 64], Some(&profile), Some(SoundLabPage::Spectral))
            .expect("MW overlays Spectral page");
    let ids = mw_overlay
        .iter()
        .map(|event| match event {
            ControllerEvent::DirectParam { id, .. } => id,
            other => panic!("unexpected overlay event: {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(ids, vec![ParamId::SpectralPosition]);
}

#[test]
fn sound_lab_overlay_rounds_indexed_params() {
    let profile = pc4_full_profile();
    let high =
        sound_lab_midi_overlay_events(&[0xB0, 18, 127], Some(&profile), Some(SoundLabPage::Osc2))
            .expect("K7 overlays Osc2 pitch mode");
    let low =
        sound_lab_midi_overlay_events(&[0xB0, 18, 0], Some(&profile), Some(SoundLabPage::Osc2))
            .expect("K7 overlays Osc2 pitch mode");

    for (overlay, expected) in [(high, 1.0), (low, 0.0)] {
        let events = overlay.iter().collect::<Vec<_>>();
        assert_eq!(events.len(), 1);
        match events[0] {
            ControllerEvent::DirectParam { id, value } => {
                assert_eq!(id, ParamId::Osc2PitchMode);
                assert_eq!(value, expected);
            }
            other => panic!("unexpected overlay event: {other:?}"),
        }
    }
}

#[test]
fn sound_lab_badges_include_mod_wheel_page_macro() {
    assert_eq!(
        sound_lab_badges_for_param(SoundLabPage::Relations, ParamId::FmAmount),
        vec!["K1".to_string(), "MW".to_string()]
    );
    assert_eq!(
        sound_lab_badges_for_param(SoundLabPage::Spectral, ParamId::SpectralPosition),
        vec!["K3".to_string(), "MW".to_string()]
    );
}

#[test]
fn sound_lab_extension_records_runtime_layer_intent() {
    let mut state = test_engine_thread_state();
    state
        .engine
        .set_gfm_layer_mode(GfmLayerMode::Enabled { seed: 0x6A46_4D40 });
    state.engine.set_bcs_layer_mode(BcsLayerMode::Enabled {
        scenario: BcsScenario::SubharmonicPressure,
    });
    let snapshot = state.engine.snapshot();

    let extensions = crate::session::sound_lab_extension_table(
        toml::Table::new(),
        &snapshot,
        Path::new("patches/factory/molten-horizon.toml"),
    );
    let sound_lab = extensions
        .get("sound_lab")
        .and_then(toml::Value::as_table)
        .expect("sound lab table exists");

    assert_eq!(
        sound_lab.get("gfm_enabled").and_then(toml::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        sound_lab.get("gfm_seed").and_then(toml::Value::as_str),
        Some("0x6A464D40")
    );
    assert_eq!(
        sound_lab.get("bcs_enabled").and_then(toml::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        sound_lab.get("bcs_scenario").and_then(toml::Value::as_str),
        Some("subharmonic-pressure")
    );
}
