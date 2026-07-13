use super::*;

pub fn profile_binding_for_message<'a>(
    message: &[u8],
    controller_profile: Option<&'a ControllerProfile>,
) -> Option<&'a ControllerBinding> {
    let raw_status = *message.first()?;
    let status = raw_status & 0xF0;
    if status == 0xB0 && message.len() >= 3 {
        controller_profile.and_then(|profile| profile.binding_for_cc(message[1]))
    } else {
        None
    }
}

pub fn describe_profile_cc_message(
    binding: &ControllerBinding,
    value: f32,
    parsed: Option<ParsedMidiMessage>,
) -> String {
    match (binding.action, parsed) {
        (ControllerBindingAction::Reserved, _) | (_, Some(ParsedMidiMessage::Reserved)) => {
            format!("profile {} reserved value={value:.3}", binding.control)
        }
        (ControllerBindingAction::Runtime(_), None)
        | (ControllerBindingAction::ToggleParam(_), None) => {
            format!(
                "profile {} {} release ignored value={value:.3}",
                binding.control,
                describe_binding_action(binding.action)
            )
        }
        (_, Some(parsed)) => format!(
            "profile {} {} value={value:.3} -> {}",
            binding.control,
            describe_binding_action(binding.action),
            describe_parsed_midi_message(parsed)
        ),
        (_, None) => format!(
            "profile {} {} value={value:.3}",
            binding.control,
            describe_binding_action(binding.action)
        ),
    }
}

pub fn describe_binding_action(action: ControllerBindingAction) -> String {
    match action {
        ControllerBindingAction::Macro(id) => format!("macro {}", macro_display_name(id)),
        ControllerBindingAction::DirectParam { id, .. } => {
            format!("direct param {}", param_spec(id).name)
        }
        ControllerBindingAction::GfmLayerAmount => "gfm layer gate".to_string(),
        ControllerBindingAction::BcsLayerAmount => "bcs layer gain".to_string(),
        ControllerBindingAction::BcsLayerEnabled => "bcs layer enable".to_string(),
        ControllerBindingAction::MozaikControl(param) => {
            format!("mozaik {}", mozaik_param_name(param))
        }
        ControllerBindingAction::Runtime(message) => describe_runtime_control_message(message),
        ControllerBindingAction::ToggleParam(id) => format!("toggle {}", param_spec(id).name),
        ControllerBindingAction::Reserved => "reserved".to_string(),
    }
}

pub fn describe_runtime_control_message(message: RuntimeControlMessage) -> String {
    match message {
        RuntimeControlMessage::ProgramChange(slot) => format!("program change slot={slot}"),
        RuntimeControlMessage::Panic => "panic".to_string(),
        RuntimeControlMessage::ResetControllers => "reset controllers".to_string(),
        RuntimeControlMessage::NextFavorite => "next favorite".to_string(),
        RuntimeControlMessage::PrevFavorite => "previous favorite".to_string(),
        RuntimeControlMessage::FavoriteSlot(slot) => format!("favorite slot={slot}"),
        RuntimeControlMessage::ToggleParam(id) => format!("toggle {}", param_spec(id).name),
        RuntimeControlMessage::MozaikControl(param, value) => {
            format!("mozaik {} value={value:.3}", mozaik_param_name(param))
        }
    }
}

/// Short display name for a Mozaik session control, shared by trace and mirror text.
pub fn mozaik_param_name(param: MozaikParam) -> &'static str {
    match param {
        MozaikParam::Mix => "mix",
        MozaikParam::Slope => "slope",
        MozaikParam::Contrast => "contrast",
        MozaikParam::Phason => "phason",
        MozaikParam::Drift => "drift",
    }
}

pub fn describe_parsed_midi_message(parsed: ParsedMidiMessage) -> String {
    match parsed {
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Note(NoteEvent::NoteOn {
            note,
            velocity,
        })) => format!("note on note={note} velocity={velocity:.3}"),
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Note(NoteEvent::NoteOff { note })) => {
            format!("note off note={note}")
        }
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
            ControllerEvent::ModWheel { amount },
        )) => format!("mod wheel amount={amount:.3}"),
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
            ControllerEvent::Sustain { down },
        )) => format!("sustain down={down}"),
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
            ControllerEvent::ChannelAftertouch { pressure },
        )) => format!("aftertouch pressure={pressure:.3}"),
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
            ControllerEvent::PitchBend { semitones },
        )) => format!("pitch bend semitones={semitones:.3}"),
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(ControllerEvent::Macro {
            id,
            value,
        })) => format!("macro {} value={value:.3}", macro_display_name(id)),
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
            ControllerEvent::GfmLayerAmount { amount },
        )) => format!("gfm layer gate={amount:.3}"),
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
            ControllerEvent::BcsLayerAmount { amount },
        )) => format!("bcs layer gain={amount:.3}"),
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
            ControllerEvent::BcsLayerEnabled { enabled },
        )) => format!("bcs layer enable={enabled}"),
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
            ControllerEvent::DirectParam { id, value },
        )) => format!("direct param {} value={value:.3}", param_spec(id).name),
        ParsedMidiMessage::Runtime(message) => describe_runtime_control_message(message),
        ParsedMidiMessage::Reserved => "profile reserved".to_string(),
    }
}

pub fn describe_unparsed_midi_message(message: &[u8]) -> String {
    let Some(raw_status) = message.first().copied() else {
        return "ignored empty message".to_string();
    };
    let status = raw_status & 0xF0;
    match status {
        0xA0 if message.len() >= 3 => {
            format!(
                "ignored poly aftertouch note={} value={}",
                message[1], message[2]
            )
        }
        0xB0 if message.len() >= 3 => format!("ignored cc={} value={}", message[1], message[2]),
        0xF0 => "ignored system/common message".to_string(),
        _ => "ignored unsupported message".to_string(),
    }
}

pub fn select_named_index(selector: &str, names: &[String], kind: &str) -> Result<usize> {
    if let Ok(index) = selector.parse::<usize>() {
        return if index < names.len() {
            Ok(index)
        } else {
            Err(anyhow!(
                "{kind} index {index} is out of range; available count is {}",
                names.len()
            ))
        };
    }

    if let Some((index, _)) = names
        .iter()
        .enumerate()
        .find(|(_, name)| name.eq_ignore_ascii_case(selector))
    {
        return Ok(index);
    }

    let selector_lower = selector.to_ascii_lowercase();
    let matches: Vec<usize> = names
        .iter()
        .enumerate()
        .filter_map(|(index, name)| {
            name.to_ascii_lowercase()
                .contains(&selector_lower)
                .then_some(index)
        })
        .collect();

    match matches.as_slice() {
        [index] => Ok(*index),
        [] => Err(anyhow!(
            "no {kind} matched `{selector}`; run the relevant list command to inspect available devices"
        )),
        _ => Err(anyhow!(
            "selector `{selector}` is ambiguous for {kind}; use a numeric index or a more specific name"
        )),
    }
}
