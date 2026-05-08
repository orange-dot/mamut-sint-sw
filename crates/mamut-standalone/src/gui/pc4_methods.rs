use super::*;

impl PerformanceApp {
    pub(crate) fn render_pc4_tab(&mut self, ui: &mut egui::Ui) {
        let input = self.session.input_metrics_snapshot();

        let Some(snapshot) = self.snapshot.as_ref() else {
            epm_frame(epm_panel()).show(ui, |ui| {
                ui.label(epm_body("waiting for engine snapshot"));
            });
            return;
        };

        if let Some(profile) = self.session.controller_profile.clone() {
            self.render_pc4_surface(ui, &profile, snapshot, input.last_control.as_ref());
            let other = sorted_bindings_for_section(&profile, ControllerBindingSection::Other);
            if !other.is_empty() {
                ui.add_space(6.0);
                self.render_binding_grid(
                    ui,
                    "Other",
                    &other,
                    snapshot,
                    input.last_control.as_ref(),
                );
            }
        } else {
            self.render_legacy_pc4_tab(ui, snapshot, input.last_control.as_ref());
            ui.add_space(12.0);
            self.render_program_change_map(ui, input.last_control.as_ref());
        }
    }

    pub(crate) fn render_pc4_surface(
        &self,
        ui: &mut egui::Ui,
        profile: &ControllerProfile,
        snapshot: &EngineSnapshot,
        last_control: Option<&LastControlEvent>,
    ) {
        let knobs = sorted_bindings_for_section(profile, ControllerBindingSection::Knob);
        let sliders = sorted_bindings_for_section(profile, ControllerBindingSection::Slider);
        let switches = sorted_bindings_for_section(profile, ControllerBindingSection::Switch);

        self.render_pc4_status_bar(ui, profile, last_control);
        ui.add_space(6.0);
        self.render_pc4_knob_bank(ui, &knobs, snapshot, last_control);
        ui.add_space(6.0);
        self.render_pc4_slider_bank(ui, &sliders, snapshot, last_control);
        ui.add_space(6.0);
        self.render_pc4_switch_and_program_bank(ui, &switches, snapshot, last_control);
    }

    pub(crate) fn render_pc4_status_bar(
        &self,
        ui: &mut egui::Ui,
        profile: &ControllerProfile,
        last_control: Option<&LastControlEvent>,
    ) {
        compact_epm_frame(epm_panel()).show(ui, |ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), 44.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.vertical(|ui| {
                        ui.label(epm_eyebrow("PC4 CONTROL SURFACE"));
                        ui.label(epm_small(format!(
                            "{} / {}",
                            profile.name,
                            profile.path.display()
                        )));
                    });
                    ui.add_space(20.0);
                    ui.vertical(|ui| {
                        ui.label(epm_eyebrow("LATEST"));
                        if let Some(event) = last_control {
                            ui.label(epm_value(format!("{} -> {}", event.label, event.action)));
                        } else {
                            ui.label(epm_value("none"));
                        }
                    });
                    ui.add_space(20.0);
                    ui.vertical(|ui| {
                        ui.label(epm_eyebrow("VERDICT"));
                        ui.label(epm_value(
                            last_control
                                .map(|event| verdict_label(event.verdict))
                                .unwrap_or("idle"),
                        ));
                    });
                    ui.add_space(20.0);
                    ui.vertical(|ui| {
                        ui.label(epm_eyebrow("PATCH"));
                        ui.label(epm_value(&self.session.patch_name));
                    });
                },
            );
        });
    }

    pub(crate) fn render_pc4_knob_bank(
        &self,
        ui: &mut egui::Ui,
        bindings: &[&ControllerBinding],
        snapshot: &EngineSnapshot,
        last_control: Option<&LastControlEvent>,
    ) {
        compact_epm_frame(epm_panel_deep()).show(ui, |ui| {
            ui.label(epm_eyebrow("KNOBS"));
            ui.add_space(4.0);
            let count = bindings.len().max(1) as f32;
            let spacing = 8.0;
            let width = ((ui.available_width() - spacing * (count - 1.0)) / count).max(78.0);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = spacing;
                for binding in bindings {
                    render_pc4_knob(
                        ui,
                        binding,
                        snapshot,
                        last_control,
                        egui::vec2(width, 124.0),
                    );
                }
            });
        });
    }

    pub(crate) fn render_pc4_slider_bank(
        &self,
        ui: &mut egui::Ui,
        bindings: &[&ControllerBinding],
        snapshot: &EngineSnapshot,
        last_control: Option<&LastControlEvent>,
    ) {
        compact_epm_frame(epm_panel()).show(ui, |ui| {
            ui.label(epm_eyebrow("SLIDERS"));
            ui.add_space(4.0);
            let count = bindings.len().max(1) as f32;
            let spacing = 8.0;
            let width = ((ui.available_width() - spacing * (count - 1.0)) / count).max(78.0);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = spacing;
                for binding in bindings {
                    render_pc4_slider(
                        ui,
                        binding,
                        snapshot,
                        last_control,
                        egui::vec2(width, 204.0),
                    );
                }
            });
        });
    }

    pub(crate) fn render_pc4_switch_and_program_bank(
        &self,
        ui: &mut egui::Ui,
        bindings: &[&ControllerBinding],
        snapshot: &EngineSnapshot,
        last_control: Option<&LastControlEvent>,
    ) {
        compact_epm_frame(epm_panel_deep()).show(ui, |ui| {
            ui.label(epm_eyebrow("SWITCHES"));
            ui.add_space(4.0);
            let count = bindings.len().max(1) as f32;
            let spacing = 6.0;
            let width = ((ui.available_width() - spacing * (count - 1.0)) / count).max(72.0);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = spacing;
                for binding in bindings {
                    render_pc4_switch(ui, binding, snapshot, last_control, egui::vec2(width, 58.0));
                }
            });
            ui.add_space(6.0);
            self.render_program_change_strip(ui, last_control);
        });
    }

    pub(crate) fn render_binding_grid(
        &self,
        ui: &mut egui::Ui,
        title: &str,
        bindings: &[&ControllerBinding],
        snapshot: &EngineSnapshot,
        last_control: Option<&LastControlEvent>,
    ) {
        epm_frame(epm_panel()).show(ui, |ui| {
            ui.label(epm_eyebrow(title.to_ascii_uppercase()));
            ui.add_space(6.0);
            egui::Grid::new(format!("pc4-{title}"))
                .num_columns(3)
                .spacing([10.0, 10.0])
                .show(ui, |ui| {
                    for (index, binding) in bindings.iter().enumerate() {
                        render_binding_tile(ui, binding, snapshot, last_control);
                        if index % 3 == 2 {
                            ui.end_row();
                        }
                    }
                });
        });
    }

    pub(crate) fn render_legacy_pc4_tab(
        &self,
        ui: &mut egui::Ui,
        snapshot: &EngineSnapshot,
        last_control: Option<&LastControlEvent>,
    ) {
        epm_frame(epm_panel()).show(ui, |ui| {
            ui.label(epm_eyebrow("LEGACY PC4"));
            ui.label(epm_body(
                "Full PC4 profile not loaded; showing legacy live map.",
            ));
        });
        ui.add_space(12.0);
        let legacy = [
            (
                16,
                "Legacy CC16",
                "macro Gravitacija",
                snapshot.live_macros.gravitacija,
            ),
            (17, "Legacy CC17", "macro Bloom", snapshot.live_macros.bloom),
            (18, "Legacy CC18", "macro Heat", snapshot.live_macros.heat),
            (19, "Legacy CC19", "macro Ruin", snapshot.live_macros.ruin),
            (20, "Legacy CC20", "macro Swarm", snapshot.live_macros.swarm),
        ];
        epm_frame(epm_panel()).show(ui, |ui| {
            egui::Grid::new("pc4-legacy-macros")
                .num_columns(5)
                .spacing([10.0, 10.0])
                .show(ui, |ui| {
                    for (cc, label, action, value) in legacy {
                        let highlighted =
                            last_control.is_some_and(|event| event_matches_legacy_cc(event, cc));
                        render_small_status_tile(
                            ui,
                            highlighted,
                            label,
                            &format!("CC{cc}"),
                            action,
                            &format!("{value:.2}"),
                        );
                    }
                });
            ui.add_space(6.0);
            ui.label(epm_small(
                "CC1 mod wheel / CC64 sustain / channel aftertouch / pitch bend",
            ));
        });
    }

    pub(crate) fn render_program_change_map(
        &self,
        ui: &mut egui::Ui,
        last_control: Option<&LastControlEvent>,
    ) {
        epm_frame(epm_panel()).show(ui, |ui| {
            ui.label(epm_eyebrow("PROGRAM CHANGE"));
            ui.add_space(6.0);
            egui::Grid::new("pc4-program-change")
                .num_columns(4)
                .spacing([10.0, 10.0])
                .show(ui, |ui| {
                    for (slot, stem) in LIVE_SET_STEMS.iter().enumerate() {
                        let highlighted = last_control
                            .is_some_and(|event| event_matches_program_change(event, slot as u8));
                        render_small_status_tile(
                            ui,
                            highlighted,
                            &format!("PC {slot}"),
                            "program",
                            stem,
                            if self.session.current_live_slot() == Some(slot) {
                                "current"
                            } else {
                                ""
                            },
                        );
                        if slot % 4 == 3 {
                            ui.end_row();
                        }
                    }
                });
        });
    }

    pub(crate) fn render_program_change_strip(
        &self,
        ui: &mut egui::Ui,
        last_control: Option<&LastControlEvent>,
    ) {
        ui.horizontal_top(|ui| {
            ui.label(epm_eyebrow("PROGRAM CHANGE"));
            ui.add_space(6.0);
            let count = LIVE_SET_STEMS.len() as f32;
            let spacing = 5.0;
            let available = (ui.available_width() - 6.0).max(360.0);
            let width = ((available - spacing * (count - 1.0)) / count).max(68.0);
            ui.spacing_mut().item_spacing.x = spacing;
            for (slot, stem) in LIVE_SET_STEMS.iter().enumerate() {
                let highlighted = last_control
                    .is_some_and(|event| event_matches_program_change(event, slot as u8));
                let selected = self.session.current_live_slot() == Some(slot);
                render_pc4_program_slot(
                    ui,
                    highlighted || selected,
                    slot,
                    stem,
                    selected,
                    egui::vec2(width, 34.0),
                );
            }
        });
    }
}
