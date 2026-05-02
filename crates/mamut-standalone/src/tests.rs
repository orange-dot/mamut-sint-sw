use super::*;

fn pc4_full_profile() -> ControllerProfile {
    controller_profile_from_toml(
        include_str!("../../../profiles/pc4-full.toml"),
        Path::new("profiles/pc4-full.toml"),
    )
    .expect("pc4-full profile parses")
}

fn test_engine_thread_state_with_patch_path(path: &Path) -> EngineThreadState {
    let patch = load_patch_from_path(path).expect("patch loads");
    let engine = Engine::new(
        EngineConfig {
            sample_rate_hz: ALSA_PLAYBACK_SAMPLE_RATE_HZ as f32,
            max_block_frames: 2_048,
            voice_count: 6,
        },
        patch,
    )
    .expect("engine builds");
    let (_tx, rx) = mpsc::channel();
    let (producer, _consumer) = RingBuffer::<StereoFrame>::new(AUDIO_QUEUE_CAPACITY_FRAMES);
    EngineThreadState::new(
        engine,
        rx,
        producer,
        Arc::new(ArrayQueue::new(MIDI_INPUT_QUEUE_CAPACITY)),
        Arc::new(PriorityActions::default()),
        Arc::new(TransportMetrics::default()),
        Arc::new(RecordingMetrics::default()),
    )
}

fn test_engine_thread_state() -> EngineThreadState {
    test_engine_thread_state_with_patch_path(&default_patch_path())
}

fn test_engine_thread_state_for_patch(stem: &str) -> EngineThreadState {
    let path = resolve_patch_argument(Some(stem)).expect("factory patch resolves");
    test_engine_thread_state_with_patch_path(&path)
}

fn test_snapshot() -> EngineSnapshot {
    let patch = load_patch_from_path(&default_patch_path()).expect("default patch loads");
    Engine::new(
        EngineConfig {
            sample_rate_hz: ALSA_PLAYBACK_SAMPLE_RATE_HZ as f32,
            max_block_frames: 2_048,
            voice_count: 6,
        },
        patch,
    )
    .expect("engine builds")
    .snapshot()
}

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
    assert!(options.trace_midi);
    assert_eq!(options.audio_selector.as_deref(), Some("hw:4,0"));
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
fn parse_play_options_defaults_gfm_layer_disabled() {
    let args = vec!["razor-thaw".to_string()];
    let options = parse_play_options(&args).expect("play options parse");

    assert_eq!(options.gfm_layer_seed, None);
    assert_eq!(options.bcs_layer_scenario, None);
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
        "bcs: mode=disabled playable=off amount=0.000 effective=0.000"
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
    assert!(enabled.contains("amount=0.000"));
    assert!(enabled.contains("effective=0.000"));
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

#[test]
fn float_stereo_wav_writer_patches_header_sizes() {
    let path = std::env::temp_dir().join(format!(
        "mamut-test-{}.wav",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after unix epoch")
            .as_nanos()
    ));
    {
        let mut writer = FloatStereoWavWriter::create(&path).expect("writer creates");
        writer.write_frame([0.25, -0.25]).expect("frame writes");
        writer.write_frame([0.5, -0.5]).expect("frame writes");
        assert_eq!(writer.finalize().expect("writer finalizes"), 2);
    }
    let bytes = fs::read(&path).expect("wav file reads");
    let _ = fs::remove_file(&path);
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    assert_eq!(&bytes[12..16], b"fmt ");
    assert_eq!(u16::from_le_bytes([bytes[20], bytes[21]]), 3);
    assert_eq!(u16::from_le_bytes([bytes[22], bytes[23]]), 2);
    assert_eq!(
        u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]),
        44_100
    );
    assert_eq!(&bytes[36..40], b"data");
    assert_eq!(
        u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]),
        16
    );
    assert_eq!(bytes.len(), 44 + 16);
}

#[test]
fn factory_bank_has_productized_patch_set() {
    let entries = factory_patch_entries().expect("factory entries load");
    assert!(entries.len() >= 8);
    assert!(entries.iter().any(|entry| entry.favorite));
}

#[test]
fn favorite_navigation_wraps_across_live_set() {
    let next = adjacent_live_patch(Path::new("patches/factory/glass-tide.toml"), 1)
        .expect("favorite next resolves");
    assert!(next.ends_with("patches/factory/molten-horizon.toml"));

    let prev = adjacent_live_patch(Path::new("patches/factory/molten-horizon.toml"), -1)
        .expect("favorite prev resolves");
    assert!(prev.ends_with("patches/factory/glass-tide.toml"));
}

#[test]
fn live_set_slots_follow_locked_pc4_order() {
    let live_set = live_patch_entries().expect("live set loads");
    assert_eq!(live_set.len(), 8);
    assert_eq!(live_set[0].stem, "molten-horizon");
    assert_eq!(live_set[1].stem, "cathedral-bloom");
    assert_eq!(live_set[7].stem, "glass-tide");
}

#[test]
fn snapshot_requests_are_one_shot_and_do_not_drain_events() {
    let mut state = test_engine_thread_state();
    let (reply_tx, reply_rx) = mpsc::channel();
    state.snapshot_requests.push(reply_tx);
    state.note_events.push(Scheduled {
        frame_offset: 0,
        event: NoteEvent::NoteOn {
            note: 60,
            velocity: 1.0,
        },
    });
    state.controller_events.push(Scheduled {
        frame_offset: 0,
        event: ControllerEvent::Macro {
            id: MacroId::Bloom,
            value: 0.5,
        },
    });

    state.flush_snapshot_requests();

    assert!(reply_rx.recv_timeout(Duration::from_millis(50)).is_ok());
    assert!(reply_rx.try_recv().is_err());
    assert!(state.snapshot_requests.is_empty());
    assert_eq!(state.note_events.len(), 1);
    assert_eq!(state.controller_events.len(), 1);
}

#[test]
fn parse_midi_message_maps_pc4_knobs_and_program_change() {
    let parsed = parse_midi_message(&[0xB0, 16, 64], 2.0, None, None).expect("cc16 parses");
    match parsed {
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(ControllerEvent::Macro {
            id,
            value,
        })) => {
            assert_eq!(id, MacroId::Gravitacija);
            assert!((value - (64.0 / 127.0)).abs() < 0.0001);
        }
        other => panic!("unexpected parsed MIDI message: {other:?}"),
    }

    let parsed = parse_midi_message(&[0xB0, 20, 100], 2.0, None, None).expect("cc20 parses");
    match parsed {
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(ControllerEvent::Macro {
            id,
            value,
        })) => {
            assert_eq!(id, MacroId::Swarm);
            assert!((value - (100.0 / 127.0)).abs() < 0.0001);
        }
        other => panic!("unexpected parsed MIDI message: {other:?}"),
    }

    let parsed = parse_midi_message(&[0xC0, 5], 2.0, None, None).expect("program change parses");
    match parsed {
        ParsedMidiMessage::Runtime(RuntimeControlMessage::ProgramChange(slot)) => {
            assert_eq!(slot, 5);
        }
        other => panic!("unexpected parsed MIDI message: {other:?}"),
    }
}

#[test]
fn parse_midi_message_covers_pc4gen_mamut_smoke_contract() {
    for (cc, expected_id) in [
        (16, MacroId::Gravitacija),
        (17, MacroId::Bloom),
        (18, MacroId::Heat),
        (19, MacroId::Ruin),
        (20, MacroId::Swarm),
    ] {
        let parsed =
            parse_midi_message(&[0xB0, cc, 127], 2.0, None, None).expect("macro control parses");
        match parsed {
            ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
                ControllerEvent::Macro { id, value },
            )) => {
                assert_eq!(id, expected_id);
                assert!((value - 1.0).abs() < 0.0001);
            }
            other => panic!("unexpected parsed MIDI message: {other:?}"),
        }
    }

    assert!(matches!(
        parse_midi_message(&[0xB0, 1, 127], 2.0, None, None),
        Some(ParsedMidiMessage::Realtime(
            RealtimeMidiMessage::Controller(ControllerEvent::ModWheel { .. })
        ))
    ));
    assert!(matches!(
        parse_midi_message(&[0xB0, 64, 127], 2.0, None, None),
        Some(ParsedMidiMessage::Realtime(
            RealtimeMidiMessage::Controller(ControllerEvent::Sustain { down: true })
        ))
    ));
    assert!(matches!(
        parse_midi_message(&[0xB0, 64, 0], 2.0, None, None),
        Some(ParsedMidiMessage::Realtime(
            RealtimeMidiMessage::Controller(ControllerEvent::Sustain { down: false })
        ))
    ));
    assert!(matches!(
        parse_midi_message(&[0xD0, 100], 2.0, None, None),
        Some(ParsedMidiMessage::Realtime(
            RealtimeMidiMessage::Controller(ControllerEvent::ChannelAftertouch { .. })
        ))
    ));
    assert!(matches!(
        parse_midi_message(&[0xE0, 0x00, 0x40], 2.0, None, None),
        Some(ParsedMidiMessage::Realtime(
            RealtimeMidiMessage::Controller(ControllerEvent::PitchBend { .. })
        ))
    ));

    let slots = (0..8)
        .map(
            |slot| match parse_midi_message(&[0xC0, slot], 2.0, None, None) {
                Some(ParsedMidiMessage::Runtime(RuntimeControlMessage::ProgramChange(slot))) => {
                    slot
                }
                other => panic!("unexpected parsed MIDI message: {other:?}"),
            },
        )
        .collect::<Vec<_>>();
    assert_eq!(slots, (0..8).collect::<Vec<_>>());
}

#[test]
fn controller_profile_loads_pc4_full_bindings() {
    let profile = pc4_full_profile();
    assert_eq!(profile.name, "pc4-full");
    assert_eq!(profile.bindings_by_cc.len(), 27);
    assert!(matches!(
        profile.binding_for_cc(71).map(|binding| binding.action),
        Some(ControllerBindingAction::Macro(MacroId::Gravitacija))
    ));
    assert!(matches!(
        profile.binding_for_cc(80).map(|binding| binding.action),
        Some(ControllerBindingAction::Runtime(
            RuntimeControlMessage::Panic
        ))
    ));
    assert!(matches!(
        profile.binding_for_cc(3).map(|binding| binding.action),
        Some(ControllerBindingAction::GfmLayerAmount)
    ));
    assert!(matches!(
        profile.binding_for_cc(28).map(|binding| binding.action),
        Some(ControllerBindingAction::BcsLayerAmount)
    ));
    assert!(matches!(
        profile.binding_for_cc(90).map(|binding| binding.action),
        Some(ControllerBindingAction::BcsLayerEnabled)
    ));
    let knobs = sorted_bindings_for_section(&profile, ControllerBindingSection::Knob);
    let sliders = sorted_bindings_for_section(&profile, ControllerBindingSection::Slider);
    let switches = sorted_bindings_for_section(&profile, ControllerBindingSection::Switch);
    assert_eq!(knobs.len(), 9);
    assert_eq!(sliders.len(), 9);
    assert_eq!(switches.len(), 9);
    assert_eq!(knobs[0].control, "K1 Filter 1");
    assert_eq!(sliders[6].control, "S7");
    assert_eq!(sliders[8].control, "S9 BCS Amount");
    assert_eq!(switches[8].control, "SW9 BCS Enable");
}

#[test]
fn controller_profile_groups_unknown_controls_as_other() {
    let input = r#"
[[binding]]
control = "X1"
cc = 44
kind = "reserved"
"#;
    let profile =
        controller_profile_from_toml(input, Path::new("other.toml")).expect("profile parses");
    let binding = profile.binding_for_cc(44).expect("binding exists");
    assert_eq!(binding.section, ControllerBindingSection::Other);
    assert_eq!(binding.index, None);
}

#[test]
fn pc4_display_values_come_from_mamut_snapshot() {
    let profile = pc4_full_profile();
    let snapshot = test_snapshot();
    let cutoff = profile.binding_for_cc(26).expect("s7 binding");
    let attack = profile.binding_for_cc(72).expect("k3 binding");
    let chorus = profile.binding_for_cc(85).expect("sw5 binding");
    let gfm_gate = profile.binding_for_cc(3).expect("k8 binding");
    let bcs_amount = profile.binding_for_cc(28).expect("s9 binding");
    let bcs_enable = profile.binding_for_cc(90).expect("sw9 binding");

    assert!(
        binding_display_value(&snapshot, cutoff).0.contains("Hz"),
        "cutoff should display in Hz"
    );
    assert!(
        binding_display_value(&snapshot, attack).0.contains("ms"),
        "attack should display in ms"
    );
    assert!(matches!(
        binding_display_value(&snapshot, chorus).0.as_str(),
        "on" | "off"
    ));
    assert_eq!(binding_display_value(&snapshot, gfm_gate).0, "0.00");
    assert_eq!(binding_display_value(&snapshot, bcs_amount).0, "0.00");
    assert_eq!(binding_display_value(&snapshot, bcs_enable).0, "off");
}

#[test]
fn last_control_event_tracks_profile_gfm_gate_and_program_change() {
    let profile = pc4_full_profile();
    let parsed = parse_midi_message(&[0xB0, 3, 64], 2.0, None, Some(&profile));
    let event = last_control_event(
        &[0xB0, 3, 64],
        None,
        Some(&profile),
        parsed,
        Instant::now(),
        false,
    )
    .expect("gfm amount event");
    assert_eq!(event.kind, LastControlKind::ProfileCc(3));
    assert_eq!(event.action, "gfm layer gate");
    assert_eq!(event.verdict, LastControlVerdict::Accepted);

    let parsed = parse_midi_message(&[0xC0, 4], 2.0, None, Some(&profile));
    let event = last_control_event(
        &[0xC0, 4],
        None,
        Some(&profile),
        parsed,
        Instant::now(),
        false,
    )
    .expect("program change event");
    assert_eq!(event.kind, LastControlKind::ProgramChange(4));
    assert_eq!(event.program, Some(4));
}

#[test]
fn midi_startup_guard_suppresses_realtime_and_runtime_messages() {
    let now = Instant::now();
    let guard_until = now + MIDI_STARTUP_GUARD;
    let note = parse_midi_message(&[0x90, 48, 92], 2.0, Some(1), None);
    let cc = parse_midi_message(&[0xB0, 1, 64], 2.0, Some(1), None);
    let program = parse_midi_message(&[0xC0, 2], 2.0, Some(1), None);

    assert!(startup_guard_suppresses_message(note, now, guard_until));
    assert!(startup_guard_suppresses_message(cc, now, guard_until));
    assert!(startup_guard_suppresses_message(program, now, guard_until));
    assert!(!startup_guard_suppresses_message(
        note,
        guard_until,
        guard_until
    ));
    assert!(!startup_guard_suppresses_message(None, now, guard_until));
}

#[test]
fn last_control_event_marks_startup_suppressed_profile_cc() {
    let profile = pc4_full_profile();
    let parsed = parse_midi_message(&[0xB0, 71, 127], 2.0, None, Some(&profile));
    let event = last_control_event(
        &[0xB0, 71, 127],
        None,
        Some(&profile),
        None,
        Instant::now(),
        startup_guard_suppresses_message(
            parsed,
            Instant::now(),
            Instant::now() + MIDI_STARTUP_GUARD,
        ),
    )
    .expect("profile cc event");

    assert_eq!(event.kind, LastControlKind::ProfileCc(71));
    assert_eq!(event.verdict, LastControlVerdict::StartupSuppressed);
}

#[test]
fn parse_midi_message_uses_pc4_full_profile() {
    let profile = pc4_full_profile();

    let parsed =
        parse_midi_message(&[0xB0, 71, 127], 2.0, None, Some(&profile)).expect("k1 parses");
    match parsed {
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(ControllerEvent::Macro {
            id,
            value,
        })) => {
            assert_eq!(id, MacroId::Gravitacija);
            assert!((value - 1.0).abs() < 0.0001);
        }
        other => panic!("unexpected parsed MIDI message: {other:?}"),
    }

    let parsed =
        parse_midi_message(&[0xB0, 72, 127], 2.0, None, Some(&profile)).expect("k3 parses");
    match parsed {
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
            ControllerEvent::DirectParam { id, value },
        )) => {
            assert_eq!(id, ParamId::AmpEnvAttackMs);
            assert!((value - param_spec(ParamId::AmpEnvAttackMs).max).abs() < 0.01);
        }
        other => panic!("unexpected parsed MIDI message: {other:?}"),
    }

    let parsed =
        parse_midi_message(&[0xB0, 26, 127], 2.0, None, Some(&profile)).expect("s7 parses");
    match parsed {
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
            ControllerEvent::DirectParam { id, value },
        )) => {
            assert_eq!(id, ParamId::FilterCutoffHz);
            assert!((value - param_spec(ParamId::FilterCutoffHz).max).abs() < 0.01);
        }
        other => panic!("unexpected parsed MIDI message: {other:?}"),
    }

    assert!(matches!(
        parse_midi_message(&[0xB0, 80, 127], 2.0, None, Some(&profile)),
        Some(ParsedMidiMessage::Runtime(RuntimeControlMessage::Panic))
    ));
    assert!(parse_midi_message(&[0xB0, 80, 0], 2.0, None, Some(&profile)).is_none());
    match parse_midi_message(&[0xB0, 3, 64], 2.0, None, Some(&profile)).expect("k8 parses") {
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
            ControllerEvent::GfmLayerAmount { amount },
        )) => assert!((amount - (64.0 / 127.0)).abs() < 0.0001),
        other => panic!("unexpected parsed MIDI message: {other:?}"),
    }
    match parse_midi_message(&[0xB0, 28, 96], 2.0, None, Some(&profile)).expect("s9 parses") {
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
            ControllerEvent::BcsLayerAmount { amount },
        )) => assert!((amount - (96.0 / 127.0)).abs() < 0.0001),
        other => panic!("unexpected parsed MIDI message: {other:?}"),
    }
    assert!(matches!(
        parse_midi_message(&[0xB0, 90, 127], 2.0, None, Some(&profile)),
        Some(ParsedMidiMessage::Realtime(
            RealtimeMidiMessage::Controller(ControllerEvent::BcsLayerEnabled { enabled: true })
        ))
    ));
    assert!(matches!(
        parse_midi_message(&[0xB0, 90, 0], 2.0, None, Some(&profile)),
        Some(ParsedMidiMessage::Realtime(
            RealtimeMidiMessage::Controller(ControllerEvent::BcsLayerEnabled { enabled: false })
        ))
    ));
}

#[test]
fn controller_profile_rejects_duplicate_cc() {
    let input = r#"
[[binding]]
control = "A"
cc = 71
kind = "macro"
target = "gravitacija"

[[binding]]
control = "B"
cc = 71
kind = "macro"
target = "bloom"
"#;
    assert!(controller_profile_from_toml(input, Path::new("dup.toml")).is_err());
}

#[test]
fn controller_profile_rejects_unknown_param() {
    let input = r#"
[[binding]]
control = "S1"
cc = 12
kind = "direct_param"
target = "not_a_param"
"#;
    assert!(controller_profile_from_toml(input, Path::new("unknown.toml")).is_err());
}

#[test]
fn controller_profile_rejects_boolean_direct_param() {
    let input = r#"
[[binding]]
control = "SW5"
cc = 85
kind = "direct_param"
target = "chorus_enabled"
"#;
    assert!(controller_profile_from_toml(input, Path::new("boolean.toml")).is_err());
}

#[test]
fn parse_midi_message_respects_channel_filter() {
    assert!(parse_midi_message(&[0x90, 60, 100], 2.0, Some(2), None).is_none());

    let parsed =
        parse_midi_message(&[0x91, 60, 100], 2.0, Some(2), None).expect("channel 2 note parses");
    match parsed {
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Note(NoteEvent::NoteOn {
            note,
            velocity,
        })) => {
            assert_eq!(note, 60);
            assert!((velocity - (100.0 / 127.0)).abs() < 0.0001);
        }
        other => panic!("unexpected parsed MIDI message: {other:?}"),
    }
}

#[test]
fn queue_drain_silence_fills_missing_frames_and_counts_underrun() {
    let metrics = Arc::new(TransportMetrics::default());
    let (mut producer, mut consumer) = RingBuffer::<StereoFrame>::new(4);
    push_frames_into_queue(&mut producer, &metrics, &[[0.25, -0.25]]);

    let mut output = vec![1.0_f32; 6];
    drain_queue_into_output(&mut consumer, &metrics, &mut output, 2);

    assert_eq!(output, vec![0.25, -0.25, 0.0, 0.0, 0.0, 0.0]);
    assert_eq!(
        metrics.snapshot(),
        TransportMetricsSnapshot {
            queued_frames: 0,
            queue_target_frames: 0,
            write_frames_hint: 3,
            underrun_batches: 1,
            underrun_frames: 2,
            xrun_recoveries: 0,
            overflow_batches: 0,
            overflow_frames: 0,
        }
    );
}

#[test]
fn queue_drain_zero_fills_partial_tail_samples() {
    let metrics = Arc::new(TransportMetrics::default());
    let (mut producer, mut consumer) = RingBuffer::<StereoFrame>::new(4);
    push_frames_into_queue(&mut producer, &metrics, &[[0.5, -0.5]]);

    let mut output = vec![1.0_f32; 5];
    drain_queue_into_output(&mut consumer, &metrics, &mut output, 2);

    assert_eq!(output, vec![0.5, -0.5, 0.0, 0.0, 0.0]);
}

#[test]
fn s32_playback_conversion_clips_and_sanitizes_samples() {
    assert_eq!(f32_sample_to_s32(1.0), i32::MAX);
    assert_eq!(f32_sample_to_s32(2.0), i32::MAX);
    assert_eq!(f32_sample_to_s32(-1.0), i32::MIN);
    assert_eq!(f32_sample_to_s32(-2.0), i32::MIN);
    assert_eq!(f32_sample_to_s32(0.0), 0);
    assert_eq!(f32_sample_to_s32(f32::NAN), 0);
}

#[test]
fn s32_playback_conversion_preserves_interleaved_order() {
    let samples = [0.0, 1.0, -1.0, f32::INFINITY];
    let mut output = [123_i32; 4];

    convert_f32_samples_to_s32(&samples, &mut output);

    assert_eq!(output, [0, i32::MAX, i32::MIN, 0]);
}

#[test]
fn queue_push_records_overflow_without_blocking() {
    let metrics = Arc::new(TransportMetrics::default());
    let (mut producer, mut consumer) = RingBuffer::<StereoFrame>::new(2);
    push_frames_into_queue(
        &mut producer,
        &metrics,
        &[[0.1, 0.2], [0.3, 0.4], [0.5, 0.6]],
    );

    assert_eq!(
        metrics.snapshot(),
        TransportMetricsSnapshot {
            queued_frames: 2,
            queue_target_frames: 0,
            write_frames_hint: 0,
            underrun_batches: 0,
            underrun_frames: 0,
            xrun_recoveries: 0,
            overflow_batches: 1,
            overflow_frames: 1,
        }
    );
    assert_eq!(consumer.pop().expect("first frame"), [0.1, 0.2]);
    assert_eq!(consumer.pop().expect("second frame"), [0.3, 0.4]);
    assert!(consumer.pop().is_err());
}

#[test]
fn queue_target_stays_on_default_without_write_hint() {
    let metrics = TransportMetrics::default();
    assert_eq!(
        metrics.queue_target_frames(1_024),
        AUDIO_QUEUE_TARGET_FRAMES
    );
}

#[test]
fn queue_target_scales_to_observed_write_size() {
    let metrics = TransportMetrics::default();
    metrics.record_write_request(900);

    assert_eq!(metrics.queue_target_frames(1_536), 1_024);
    assert_eq!(
        metrics.snapshot(),
        TransportMetricsSnapshot {
            queued_frames: 0,
            queue_target_frames: 1_024,
            write_frames_hint: 900,
            underrun_batches: 0,
            underrun_frames: 0,
            xrun_recoveries: 0,
            overflow_batches: 0,
            overflow_frames: 0,
        }
    );
}
