use super::*;

pub fn parse_midi_message(
    message: &[u8],
    bend_range: f32,
    midi_channel: Option<u8>,
    controller_profile: Option<&ControllerProfile>,
) -> Option<ParsedMidiMessage> {
    let raw_status = *message.first()?;
    let status = raw_status & 0xF0;
    let channel = (raw_status & 0x0F) + 1;
    if status < 0xF0 && midi_channel.is_some_and(|expected| channel != expected) {
        return None;
    }
    match status {
        0x80 if message.len() >= 2 => Some(ParsedMidiMessage::Realtime(RealtimeMidiMessage::Note(
            NoteEvent::NoteOff { note: message[1] },
        ))),
        0x90 if message.len() >= 3 => {
            if message[2] == 0 {
                Some(ParsedMidiMessage::Realtime(RealtimeMidiMessage::Note(
                    NoteEvent::NoteOff { note: message[1] },
                )))
            } else {
                Some(ParsedMidiMessage::Realtime(RealtimeMidiMessage::Note(
                    NoteEvent::NoteOn {
                        note: message[1],
                        velocity: message[2] as f32 / 127.0,
                    },
                )))
            }
        }
        0xB0 if message.len() >= 3 => {
            let cc = message[1];
            let value = message[2] as f32 / 127.0;
            if let Some(binding) = controller_profile.and_then(|profile| profile.binding_for_cc(cc))
            {
                return parse_profile_cc_binding(binding, value);
            }
            let event = match cc {
                1 => ControllerEvent::ModWheel { amount: value },
                64 => ControllerEvent::Sustain { down: value >= 0.5 },
                16 => ControllerEvent::Macro {
                    id: MacroId::Gravitacija,
                    value,
                },
                17 => ControllerEvent::Macro {
                    id: MacroId::Bloom,
                    value,
                },
                18 => ControllerEvent::Macro {
                    id: MacroId::Heat,
                    value,
                },
                19 => ControllerEvent::Macro {
                    id: MacroId::Ruin,
                    value,
                },
                20 => ControllerEvent::Macro {
                    id: MacroId::Swarm,
                    value,
                },
                _ => return None,
            };
            Some(ParsedMidiMessage::Realtime(
                RealtimeMidiMessage::Controller(event),
            ))
        }
        0xC0 if message.len() >= 2 => Some(ParsedMidiMessage::Runtime(
            RuntimeControlMessage::ProgramChange(message[1]),
        )),
        0xD0 if message.len() >= 2 => Some(ParsedMidiMessage::Realtime(
            RealtimeMidiMessage::Controller(ControllerEvent::ChannelAftertouch {
                pressure: message[1] as f32 / 127.0,
            }),
        )),
        0xE0 if message.len() >= 3 => {
            let value = u16::from(message[1]) | (u16::from(message[2]) << 7);
            let normalized = (value as f32 - 8_192.0) / 8_192.0;
            Some(ParsedMidiMessage::Realtime(
                RealtimeMidiMessage::Controller(ControllerEvent::PitchBend {
                    semitones: normalized.clamp(-1.0, 1.0) * bend_range,
                }),
            ))
        }
        _ => None,
    }
}

pub fn parse_profile_cc_binding(
    binding: &ControllerBinding,
    value: f32,
) -> Option<ParsedMidiMessage> {
    match binding.action {
        ControllerBindingAction::Macro(id) => Some(ParsedMidiMessage::Realtime(
            RealtimeMidiMessage::Controller(ControllerEvent::Macro { id, value }),
        )),
        ControllerBindingAction::DirectParam { id, scale } => Some(ParsedMidiMessage::Realtime(
            RealtimeMidiMessage::Controller(ControllerEvent::DirectParam {
                id,
                value: scale_controller_value(id, value, scale),
            }),
        )),
        ControllerBindingAction::GfmLayerAmount => Some(ParsedMidiMessage::Realtime(
            RealtimeMidiMessage::Controller(ControllerEvent::GfmLayerAmount { amount: value }),
        )),
        ControllerBindingAction::BcsLayerAmount => Some(ParsedMidiMessage::Realtime(
            RealtimeMidiMessage::Controller(ControllerEvent::BcsLayerAmount { amount: value }),
        )),
        ControllerBindingAction::BcsLayerEnabled => Some(ParsedMidiMessage::Realtime(
            RealtimeMidiMessage::Controller(ControllerEvent::BcsLayerEnabled {
                enabled: value >= 0.5,
            }),
        )),
        // Route to the bounded runtime-control queue (not a realtime ControllerEvent):
        // the session drains it into the same `set_mozaik_param` path the headless
        // `mozaik set` uses. `value` is already the linear `0..127 -> 0.0..1.0` map.
        ControllerBindingAction::MozaikControl(param) => Some(ParsedMidiMessage::Runtime(
            RuntimeControlMessage::MozaikControl(param, value),
        )),
        ControllerBindingAction::Runtime(message) => {
            (value >= 0.5).then_some(ParsedMidiMessage::Runtime(message))
        }
        ControllerBindingAction::ToggleParam(id) => (value >= 0.5).then_some(
            ParsedMidiMessage::Runtime(RuntimeControlMessage::ToggleParam(id)),
        ),
        ControllerBindingAction::Reserved => Some(ParsedMidiMessage::Reserved),
    }
}

pub fn sound_lab_midi_overlay_events(
    message: &[u8],
    controller_profile: Option<&ControllerProfile>,
    page: Option<SoundLabPage>,
) -> Option<SoundLabOverlayEvents> {
    let page = page?;
    let raw_status = *message.first()?;
    if raw_status & 0xF0 != 0xB0 || message.len() < 3 {
        return None;
    }

    let cc = message[1];
    let normalized = message[2] as f32 / 127.0;
    if normalized >= 0.5
        && let Some((switch_index, selected_page)) =
            sound_lab_page_switch_for_cc(cc, controller_profile)
    {
        return Some(SoundLabOverlayEvents::select_page(
            page,
            switch_index,
            selected_page,
        ));
    }

    let source = sound_lab_overlay_source_for_cc(cc, controller_profile)?;
    match source {
        SoundLabMidiSource::ModWheel => sound_lab_mod_wheel_events(page, normalized),
        SoundLabMidiSource::Knob(_) | SoundLabMidiSource::Slider(_) => {
            let binding = sound_lab_page_binding_for_source(page, source)?;
            Some(SoundLabOverlayEvents::single(
                page,
                source,
                direct_param_overlay_event(binding.id, normalized),
            ))
        }
        SoundLabMidiSource::Switch(_) => None,
    }
}

fn sound_lab_page_switch_for_cc(
    cc: u8,
    controller_profile: Option<&ControllerProfile>,
) -> Option<(u8, SoundLabPage)> {
    let binding = controller_profile.and_then(|profile| profile.binding_for_cc(cc))?;
    let (ControllerBindingSection::Switch, Some(index)) = (binding.section, binding.index) else {
        return None;
    };
    let page = SoundLabPage::from_index(index.checked_sub(1)?)?;
    Some((index, page))
}

fn sound_lab_overlay_source_for_cc(
    cc: u8,
    controller_profile: Option<&ControllerProfile>,
) -> Option<SoundLabMidiSource> {
    if cc == 1 {
        return Some(SoundLabMidiSource::ModWheel);
    }
    if cc == 64 {
        return None;
    }

    let binding = controller_profile.and_then(|profile| profile.binding_for_cc(cc))?;
    if matches!(
        binding.action,
        ControllerBindingAction::GfmLayerAmount
            | ControllerBindingAction::BcsLayerAmount
            | ControllerBindingAction::BcsLayerEnabled
            | ControllerBindingAction::MozaikControl(_)
    ) {
        return None;
    }

    match (binding.section, binding.index) {
        (ControllerBindingSection::Knob, Some(index)) => Some(SoundLabMidiSource::Knob(index)),
        (ControllerBindingSection::Slider, Some(index)) => Some(SoundLabMidiSource::Slider(index)),
        _ => None,
    }
}

fn sound_lab_mod_wheel_events(page: SoundLabPage, value: f32) -> Option<SoundLabOverlayEvents> {
    let params = sound_lab_page_mod_wheel_params(page);
    if params.is_empty() {
        return None;
    }

    let mut events = SoundLabOverlayEvents::empty(page, SoundLabMidiSource::ModWheel);
    for (slot, id) in params
        .iter()
        .copied()
        .take(SOUND_LAB_OVERLAY_EVENT_CAPACITY)
        .enumerate()
    {
        events.events[slot] = Some(direct_param_overlay_event(id, value));
    }
    Some(events)
}

fn direct_param_overlay_event(id: ParamId, normalized: f32) -> ControllerEvent {
    ControllerEvent::DirectParam {
        id,
        value: sound_lab_direct_param_value(id, normalized),
    }
}

pub fn sound_lab_direct_param_value(id: ParamId, normalized: f32) -> f32 {
    let spec = param_spec(id);
    let value = scale_controller_value(id, normalized, default_scale_for_param(id));
    match spec.unit {
        ParamUnit::Boolean => {
            if value >= 0.5 {
                1.0
            } else {
                0.0
            }
        }
        ParamUnit::Indexed | ParamUnit::Semitones => value.round().clamp(spec.min, spec.max),
        _ => value.clamp(spec.min, spec.max),
    }
}
