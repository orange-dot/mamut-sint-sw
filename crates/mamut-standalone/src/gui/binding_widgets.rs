use super::*;

pub(crate) fn sorted_bindings_for_section(
    profile: &ControllerProfile,
    section: ControllerBindingSection,
) -> Vec<&ControllerBinding> {
    let mut bindings = profile
        .bindings_by_cc
        .values()
        .filter(|binding| binding.section == section)
        .collect::<Vec<_>>();
    bindings.sort_by_key(|binding| {
        (
            binding.index.unwrap_or(u8::MAX),
            binding.control.clone(),
            binding.cc,
        )
    });
    bindings
}

pub(crate) fn render_binding_tile(
    ui: &mut egui::Ui,
    binding: &ControllerBinding,
    snapshot: &EngineSnapshot,
    last_control: Option<&LastControlEvent>,
) {
    let highlighted = last_control.is_some_and(|event| event_matches_binding(event, binding));
    let (value, secondary) = binding_display_value(snapshot, binding);
    let value = secondary
        .map(|secondary| format!("{value}  {secondary}"))
        .unwrap_or(value);
    render_small_status_tile(
        ui,
        highlighted,
        &binding.control,
        &format!("CC{}", binding.cc),
        &describe_binding_action(binding.action),
        &value,
    );
}

pub(crate) fn render_small_status_tile(
    ui: &mut egui::Ui,
    highlighted: bool,
    title: &str,
    subtitle: &str,
    action: &str,
    value: &str,
) {
    epm_tile_frame(highlighted).show(ui, |ui| {
        ui.set_min_size(egui::vec2(170.0, 88.0));
        ui.label(epm_eyebrow(title.to_ascii_uppercase()));
        ui.label(epm_small(subtitle));
        ui.label(epm_body(action));
        if !value.is_empty() {
            ui.label(epm_value(value));
        }
    });
}

pub(crate) fn render_pc4_knob(
    ui: &mut egui::Ui,
    binding: &ControllerBinding,
    snapshot: &EngineSnapshot,
    last_control: Option<&LastControlEvent>,
    size: egui::Vec2,
) {
    let display = pc4_control_display(snapshot, binding);
    let highlighted = last_control.is_some_and(|event| event_matches_binding(event, binding));
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    draw_pc4_control_shell(&painter, rect, highlighted);

    let center = egui::pos2(rect.center().x, rect.top() + 48.0);
    let radius = (rect.width().min(rect.height()) * 0.26).clamp(22.0, 34.0);
    painter.circle_stroke(center, radius, egui::Stroke::new(5.0, epm_stroke().color));
    let marker_angle = pc4_knob_angle(display.normalized);
    draw_arc(
        &painter,
        center,
        radius,
        PC4_KNOB_START_ANGLE,
        marker_angle,
        egui::Stroke::new(5.0, if highlighted { epm_ok() } else { epm_orange() }),
    );
    let marker = egui::pos2(
        center.x + marker_angle.cos() * (radius - 5.0),
        center.y + marker_angle.sin() * (radius - 5.0),
    );
    painter.line_segment(
        [center, marker],
        egui::Stroke::new(2.0, if highlighted { epm_ok() } else { epm_text() }),
    );

    draw_centered_text(&painter, rect, 10.0, &binding.control, 11.0, epm_orange());
    draw_centered_text(
        &painter,
        rect,
        82.0,
        &format!("CC{}", binding.cc),
        10.0,
        epm_muted(),
    );
    draw_centered_text(
        &painter,
        rect,
        98.0,
        &display.short_action,
        10.0,
        epm_cyan(),
    );
    draw_centered_text(&painter, rect, 112.0, &display.value, 13.0, epm_text());
}

pub(crate) fn pc4_knob_angle(normalized: f32) -> f32 {
    PC4_KNOB_START_ANGLE + normalized.clamp(0.0, 1.0) * PC4_KNOB_SWEEP_ANGLE
}

pub(crate) fn render_pc4_slider(
    ui: &mut egui::Ui,
    binding: &ControllerBinding,
    snapshot: &EngineSnapshot,
    last_control: Option<&LastControlEvent>,
    size: egui::Vec2,
) {
    let display = pc4_control_display(snapshot, binding);
    let highlighted = last_control.is_some_and(|event| event_matches_binding(event, binding));
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    draw_pc4_control_shell(&painter, rect, highlighted);

    let track_top = rect.top() + 36.0;
    let track_bottom = rect.bottom() - 48.0;
    let track_x = rect.center().x;
    painter.line_segment(
        [
            egui::pos2(track_x, track_top),
            egui::pos2(track_x, track_bottom),
        ],
        egui::Stroke::new(8.0, epm_stroke().color),
    );
    let handle_y = track_bottom - display.normalized * (track_bottom - track_top);
    painter.line_segment(
        [
            egui::pos2(track_x, handle_y),
            egui::pos2(track_x, track_bottom),
        ],
        egui::Stroke::new(8.0, if highlighted { epm_ok() } else { epm_orange() }),
    );
    let handle =
        egui::Rect::from_center_size(egui::pos2(track_x, handle_y), egui::vec2(36.0, 10.0));
    painter.rect_filled(handle, egui::CornerRadius::same(2), epm_text());
    painter.rect_stroke(
        handle,
        egui::CornerRadius::same(2),
        egui::Stroke::new(1.0, epm_bg()),
        egui::StrokeKind::Inside,
    );

    draw_centered_text(&painter, rect, 10.0, &binding.control, 11.0, epm_orange());
    draw_centered_text(
        &painter,
        rect,
        24.0,
        &format!("CC{}", binding.cc),
        10.0,
        epm_muted(),
    );
    draw_centered_text(
        &painter,
        rect,
        rect.height() - 34.0,
        &display.short_action,
        10.0,
        epm_cyan(),
    );
    draw_centered_text(
        &painter,
        rect,
        rect.height() - 18.0,
        &display.value,
        13.0,
        epm_text(),
    );
}

pub(crate) fn render_pc4_switch(
    ui: &mut egui::Ui,
    binding: &ControllerBinding,
    snapshot: &EngineSnapshot,
    last_control: Option<&LastControlEvent>,
    size: egui::Vec2,
) {
    let display = pc4_control_display(snapshot, binding);
    let highlighted = last_control.is_some_and(|event| event_matches_binding(event, binding));
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    draw_pc4_control_shell(&painter, rect, highlighted);

    let switch_rect = egui::Rect::from_center_size(
        egui::pos2(rect.center().x, rect.top() + 22.0),
        egui::vec2((rect.width() - 26.0).clamp(42.0, 76.0), 16.0),
    );
    painter.rect_filled(switch_rect, egui::CornerRadius::same(8), epm_panel());
    painter.rect_stroke(
        switch_rect,
        egui::CornerRadius::same(8),
        epm_stroke(),
        egui::StrokeKind::Inside,
    );
    let knob_x = if display.normalized >= 0.5 || highlighted {
        switch_rect.right() - 9.0
    } else {
        switch_rect.left() + 9.0
    };
    painter.circle_filled(
        egui::pos2(knob_x, switch_rect.center().y),
        6.0,
        if highlighted {
            epm_ok()
        } else {
            epm_orange_dim()
        },
    );
    draw_centered_text(&painter, rect, 35.0, &binding.control, 10.0, epm_orange());
    draw_centered_text(
        &painter,
        rect,
        48.0,
        &format!("CC{} {}", binding.cc, display.value),
        9.0,
        epm_muted(),
    );
}

pub(crate) fn render_pc4_program_slot(
    ui: &mut egui::Ui,
    highlighted: bool,
    slot: usize,
    stem: &str,
    selected: bool,
    size: egui::Vec2,
) {
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    draw_pc4_control_shell(&painter, rect, highlighted);
    draw_centered_text(
        &painter,
        rect,
        6.0,
        &format!("PC{slot}"),
        10.0,
        if selected { epm_orange() } else { epm_muted() },
    );
    draw_centered_text(&painter, rect, 20.0, stem, 9.0, epm_text());
}

pub(crate) fn render_metric_tile(
    ui: &mut egui::Ui,
    highlighted: bool,
    title: &str,
    value: &str,
    detail: &str,
) {
    render_metric_tile_sized(ui, highlighted, title, value, detail, 112.0);
}

pub(crate) fn render_metric_tile_sized(
    ui: &mut egui::Ui,
    highlighted: bool,
    title: &str,
    value: &str,
    detail: &str,
    width: f32,
) {
    epm_tile_frame(highlighted).show(ui, |ui| {
        ui.set_width(width);
        ui.set_min_width(width);
        ui.label(epm_eyebrow(title));
        ui.label(epm_value(value));
        if !detail.is_empty() {
            ui.label(epm_small(detail));
        }
    });
}

pub(crate) fn epm_command_button(
    label: &str,
    danger: bool,
    selected: bool,
) -> egui::Button<'static> {
    let fill = if danger {
        egui::Color32::from_rgb(115, 25, 22)
    } else if selected {
        epm_tile_hot()
    } else {
        epm_tile()
    };
    let stroke = if danger || selected {
        epm_accent_stroke()
    } else {
        epm_stroke()
    };
    egui::Button::new(
        egui::RichText::new(label.to_string())
            .monospace()
            .size(12.0)
            .strong()
            .color(epm_text()),
    )
    .fill(fill)
    .stroke(stroke)
    .corner_radius(egui::CornerRadius::same(4))
    .min_size(egui::vec2(92.0, 34.0))
}
