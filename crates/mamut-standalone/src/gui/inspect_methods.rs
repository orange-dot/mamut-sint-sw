use super::*;

const GFM_FIELD_MAP_MAX_SIDE: f32 = 520.0;
const GFM_FIELD_CELL_GAP: f32 = 1.0;
const GFM_STRIKE_MARKER_FADE_SECONDS: f32 = 1.6;

impl PerformanceApp {
    pub(crate) fn render_inspect_tab(&mut self, ui: &mut egui::Ui) {
        epm_frame(epm_panel()).show(ui, |ui| {
            ui.label(epm_eyebrow("GFM FIELD"));
            ui.add_space(6.0);

            let Some(snapshot) = self.snapshot.as_ref() else {
                ui.label(epm_body("waiting for the first engine snapshot"));
                return;
            };
            let layer = snapshot.gfm_layer;
            let Some(terrain) = layer.terrain else {
                ui.label(epm_body(
                    "GFM layer is disarmed. Arm it (K8 + aftertouch, or the Sound Lab \
                     GFM toggle) to watch the live field while the audible gate stays \
                     closed.",
                ));
                return;
            };

            let program = layer
                .active_program_id
                .map(|id| format!("{id:?}"))
                .unwrap_or_else(|| "-".to_string());
            let (max_ruptures, strike_count) = layer
                .diagnostics
                .map(|diagnostics| (diagnostics.max_rupture_count, diagnostics.strike_count))
                .unwrap_or((0, 0));
            ui.label(epm_small(format!(
                "program {program}  |  strikes {strike_count}  |  max ruptures {max_ruptures}  |  note strikes {}",
                if layer.note_strikes_enabled {
                    "on"
                } else {
                    "off"
                }
            )));
            ui.add_space(8.0);
            render_gfm_field_map(ui, &terrain, snapshot.sample_rate_hz);
            ui.add_space(8.0);
            render_gfm_field_legend(ui);
        });
    }
}

fn render_gfm_field_map(ui: &mut egui::Ui, terrain: &GfmTerrainSnapshot16, sample_rate_hz: f32) {
    let rows = terrain.energy.len();
    let columns = terrain.energy[0].len();
    let side = ui.available_width().min(GFM_FIELD_MAP_MAX_SIDE);
    let cell = ((side - GFM_FIELD_CELL_GAP * (columns as f32 - 1.0)) / columns as f32)
        .floor()
        .max(6.0);
    let step = cell + GFM_FIELD_CELL_GAP;
    let map_size = egui::vec2(
        cell * columns as f32 + GFM_FIELD_CELL_GAP * (columns as f32 - 1.0),
        cell * rows as f32 + GFM_FIELD_CELL_GAP * (rows as f32 - 1.0),
    );
    let (response, painter) = ui.allocate_painter(map_size, egui::Sense::hover());
    let origin = response.rect.min;
    let cell_rect = |x: usize, y: usize| {
        egui::Rect::from_min_size(
            origin + egui::vec2(x as f32 * step, y as f32 * step),
            egui::vec2(cell, cell),
        )
    };

    for y in 0..rows {
        for x in 0..columns {
            let rect = cell_rect(x, y);
            painter.rect_filled(
                rect,
                egui::CornerRadius::same(2),
                gfm_cell_fill(terrain.energy[y][x], terrain.heat[y][x]),
            );
            if let Some(stroke) = gfm_health_stroke(terrain.health[y][x]) {
                painter.rect_stroke(
                    rect,
                    egui::CornerRadius::same(2),
                    stroke,
                    egui::StrokeKind::Inside,
                );
            }
            if terrain.recently_ruptured[y][x] {
                painter.circle_filled(rect.center(), (cell * 0.14).max(1.5), epm_bad());
            }
        }
    }

    for marker in terrain.recent_strikes.iter().flatten() {
        let age_frames = terrain.frame_index.saturating_sub(marker.frame_index);
        let alpha = gfm_strike_marker_alpha(age_frames, sample_rate_hz);
        if alpha == 0 {
            continue;
        }
        let center = cell_rect(marker.x as usize, marker.y as usize).center();
        let color = egui::Color32::from_rgba_unmultiplied(255, 106, 24, alpha);
        painter.circle_stroke(center, cell * 0.55, egui::Stroke::new(2.0, color));
    }

    let probe_center = cell_rect(terrain.probe.0 as usize, terrain.probe.1 as usize).center();
    let cross_arm = cell * 0.65;
    let probe_stroke = egui::Stroke::new(1.5, epm_cyan());
    painter.line_segment(
        [
            probe_center - egui::vec2(cross_arm, 0.0),
            probe_center + egui::vec2(cross_arm, 0.0),
        ],
        probe_stroke,
    );
    painter.line_segment(
        [
            probe_center - egui::vec2(0.0, cross_arm),
            probe_center + egui::vec2(0.0, cross_arm),
        ],
        probe_stroke,
    );
    for (label_position, glyph) in [
        (terrain.stereo_probes.0, "L"),
        (terrain.stereo_probes.1, "R"),
    ] {
        let center = cell_rect(label_position.0 as usize, label_position.1 as usize).center();
        painter.circle_stroke(center, cell * 0.34, probe_stroke);
        painter.text(
            center,
            egui::Align2::CENTER_CENTER,
            glyph,
            egui::FontId::monospace((cell * 0.5).max(8.0)),
            epm_cyan(),
        );
    }
}

fn render_gfm_field_legend(ui: &mut egui::Ui) {
    ui.label(epm_small(
        "fill: energy (dark → cyan), warm shift: heat  |  border: suspect / \
         quarantined / recovering  |  dot: recent rupture  |  fading ring: note \
         strike  |  cross: mono probe, L/R: stereo probes",
    ));
}

fn gfm_cell_fill(energy: u8, heat: u8) -> egui::Color32 {
    let energy = energy as f32 / 255.0;
    let heat = heat as f32 / 255.0;
    let base = if energy < 0.5 {
        lerp_color(epm_panel_deep(), epm_muted(), energy * 2.0)
    } else {
        lerp_color(epm_muted(), epm_cyan(), (energy - 0.5) * 2.0)
    };
    lerp_color(base, epm_orange(), heat * 0.45)
}

fn gfm_health_stroke(health: mamut_engine::GfmCellHealth) -> Option<egui::Stroke> {
    match health {
        mamut_engine::GfmCellHealth::Healthy => None,
        mamut_engine::GfmCellHealth::Suspect => Some(egui::Stroke::new(1.0, epm_orange_dim())),
        mamut_engine::GfmCellHealth::Quarantined => Some(egui::Stroke::new(1.5, epm_bad())),
        mamut_engine::GfmCellHealth::Recovering => Some(egui::Stroke::new(1.0, epm_ok())),
    }
}

fn gfm_strike_marker_alpha(age_frames: u64, sample_rate_hz: f32) -> u8 {
    let fade_frames = (sample_rate_hz.max(1.0) * GFM_STRIKE_MARKER_FADE_SECONDS) as u64;
    if fade_frames == 0 || age_frames >= fade_frames {
        return 0;
    }
    (255.0 * (1.0 - age_frames as f32 / fade_frames as f32)).round() as u8
}

fn lerp_color(from: egui::Color32, to: egui::Color32, amount: f32) -> egui::Color32 {
    let amount = if amount.is_finite() {
        amount.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let channel = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * amount).round() as u8;
    egui::Color32::from_rgb(
        channel(from.r(), to.r()),
        channel(from.g(), to.g()),
        channel(from.b(), to.b()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strike_marker_alpha_fades_monotonically_and_expires() {
        let sample_rate_hz = 48_000.0;
        let fresh = gfm_strike_marker_alpha(0, sample_rate_hz);
        let mid = gfm_strike_marker_alpha(24_000, sample_rate_hz);
        let old = gfm_strike_marker_alpha(200_000, sample_rate_hz);
        assert_eq!(fresh, 255);
        assert!(mid < fresh && mid > 0);
        assert_eq!(old, 0);
        assert_eq!(gfm_strike_marker_alpha(0, 0.0), 255);
    }

    #[test]
    fn cell_fill_tracks_energy_and_heat() {
        let dark = gfm_cell_fill(0, 0);
        let bright = gfm_cell_fill(255, 0);
        let hot = gfm_cell_fill(128, 255);
        let cold = gfm_cell_fill(128, 0);
        assert_ne!(dark, bright);
        assert!(bright.r() > dark.r() && bright.b() > dark.b());
        assert!(hot.r() > cold.r(), "heat must shift the fill warm");
    }

    #[test]
    fn health_strokes_flag_only_unhealthy_cells() {
        assert!(gfm_health_stroke(mamut_engine::GfmCellHealth::Healthy).is_none());
        for health in [
            mamut_engine::GfmCellHealth::Suspect,
            mamut_engine::GfmCellHealth::Quarantined,
            mamut_engine::GfmCellHealth::Recovering,
        ] {
            assert!(gfm_health_stroke(health).is_some());
        }
    }
}
