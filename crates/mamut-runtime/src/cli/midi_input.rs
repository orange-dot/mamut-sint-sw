use super::*;

pub fn collect_midi_ports(midi_input: &MidiInput) -> Result<Vec<NamedMidiPort>> {
    let mut ports = Vec::new();
    for port in midi_input.ports() {
        let name = midi_input
            .port_name(&port)
            .unwrap_or_else(|_| "unknown-midi-port".to_string());
        ports.push(NamedMidiPort { port, name });
    }
    Ok(ports)
}

#[allow(clippy::too_many_arguments)]
pub fn open_midi_input(
    midi_input_queue: Arc<ArrayQueue<RealtimeMidiMessage>>,
    bend_range: f32,
    selector: Option<&str>,
    midi_channel: Option<u8>,
    controller_profile: Option<Arc<ControllerProfile>>,
    trace_midi: bool,
    runtime_control_queue: Arc<ArrayQueue<RuntimeControlMessage>>,
    priority_actions: Arc<PriorityActions>,
    input_metrics: Arc<InputMetrics>,
    midi_trace_log: Arc<MidiTraceLog>,
    midi_trace_publisher: MidiTracePublisher,
    sound_lab_midi_focus: Arc<SoundLabMidiFocus>,
) -> Result<Option<OpenedMidiConnection>> {
    let mut midi_input = match MidiInput::new("mamut-standalone") {
        Ok(midi_input) => midi_input,
        Err(error) => {
            return if selector.is_some() {
                Err(anyhow!("failed to create MIDI input: {error}"))
            } else {
                Ok(None)
            };
        }
    };
    midi_input.ignore(Ignore::None);
    let ports = collect_midi_ports(&midi_input)?;

    if ports.is_empty() {
        return if selector.is_some() {
            Err(anyhow!("no MIDI input devices available"))
        } else {
            Ok(None)
        };
    }

    let port_index = if let Some(selector) = selector {
        let names: Vec<String> = ports.iter().map(|entry| entry.name.clone()).collect();
        select_named_index(selector, &names, "MIDI input device")?
    } else {
        0
    };

    let selected = ports
        .into_iter()
        .nth(port_index)
        .ok_or_else(|| anyhow!("MIDI input device index {port_index} is out of range"))?;
    let port_name = selected.name.clone();
    let trace_started_at = Instant::now();
    let mut previous_trace_at: Option<Instant> = None;
    let startup_guard_until = trace_started_at
        .checked_add(MIDI_STARTUP_GUARD)
        .unwrap_or_else(Instant::now);
    let connection = midi_input
        .connect(
            &selected.port,
            "mamut-midi-in",
            move |_stamp, message, _| {
                let received_at = Instant::now();
                let parsed = parse_midi_message(
                    message,
                    bend_range,
                    midi_channel,
                    controller_profile.as_deref(),
                );
                let startup_suppressed =
                    startup_guard_suppresses_message(parsed, received_at, startup_guard_until);
                let overlay = if startup_suppressed || parsed.is_none() {
                    None
                } else {
                    sound_lab_midi_overlay_events(
                        message,
                        controller_profile.as_deref(),
                        sound_lab_midi_focus.page(),
                    )
                };
                let routed = if startup_suppressed { None } else { parsed };
                if trace_midi
                    || midi_trace_log.is_active()
                    || midi_message_can_update_last_control(
                        message,
                        midi_channel,
                        controller_profile.as_deref(),
                    )
                {
                    let trace_timing = MidiTraceTiming::from_received_at(
                        trace_started_at,
                        &mut previous_trace_at,
                        received_at,
                    );
                    midi_trace_publisher.publish(RawMidiTraceRecord::new(
                        message,
                        received_at,
                        trace_timing,
                        parsed,
                        overlay,
                        startup_suppressed,
                    ));
                }
                if let Some(overlay) = overlay {
                    input_metrics.record_midi_message();
                    if let Some(page) = overlay.page_select {
                        sound_lab_midi_focus.request_page(page);
                        input_metrics.record_midi_message_accepted();
                    }
                    for event in overlay.iter() {
                        publish_realtime_midi(
                            &midi_input_queue,
                            &input_metrics,
                            RealtimeMidiMessage::Controller(event),
                        );
                    }
                } else if let Some(parsed) = routed {
                    input_metrics.record_midi_message();
                    match parsed {
                        ParsedMidiMessage::Realtime(message) => {
                            publish_realtime_midi(&midi_input_queue, &input_metrics, message);
                        }
                        ParsedMidiMessage::Runtime(command) => {
                            publish_runtime_control(
                                &runtime_control_queue,
                                &priority_actions,
                                &input_metrics,
                                command,
                            );
                        }
                        ParsedMidiMessage::Reserved => {}
                    }
                }
            },
            (),
        )
        .map_err(|error| anyhow!("failed to open MIDI input connection: {error}"))?;

    Ok(Some(OpenedMidiConnection {
        port_name,
        _connection: connection,
    }))
}

pub fn midi_message_can_update_last_control(
    message: &[u8],
    midi_channel: Option<u8>,
    controller_profile: Option<&ControllerProfile>,
) -> bool {
    let Some(raw_status) = message.first().copied() else {
        return false;
    };
    let status = raw_status & 0xF0;
    let channel = (raw_status & 0x0F) + 1;
    if status < 0xF0 && midi_channel.is_some_and(|expected| channel != expected) {
        return true;
    }
    match status {
        0xB0 if message.len() >= 3 => {
            let cc = message[1];
            controller_profile
                .and_then(|profile| profile.binding_for_cc(cc))
                .is_some()
                || matches!(cc, 1 | 64 | 16 | 17 | 18 | 19 | 20)
        }
        0xC0 if message.len() >= 2 => true,
        0xD0 if message.len() >= 2 => true,
        0xE0 if message.len() >= 3 => true,
        _ => false,
    }
}

pub fn publish_realtime_midi(
    midi_input_queue: &ArrayQueue<RealtimeMidiMessage>,
    input_metrics: &InputMetrics,
    message: RealtimeMidiMessage,
) {
    if midi_input_queue.push(message).is_ok() {
        input_metrics.record_midi_message_accepted();
    } else {
        input_metrics.record_midi_message_dropped();
    }
}

pub fn publish_runtime_control(
    runtime_control_queue: &ArrayQueue<RuntimeControlMessage>,
    priority_actions: &PriorityActions,
    input_metrics: &InputMetrics,
    command: RuntimeControlMessage,
) {
    match command {
        RuntimeControlMessage::Panic => {
            priority_actions.request_panic();
            input_metrics.record_midi_message_accepted();
        }
        RuntimeControlMessage::ResetControllers => {
            priority_actions.request_reset_controllers();
            input_metrics.record_midi_message_accepted();
        }
        command => {
            if runtime_control_queue.push(command).is_ok() {
                input_metrics.record_midi_message_accepted();
            } else {
                input_metrics.record_runtime_control_dropped();
            }
        }
    }
}

pub fn startup_guard_suppresses_message(
    parsed: Option<ParsedMidiMessage>,
    received_at: Instant,
    guard_until: Instant,
) -> bool {
    if received_at >= guard_until {
        return false;
    }

    matches!(
        parsed,
        Some(ParsedMidiMessage::Realtime(_)) | Some(ParsedMidiMessage::Runtime(_))
    )
}

pub fn format_midi_trace_startup_suppressed(
    message: &[u8],
    midi_channel: Option<u8>,
    controller_profile: Option<&ControllerProfile>,
    parsed: Option<ParsedMidiMessage>,
    timing: MidiTraceTiming,
) -> String {
    format_midi_trace_message_with_prefix(
        "startup suppressed ",
        message,
        midi_channel,
        controller_profile,
        parsed,
        timing,
    )
}

pub fn format_midi_trace_message(
    message: &[u8],
    midi_channel: Option<u8>,
    controller_profile: Option<&ControllerProfile>,
    parsed: Option<ParsedMidiMessage>,
    timing: MidiTraceTiming,
) -> String {
    format_midi_trace_message_with_prefix(
        "",
        message,
        midi_channel,
        controller_profile,
        parsed,
        timing,
    )
}

pub fn format_midi_trace_message_with_prefix(
    verdict_prefix: &str,
    message: &[u8],
    midi_channel: Option<u8>,
    controller_profile: Option<&ControllerProfile>,
    parsed: Option<ParsedMidiMessage>,
    timing: MidiTraceTiming,
) -> String {
    let raw_status = *message.first().unwrap_or(&0);
    let status = raw_status & 0xF0;
    let channel = (raw_status & 0x0F) + 1;
    let raw = message
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ");

    let channel_filtered =
        status < 0xF0 && midi_channel.is_some_and(|expected| channel != expected);
    let verdict = if channel_filtered {
        format!(
            "filtered channel {channel} (expected channel {})",
            midi_channel.unwrap_or(channel)
        )
    } else if let Some(binding) = profile_binding_for_message(message, controller_profile) {
        describe_profile_cc_message(binding, message[2] as f32 / 127.0, parsed)
    } else if let Some(parsed) = parsed {
        describe_parsed_midi_message(parsed)
    } else {
        describe_unparsed_midi_message(message)
    };
    let verdict = format!("{verdict_prefix}{verdict}");

    if status < 0xF0 {
        format!(
            "midi trace: t={:.3}s dt={:.1}ms ch={channel} raw=[{raw}] {verdict}",
            timing.elapsed_seconds, timing.delta_millis
        )
    } else {
        format!(
            "midi trace: t={:.3}s dt={:.1}ms system raw=[{raw}] {verdict}",
            timing.elapsed_seconds, timing.delta_millis
        )
    }
}

pub fn last_control_event(
    message: &[u8],
    midi_channel: Option<u8>,
    controller_profile: Option<&ControllerProfile>,
    parsed: Option<ParsedMidiMessage>,
    received_at: Instant,
    startup_suppressed: bool,
) -> Option<LastControlEvent> {
    let raw_status = *message.first()?;
    let status = raw_status & 0xF0;
    let channel = (raw_status & 0x0F) + 1;
    if status < 0xF0 && midi_channel.is_some_and(|expected| channel != expected) {
        return Some(LastControlEvent {
            kind: LastControlKind::Other,
            label: format!("channel {channel}"),
            action: "filtered".to_string(),
            raw_status,
            channel: Some(channel),
            raw_value: message.get(2).map(|value| *value as f32 / 127.0),
            program: None,
            verdict: LastControlVerdict::Filtered,
            received_at,
        });
    }

    match status {
        0xB0 if message.len() >= 3 => {
            let cc = message[1];
            let raw_value = message[2] as f32 / 127.0;
            if let Some(binding) = controller_profile.and_then(|profile| profile.binding_for_cc(cc))
            {
                let verdict = if startup_suppressed {
                    LastControlVerdict::StartupSuppressed
                } else {
                    match (binding.action, parsed) {
                        (ControllerBindingAction::Reserved, _)
                        | (_, Some(ParsedMidiMessage::Reserved)) => LastControlVerdict::Reserved,
                        (ControllerBindingAction::Runtime(_), None)
                        | (ControllerBindingAction::ToggleParam(_), None) => {
                            LastControlVerdict::ReleaseIgnored
                        }
                        (_, Some(_)) => LastControlVerdict::Accepted,
                        (_, None) => LastControlVerdict::Ignored,
                    }
                };
                return Some(LastControlEvent {
                    kind: LastControlKind::ProfileCc(cc),
                    label: binding.control.clone(),
                    action: describe_binding_action(binding.action),
                    raw_status,
                    channel: Some(channel),
                    raw_value: Some(raw_value),
                    program: None,
                    verdict,
                    received_at,
                });
            }

            let (label, action) = match cc {
                1 => ("Mod Wheel".to_string(), "mod wheel".to_string()),
                64 => ("Sustain".to_string(), "sustain".to_string()),
                16 => ("Legacy CC16".to_string(), "macro Gravitacija".to_string()),
                17 => ("Legacy CC17".to_string(), "macro Bloom".to_string()),
                18 => ("Legacy CC18".to_string(), "macro Heat".to_string()),
                19 => ("Legacy CC19".to_string(), "macro Ruin".to_string()),
                20 => ("Legacy CC20".to_string(), "macro Swarm".to_string()),
                _ => return None,
            };
            Some(LastControlEvent {
                kind: LastControlKind::LegacyCc(cc),
                label,
                action,
                raw_status,
                channel: Some(channel),
                raw_value: Some(raw_value),
                program: None,
                verdict: if startup_suppressed {
                    LastControlVerdict::StartupSuppressed
                } else if parsed.is_some() {
                    LastControlVerdict::Accepted
                } else {
                    LastControlVerdict::Ignored
                },
                received_at,
            })
        }
        0xC0 if message.len() >= 2 => Some(LastControlEvent {
            kind: LastControlKind::ProgramChange(message[1]),
            label: "Program Change".to_string(),
            action: format!("live slot {}", message[1]),
            raw_status,
            channel: Some(channel),
            raw_value: None,
            program: Some(message[1]),
            verdict: if startup_suppressed {
                LastControlVerdict::StartupSuppressed
            } else if parsed.is_some() {
                LastControlVerdict::Accepted
            } else {
                LastControlVerdict::Ignored
            },
            received_at,
        }),
        0xD0 if message.len() >= 2 => Some(LastControlEvent {
            kind: LastControlKind::Other,
            label: "Channel Aftertouch".to_string(),
            action: "aftertouch".to_string(),
            raw_status,
            channel: Some(channel),
            raw_value: Some(message[1] as f32 / 127.0),
            program: None,
            verdict: if startup_suppressed {
                LastControlVerdict::StartupSuppressed
            } else {
                LastControlVerdict::Accepted
            },
            received_at,
        }),
        0xE0 if message.len() >= 3 => Some(LastControlEvent {
            kind: LastControlKind::Other,
            label: "Pitch Bend".to_string(),
            action: "pitch bend".to_string(),
            raw_status,
            channel: Some(channel),
            raw_value: None,
            program: None,
            verdict: if startup_suppressed {
                LastControlVerdict::StartupSuppressed
            } else {
                LastControlVerdict::Accepted
            },
            received_at,
        }),
        _ => None,
    }
}
