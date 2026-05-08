use super::*;

#[test]
fn runtime_panic_bypasses_full_runtime_control_queue() {
    let queue = ArrayQueue::new(1);
    let priority = PriorityActions::default();
    let metrics = InputMetrics::default();
    queue
        .push(RuntimeControlMessage::NextFavorite)
        .expect("queue accepts first command");

    publish_runtime_control(&queue, &priority, &metrics, RuntimeControlMessage::Panic);

    assert!(priority.panic_requested.load(Ordering::Relaxed));
    assert_eq!(metrics.snapshot().midi_messages_accepted, 1);
    assert_eq!(metrics.snapshot().runtime_controls_dropped, 0);
    assert_eq!(queue.len(), 1);
}

#[test]
fn non_priority_runtime_control_drop_is_counted() {
    let queue = ArrayQueue::new(1);
    let priority = PriorityActions::default();
    let metrics = InputMetrics::default();
    queue
        .push(RuntimeControlMessage::NextFavorite)
        .expect("queue accepts first command");

    publish_runtime_control(
        &queue,
        &priority,
        &metrics,
        RuntimeControlMessage::PrevFavorite,
    );

    assert!(!priority.panic_requested.load(Ordering::Relaxed));
    assert_eq!(metrics.snapshot().midi_messages_accepted, 0);
    assert_eq!(metrics.snapshot().runtime_controls_dropped, 1);
}

#[test]
fn controller_coalescing_keeps_fifo_edge_controls() {
    let mut events = vec![
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::ModWheel { amount: 0.1 },
        },
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::Sustain { down: true },
        },
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::ModWheel { amount: 0.9 },
        },
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::BcsLayerEnabled { enabled: true },
        },
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::BcsLayerEnabled { enabled: false },
        },
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::DirectParam {
                id: ParamId::Osc1SawLevel,
                value: 0.2,
            },
        },
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::DirectParam {
                id: ParamId::Osc1SawLevel,
                value: 0.7,
            },
        },
    ];

    let coalesced = coalesce_controller_events(&mut events);

    assert_eq!(coalesced, 2);
    assert_eq!(events.len(), 5);
    assert_eq!(events[0].event, ControllerEvent::ModWheel { amount: 0.9 });
    assert_eq!(events[1].event, ControllerEvent::Sustain { down: true });
    assert_eq!(
        events[2].event,
        ControllerEvent::BcsLayerEnabled { enabled: true }
    );
    assert_eq!(
        events[3].event,
        ControllerEvent::BcsLayerEnabled { enabled: false }
    );
    assert_eq!(
        events[4].event,
        ControllerEvent::DirectParam {
            id: ParamId::Osc1SawLevel,
            value: 0.7,
        }
    );
}

#[test]
fn realtime_midi_drain_keeps_preallocated_capacity() {
    let state = test_engine_thread_state();
    let initial_capacity = state.controller_events.capacity();
    for index in 0..MIDI_INPUT_QUEUE_CAPACITY {
        state
            .midi_input_queue
            .push(RealtimeMidiMessage::Controller(ControllerEvent::ModWheel {
                amount: index as f32 / MIDI_INPUT_QUEUE_CAPACITY as f32,
            }))
            .expect("queue accepts synthetic controller event");
    }
    let mut state = state;
    state.drain_realtime_midi_nonblocking();

    assert_eq!(initial_capacity, MIDI_INPUT_QUEUE_CAPACITY);
    assert_eq!(state.controller_events.capacity(), initial_capacity);
    assert_eq!(state.controller_events.len(), MIDI_INPUT_QUEUE_CAPACITY);
}

#[test]
fn system_realtime_messages_do_not_enter_runtime_queue() {
    assert!(parse_midi_message(&[0xF8], 2.0, None, None).is_none());
    assert!(parse_midi_message(&[0xFE], 2.0, None, None).is_none());
    assert!(!midi_message_can_update_last_control(&[0xF8], None, None));
    assert!(!midi_message_can_update_last_control(&[0xFE], None, None));
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
fn scope_publish_is_disabled_until_gui_enables_it() {
    let mut state = test_engine_thread_state();
    let note_events = [Scheduled {
        frame_offset: 0,
        event: NoteEvent::NoteOn {
            note: 60,
            velocity: 0.8,
        },
    }];
    state.note_events.extend(note_events);
    state.render_audio_block();
    assert_eq!(state.scope_producer.slots(), SCOPE_QUEUE_CAPACITY_FRAMES);

    state.scope_enabled.store(true, Ordering::Relaxed);
    state.note_events.extend(note_events);
    state.render_audio_block();
    assert!(state.scope_producer.slots() < SCOPE_QUEUE_CAPACITY_FRAMES);
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
