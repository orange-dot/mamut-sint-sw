use super::param_values::{direct_param_raw_value, format_param_value};
use super::*;

pub(crate) struct Pc4ControlDisplay {
    pub(crate) normalized: f32,
    pub(crate) value: String,
    pub(crate) short_action: String,
}

pub(crate) struct SoundLabControlDisplay {
    pub(crate) normalized: f32,
    pub(crate) value: String,
    pub(crate) short_action: String,
}

pub(crate) fn sound_lab_control_display(
    snapshot: &EngineSnapshot,
    id: ParamId,
) -> Option<SoundLabControlDisplay> {
    let value = direct_param_raw_value(snapshot, id)?;
    Some(SoundLabControlDisplay {
        normalized: normalize_param_value(id, value),
        value: format_param_value(id, value),
        short_action: param_spec(id).name.to_string(),
    })
}

pub(crate) fn sound_lab_param_value_from_normalized(id: ParamId, normalized: f32) -> f32 {
    sound_lab_direct_param_value(id, normalized)
}

pub(crate) fn sound_lab_active_source(
    last_control: Option<&LastControlEvent>,
    controller_profile: Option<&ControllerProfile>,
) -> Option<SoundLabMidiSource> {
    let event = last_control.filter(|event| event_is_recent(event))?;
    match event.kind {
        LastControlKind::ProfileCc(cc) | LastControlKind::LegacyCc(cc) => {
            sound_lab_source_for_cc(cc, controller_profile)
        }
        LastControlKind::ProgramChange(_) | LastControlKind::Other => None,
    }
}

pub(crate) fn sound_lab_source_for_cc(
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
    sound_lab_source_for_controller_binding(binding)
}

pub(crate) fn sound_lab_source_for_controller_binding(
    binding: &ControllerBinding,
) -> Option<SoundLabMidiSource> {
    if let (ControllerBindingSection::Switch, Some(index)) = (binding.section, binding.index) {
        return Some(SoundLabMidiSource::Switch(index));
    }

    if matches!(
        binding.action,
        ControllerBindingAction::GfmLayerAmount
            | ControllerBindingAction::BcsLayerAmount
            | ControllerBindingAction::BcsLayerEnabled
    ) {
        return None;
    }

    match (binding.section, binding.index) {
        (ControllerBindingSection::Knob, Some(index)) => Some(SoundLabMidiSource::Knob(index)),
        (ControllerBindingSection::Slider, Some(index)) => Some(SoundLabMidiSource::Slider(index)),
        _ => None,
    }
}

pub(crate) fn render_sound_lab_disabled_control(
    ui: &mut egui::Ui,
    binding: SoundLabMidiParamBinding,
    size: egui::Vec2,
) {
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    draw_pc4_control_shell(&painter, rect, false);
    draw_centered_text(
        &painter,
        rect,
        10.0,
        &binding.source.badge(),
        11.0,
        epm_orange(),
    );
    draw_centered_text(
        &painter,
        rect,
        rect.height() * 0.50,
        param_spec(binding.id).name,
        10.0,
        epm_cyan(),
    );
    draw_centered_text(
        &painter,
        rect,
        rect.height() * 0.50 + 16.0,
        "not exposed",
        10.0,
        epm_muted(),
    );
}
