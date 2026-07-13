use super::*;

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
    assert_eq!(sliders[8].control, "S9 BCS Gain");
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
fn midi_trace_timing_reports_elapsed_and_delta() {
    let started_at = Instant::now();
    let mut previous_trace_at = None;

    let first = MidiTraceTiming::from_received_at(
        started_at,
        &mut previous_trace_at,
        started_at + std::time::Duration::from_millis(250),
    );
    assert!((first.elapsed_seconds - 0.250).abs() < 0.001);
    assert_eq!(first.delta_millis, 0.0);

    let second = MidiTraceTiming::from_received_at(
        started_at,
        &mut previous_trace_at,
        started_at + std::time::Duration::from_millis(375),
    );
    assert!((second.elapsed_seconds - 0.375).abs() < 0.001);
    assert!((second.delta_millis - 125.0).abs() < 0.001);
}

#[test]
fn midi_trace_format_includes_timing_channel_raw_and_verdict() {
    let profile = pc4_full_profile();
    let parsed = parse_midi_message(&[0xB0, 28, 64], 2.0, Some(1), Some(&profile));
    let line = format_midi_trace_message(
        &[0xB0, 28, 64],
        Some(1),
        Some(&profile),
        parsed,
        MidiTraceTiming {
            elapsed_seconds: 1.234,
            delta_millis: 12.3,
        },
    );

    assert!(line.contains("midi trace: t=1.234s dt=12.3ms ch=1"));
    assert!(line.contains("raw=[B0 1C 40]"));
    assert!(line.contains("profile S9 BCS Gain"));
    assert!(line.contains("bcs layer gain=0.504"));
}

#[test]
fn midi_trace_sidecar_writes_header_lines_footer_and_stops() {
    let wav_path = std::env::temp_dir().join(format!(
        "mamut-midi-sidecar-{}.wav",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after unix epoch")
            .as_nanos()
    ));
    let log_path = output_recording_midi_log_path(&wav_path);
    let log = MidiTraceLog::default();
    log.start(MidiTraceLogStart {
        wav_path: wav_path.clone(),
        log_path: log_path.clone(),
        patch_path: PathBuf::from("patches/factory/molten-horizon.toml"),
        patch_name: "Molten Horizon".to_string(),
        sample_rate_hz: 96_000,
        max_frames: Some(96_000),
        midi_channel: Some(1),
        controller_profile: Some("PC4 Full (profiles/pc4-full.toml)".to_string()),
    })
    .expect("sidecar starts");

    let now = Instant::now();
    log.write_line(
        now,
        "midi trace: t=0.100s dt=0.0ms ch=1 raw=[90 40 60] note on note=64 velocity=0.756",
    );
    log.write_line(
        now,
        "midi trace: t=0.200s dt=100.0ms ch=1 raw=[80 40 40] note off note=64",
    );
    log.finish("test finished");
    log.write_line(
        now,
        "midi trace: t=0.300s dt=100.0ms ch=1 raw=[90 41 60] ignored",
    );

    let text = fs::read_to_string(&log_path).expect("sidecar reads");
    let _ = fs::remove_file(&log_path);
    assert!(text.contains("# Mamut MIDI trace sidecar v1"));
    assert!(text.contains("# wav_path:"));
    assert!(text.contains("# midi_log_path:"));
    assert!(text.contains("# patch_name: Molten Horizon"));
    assert!(text.contains("# sample_rate_hz: 96000"));
    assert!(text.contains("raw=[90 40 60]"));
    assert!(text.contains("raw=[80 40 40]"));
    assert!(!text.contains("raw=[90 41 60]"));
    assert!(text.contains("# finish: test finished"));
    assert!(text.contains("# midi_lines: 2"));
}

#[test]
fn midi_trace_worker_drains_records_and_writes_footer_on_shutdown() {
    let wav_path = std::env::temp_dir().join(format!(
        "mamut-midi-worker-{}.wav",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after unix epoch")
            .as_nanos()
    ));
    let log_path = output_recording_midi_log_path(&wav_path);
    let log = Arc::new(MidiTraceLog::default());
    let metrics = Arc::new(InputMetrics::default());
    log.start(MidiTraceLogStart {
        wav_path: wav_path.clone(),
        log_path: log_path.clone(),
        patch_path: PathBuf::from("patches/factory/molten-horizon.toml"),
        patch_name: "Molten Horizon".to_string(),
        sample_rate_hz: 96_000,
        max_frames: Some(96_000),
        midi_channel: Some(1),
        controller_profile: None,
    })
    .expect("sidecar starts");
    let mut worker = MidiTraceWorker::spawn(false, Some(1), None, Arc::clone(&metrics), log);
    let now = Instant::now();
    worker.publisher().publish(RawMidiTraceRecord::new(
        &[0x90, 64, 96],
        now,
        MidiTraceTiming {
            elapsed_seconds: 0.1,
            delta_millis: 0.0,
        },
        Some(ParsedMidiMessage::Realtime(RealtimeMidiMessage::Note(
            NoteEvent::NoteOn {
                note: 64,
                velocity: 96.0 / 127.0,
            },
        ))),
        None,
        false,
    ));
    worker.shutdown();

    let text = fs::read_to_string(&log_path).expect("sidecar reads");
    let _ = fs::remove_file(&log_path);
    assert!(text.contains("raw=[90 40 60]"));
    assert!(text.contains("# finish: midi trace worker shutdown"));
    assert!(text.contains("# midi_lines: 1"));
    assert_eq!(metrics.snapshot().trace_records_dropped, 0);
}

fn mozaik_control_profile(cc: u8, target: &str) -> anyhow::Result<ControllerProfile> {
    let input = format!(
        "[[binding]]\ncontrol = \"Mozaik\"\ncc = {cc}\nkind = \"mozaik_control\"\ntarget = \"{target}\"\n"
    );
    controller_profile_from_toml(&input, Path::new("mozaik.toml"))
}

#[test]
fn mozaik_control_profile_parses_target_matrix() {
    use mamut_engine::MozaikParam;
    let cases = [
        ("mix", MozaikParam::Mix),
        ("slope", MozaikParam::Slope),
        ("contrast", MozaikParam::Contrast),
        ("phason", MozaikParam::Phason),
        ("drift", MozaikParam::Drift),
    ];
    for (target, expected) in cases {
        let profile = mozaik_control_profile(21, target).expect("mozaik profile parses");
        assert_eq!(
            profile.binding_for_cc(21).map(|binding| binding.action),
            Some(ControllerBindingAction::MozaikControl(expected)),
            "target {target}"
        );
    }
}

#[test]
fn mozaik_control_profile_rejects_unknown_and_missing_target() {
    let unknown = mozaik_control_profile(21, "wobble").expect_err("unknown target rejected");
    assert!(
        format!("{unknown:#}").contains("unknown mozaik control"),
        "error should name the bad target: {unknown:#}"
    );

    let missing = controller_profile_from_toml(
        "[[binding]]\ncontrol = \"Mozaik\"\ncc = 21\nkind = \"mozaik_control\"\n",
        Path::new("mozaik.toml"),
    )
    .expect_err("missing target rejected");
    assert!(
        format!("{missing:#}").contains("mozaik_control binding requires target"),
        "error should demand a target: {missing:#}"
    );
}

#[test]
fn mozaik_control_cc_routes_to_bounded_runtime_control_queue() {
    use mamut_engine::MozaikParam;
    // A mozaik_control CC becomes a RuntimeControlMessage (bounded control queue),
    // NOT a realtime ControllerEvent — the session drains it into set_mozaik_param.
    let profile = mozaik_control_profile(21, "mix").expect("profile parses");
    let parsed = parse_midi_message(&[0xB0, 21, 64], 2.0, None, Some(&profile)).expect("cc parses");
    match parsed {
        ParsedMidiMessage::Runtime(RuntimeControlMessage::MozaikControl(param, value)) => {
            assert_eq!(param, MozaikParam::Mix);
            assert!(
                (value - 64.0 / 127.0).abs() < 1.0e-6,
                "linear 0..127 -> 0..1"
            );
        }
        other => panic!("expected runtime mozaik control, got {other:?}"),
    }
}

#[test]
fn mozaik_control_cc_and_headless_set_converge_on_same_param() {
    use mamut_engine::MozaikParam;
    // Convergence: both the bound CC and the headless `mozaik set` resolve to the
    // same MozaikParam, which the session feeds to the one set_mozaik_param path
    // (EngineCommand::SetMozaikParam). Disabled-layer behavior is therefore whatever
    // the landed SET5-4 engine semantics are — there is no second parameter path.
    let profile = mozaik_control_profile(21, "slope").expect("profile parses");
    let cc_param = match parse_midi_message(&[0xB0, 21, 80], 2.0, None, Some(&profile)) {
        Some(ParsedMidiMessage::Runtime(RuntimeControlMessage::MozaikControl(param, _))) => param,
        other => panic!("expected runtime mozaik control, got {other:?}"),
    };
    let headless_param = match parse_runtime_ui_command("mozaik set slope 0.63") {
        Ok(RuntimeUiCommand::MozaikSet(param, _)) => param,
        other => panic!("expected headless mozaik set, got {other:?}"),
    };
    assert_eq!(cc_param, MozaikParam::Slope);
    assert_eq!(cc_param, headless_param);
}

#[test]
fn mozaik_control_binding_names_appear_in_trace_and_last_control() {
    let profile = mozaik_control_profile(21, "mix").expect("profile parses");
    let parsed = parse_midi_message(&[0xB0, 21, 100], 2.0, Some(1), Some(&profile));
    let line = format_midi_trace_message(
        &[0xB0, 21, 100],
        Some(1),
        Some(&profile),
        parsed,
        MidiTraceTiming {
            elapsed_seconds: 2.0,
            delta_millis: 5.0,
        },
    );
    assert!(
        line.contains("profile Mozaik"),
        "trace names the control: {line}"
    );
    assert!(
        line.contains("mozaik mix"),
        "trace names the action: {line}"
    );

    let event = last_control_event(
        &[0xB0, 21, 100],
        Some(1),
        Some(&profile),
        parsed,
        Instant::now(),
        false,
    )
    .expect("mozaik control event");
    assert_eq!(event.action, "mozaik mix");
    assert_eq!(event.verdict, LastControlVerdict::Accepted);
}

#[test]
fn android_touch_profile_binds_macros_expression_and_mozaik() {
    use mamut_engine::MozaikParam;
    let profile = controller_profile_from_toml(
        include_str!("../../../../profiles/android-touch.toml"),
        Path::new("android-touch.toml"),
    )
    .expect("android-touch profile parses");
    assert_eq!(profile.name, "android-touch");
    // Macros CC16-20.
    assert!(matches!(
        profile.binding_for_cc(16).map(|binding| binding.action),
        Some(ControllerBindingAction::Macro(MacroId::Gravitacija))
    ));
    assert!(matches!(
        profile.binding_for_cc(20).map(|binding| binding.action),
        Some(ControllerBindingAction::Macro(MacroId::Swarm))
    ));
    // Expression CC11 -> a direct param (master output level).
    assert!(matches!(
        profile.binding_for_cc(11).map(|binding| binding.action),
        Some(ControllerBindingAction::DirectParam { .. })
    ));
    // Mozaik session layer on the Profile CC 21-31 band, mix first.
    assert_eq!(
        profile.binding_for_cc(21).map(|binding| binding.action),
        Some(ControllerBindingAction::MozaikControl(MozaikParam::Mix))
    );
    assert_eq!(
        profile.binding_for_cc(25).map(|binding| binding.action),
        Some(ControllerBindingAction::MozaikControl(MozaikParam::Drift))
    );
}
