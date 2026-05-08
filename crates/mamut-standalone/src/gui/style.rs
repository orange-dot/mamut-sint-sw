use super::*;

pub(crate) fn display_available() -> bool {
    env::var_os("DISPLAY").is_some() || env::var_os("WAYLAND_DISPLAY").is_some()
}

pub(crate) fn epm_bg() -> egui::Color32 {
    egui::Color32::from_rgb(12, 14, 16)
}

pub(crate) fn epm_panel() -> egui::Color32 {
    egui::Color32::from_rgb(17, 21, 25)
}

pub(crate) fn epm_panel_deep() -> egui::Color32 {
    egui::Color32::from_rgb(13, 16, 19)
}

pub(crate) fn epm_tile() -> egui::Color32 {
    egui::Color32::from_rgb(18, 23, 28)
}

pub(crate) fn epm_tile_hot() -> egui::Color32 {
    egui::Color32::from_rgb(54, 29, 15)
}

pub(crate) fn epm_stroke() -> egui::Stroke {
    egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 52, 60))
}

pub(crate) fn epm_accent_stroke() -> egui::Stroke {
    egui::Stroke::new(1.0, epm_orange())
}

pub(crate) fn epm_orange() -> egui::Color32 {
    egui::Color32::from_rgb(255, 106, 24)
}

pub(crate) fn epm_orange_dim() -> egui::Color32 {
    egui::Color32::from_rgb(156, 69, 24)
}

pub(crate) fn epm_text() -> egui::Color32 {
    egui::Color32::from_rgb(244, 247, 250)
}

pub(crate) fn epm_muted() -> egui::Color32 {
    egui::Color32::from_rgb(143, 158, 171)
}

pub(crate) fn epm_cyan() -> egui::Color32 {
    egui::Color32::from_rgb(189, 222, 244)
}

pub(crate) fn epm_ok() -> egui::Color32 {
    egui::Color32::from_rgb(84, 220, 135)
}

pub(crate) fn epm_bad() -> egui::Color32 {
    egui::Color32::from_rgb(232, 83, 75)
}

pub(crate) fn epm_frame(fill: egui::Color32) -> egui::Frame {
    egui::Frame::NONE
        .fill(fill)
        .stroke(epm_stroke())
        .corner_radius(egui::CornerRadius::same(4))
        .inner_margin(egui::Margin::symmetric(14, 12))
}

pub(crate) fn compact_epm_frame(fill: egui::Color32) -> egui::Frame {
    egui::Frame::NONE
        .fill(fill)
        .stroke(epm_stroke())
        .corner_radius(egui::CornerRadius::same(4))
        .inner_margin(egui::Margin::symmetric(10, 8))
}

pub(crate) fn epm_tile_frame(highlighted: bool) -> egui::Frame {
    egui::Frame::NONE
        .fill(if highlighted {
            epm_tile_hot()
        } else {
            epm_tile()
        })
        .stroke(if highlighted {
            epm_accent_stroke()
        } else {
            epm_stroke()
        })
        .corner_radius(egui::CornerRadius::same(4))
        .inner_margin(egui::Margin::symmetric(12, 10))
}

pub(crate) fn epm_eyebrow(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text.into())
        .monospace()
        .size(11.0)
        .strong()
        .color(epm_orange())
}

pub(crate) fn epm_heading(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text.into())
        .size(28.0)
        .strong()
        .color(epm_text())
}

pub(crate) fn epm_subheading(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text.into())
        .size(16.0)
        .strong()
        .color(epm_text())
}

pub(crate) fn epm_body(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text.into())
        .size(14.0)
        .color(epm_cyan())
}

pub(crate) fn epm_small(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text.into())
        .monospace()
        .size(11.0)
        .color(epm_muted())
}

pub(crate) fn epm_value(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text.into())
        .size(17.0)
        .strong()
        .color(epm_text())
}

pub(crate) fn apply_epm_gui_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.window_fill = epm_bg();
    style.visuals.panel_fill = epm_bg();
    style.visuals.faint_bg_color = epm_panel_deep();
    style.visuals.extreme_bg_color = epm_panel_deep();
    style.visuals.override_text_color = Some(epm_text());
    style.visuals.hyperlink_color = epm_orange();
    style.visuals.selection.bg_fill = epm_orange_dim();
    style.visuals.selection.stroke = epm_accent_stroke();
    style.visuals.widgets.noninteractive.bg_fill = epm_panel();
    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, epm_text());
    style.visuals.widgets.inactive.bg_fill = epm_tile();
    style.visuals.widgets.inactive.bg_stroke = epm_stroke();
    style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(4);
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(39, 31, 25);
    style.visuals.widgets.hovered.bg_stroke = epm_accent_stroke();
    style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(4);
    style.visuals.widgets.active.bg_fill = epm_tile_hot();
    style.visuals.widgets.active.bg_stroke = epm_accent_stroke();
    style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(4);
    style.visuals.widgets.open.bg_fill = epm_tile_hot();
    style.visuals.widgets.open.bg_stroke = epm_accent_stroke();
    style.visuals.widgets.open.corner_radius = egui::CornerRadius::same(4);
    style.visuals.window_corner_radius = egui::CornerRadius::same(4);
    style.spacing.item_spacing = egui::vec2(10.0, 8.0);
    style.spacing.button_padding = egui::vec2(13.0, 9.0);
    style.spacing.window_margin = egui::Margin::symmetric(18, 16);
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::new(28.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::new(14.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        egui::FontId::new(12.0, egui::FontFamily::Monospace),
    );
    ctx.set_style(style);
}
