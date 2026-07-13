use super::param_values::{
    direct_param_display_value, direct_param_raw_value, macro_snapshot_value,
    toggle_param_display_value,
};
use super::*;

pub(crate) fn pc4_control_display(
    snapshot: &EngineSnapshot,
    binding: &ControllerBinding,
) -> Pc4ControlDisplay {
    let (value, secondary) = binding_display_value(snapshot, binding);
    let normalized = binding_normalized_value(snapshot, binding);
    Pc4ControlDisplay {
        normalized,
        value: secondary
            .map(|secondary| format!("{value} {secondary}"))
            .unwrap_or(value),
        short_action: short_pc4_action(binding.action),
    }
}

pub(crate) fn binding_normalized_value(
    snapshot: &EngineSnapshot,
    binding: &ControllerBinding,
) -> f32 {
    match binding.action {
        ControllerBindingAction::Macro(id) => macro_snapshot_value(snapshot, id, false),
        ControllerBindingAction::DirectParam { id, .. } => direct_param_raw_value(snapshot, id)
            .map(|value| normalize_param_value(id, value))
            .unwrap_or(0.0),
        ControllerBindingAction::GfmLayerAmount => snapshot.gfm_layer.pressure,
        ControllerBindingAction::BcsLayerAmount => snapshot.bcs_layer.amount,
        ControllerBindingAction::BcsLayerEnabled => {
            if snapshot.bcs_layer.enabled {
                1.0
            } else {
                0.0
            }
        }
        ControllerBindingAction::ToggleParam(id) => match id {
            ParamId::ChorusEnabled => {
                if snapshot.direct.chorus_enabled {
                    1.0
                } else {
                    0.0
                }
            }
            ParamId::ReverbEnabled => {
                if snapshot.direct.reverb_enabled {
                    1.0
                } else {
                    0.0
                }
            }
            _ => 0.0,
        },
        ControllerBindingAction::MozaikControl(param) => mozaik_control_value(snapshot, param),
        ControllerBindingAction::Runtime(_) | ControllerBindingAction::Reserved => 0.0,
    }
    .clamp(0.0, 1.0)
}

/// Read-only mirror: the control-domain value of a Mozaik session control.
pub(crate) fn mozaik_control_value(snapshot: &EngineSnapshot, param: MozaikParam) -> f32 {
    match param {
        MozaikParam::Mix => snapshot.mozaik.mix,
        MozaikParam::Slope => snapshot.mozaik.slope,
        MozaikParam::Contrast => snapshot.mozaik.contrast,
        MozaikParam::Phason => snapshot.mozaik.phason,
        MozaikParam::Drift => snapshot.mozaik.drift,
    }
}

pub(crate) fn normalize_param_value(id: ParamId, value: f32) -> f32 {
    let spec = param_spec(id);
    if matches!(spec.unit, ParamUnit::Hertz | ParamUnit::Milliseconds)
        && spec.min > 0.0
        && spec.max > spec.min
        && value > 0.0
    {
        let min = spec.min.ln();
        let max = spec.max.ln();
        return ((value.ln() - min) / (max - min)).clamp(0.0, 1.0);
    }
    if spec.max <= spec.min {
        0.0
    } else {
        ((value - spec.min) / (spec.max - spec.min)).clamp(0.0, 1.0)
    }
}

pub(crate) fn short_pc4_action(action: ControllerBindingAction) -> String {
    match action {
        ControllerBindingAction::Macro(id) => macro_display_name(id).to_string(),
        ControllerBindingAction::DirectParam { id, .. } => param_spec(id).name.to_string(),
        ControllerBindingAction::GfmLayerAmount => "GFM Gate".to_string(),
        ControllerBindingAction::BcsLayerAmount => "BCS Gain".to_string(),
        ControllerBindingAction::BcsLayerEnabled => "BCS Enable".to_string(),
        ControllerBindingAction::MozaikControl(param) => {
            format!("Mozaik {}", mozaik_param_name(param))
        }
        ControllerBindingAction::Runtime(message) => describe_runtime_control_message(message),
        ControllerBindingAction::ToggleParam(id) => param_spec(id).name.to_string(),
        ControllerBindingAction::Reserved => "Reserved".to_string(),
    }
}

pub(crate) fn draw_pc4_control_shell(painter: &egui::Painter, rect: egui::Rect, highlighted: bool) {
    painter.rect_filled(
        rect,
        egui::CornerRadius::same(4),
        if highlighted {
            epm_tile_hot()
        } else {
            epm_tile()
        },
    );
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(4),
        if highlighted {
            epm_accent_stroke()
        } else {
            epm_stroke()
        },
        egui::StrokeKind::Inside,
    );
}

pub(crate) fn draw_centered_text(
    painter: &egui::Painter,
    rect: egui::Rect,
    top_offset: f32,
    text: &str,
    size: f32,
    color: egui::Color32,
) {
    let clipped = clipped_label(text, (rect.width() / (size * 0.58)).floor() as usize);
    painter.text(
        egui::pos2(rect.center().x, rect.top() + top_offset),
        egui::Align2::CENTER_TOP,
        clipped,
        egui::FontId::new(size, egui::FontFamily::Proportional),
        color,
    );
}

pub(crate) fn clipped_label(text: &str, max_chars: usize) -> String {
    if max_chars == 0 || text.chars().count() <= max_chars {
        return text.to_string();
    }
    let keep = max_chars.saturating_sub(1);
    let mut output = text.chars().take(keep).collect::<String>();
    output.push('~');
    output
}

pub(crate) fn draw_arc(
    painter: &egui::Painter,
    center: egui::Pos2,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
    stroke: egui::Stroke,
) {
    let steps = 24;
    let sweep = end_angle - start_angle;
    if sweep.abs() <= f32::EPSILON {
        return;
    }
    let points = (0..=steps)
        .map(|step| {
            let t = step as f32 / steps as f32;
            let angle = start_angle + sweep * t;
            egui::pos2(
                center.x + angle.cos() * radius,
                center.y + angle.sin() * radius,
            )
        })
        .collect::<Vec<_>>();
    painter.add(egui::Shape::line(points, stroke));
}

pub(crate) fn binding_display_value(
    snapshot: &EngineSnapshot,
    binding: &ControllerBinding,
) -> (String, Option<String>) {
    match binding.action {
        ControllerBindingAction::Macro(id) => {
            let live = macro_snapshot_value(snapshot, id, false);
            let effective = macro_snapshot_value(snapshot, id, true);
            (format!("{live:.2}"), Some(format!("eff {effective:.2}")))
        }
        ControllerBindingAction::DirectParam { id, .. } => (
            direct_param_display_value(snapshot, id).unwrap_or_else(|| "not exposed".to_string()),
            None,
        ),
        ControllerBindingAction::GfmLayerAmount => {
            (format!("{:.2}", snapshot.gfm_layer.pressure), None)
        }
        ControllerBindingAction::BcsLayerAmount => (
            format!("{:.2}", snapshot.bcs_layer.gain),
            Some(format!("eff {:.2}", snapshot.bcs_layer.effective_gain)),
        ),
        ControllerBindingAction::BcsLayerEnabled => (
            if snapshot.bcs_layer.enabled {
                "on".to_string()
            } else {
                "off".to_string()
            },
            None,
        ),
        ControllerBindingAction::MozaikControl(param) => {
            let value = format!("{:.2}", mozaik_control_value(snapshot, param));
            let secondary = matches!(param, MozaikParam::Mix)
                .then(|| format!("eff {:.2}", snapshot.mozaik.effective_mix));
            (value, secondary)
        }
        ControllerBindingAction::Runtime(_) => ("trigger".to_string(), None),
        ControllerBindingAction::ToggleParam(id) => {
            (toggle_param_display_value(snapshot, id), None)
        }
        ControllerBindingAction::Reserved => ("Reserved".to_string(), None),
    }
}
