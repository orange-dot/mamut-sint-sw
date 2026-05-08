use super::*;

impl PerformanceApp {
    pub(crate) fn render_sound_lab_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        self.render_header(ui);
        ui.add_space(12.0);
        let input = self.session.input_metrics_snapshot();
        let controller_profile = self.session.controller_profile.clone();
        let active_source =
            sound_lab_active_source(input.last_control.as_ref(), controller_profile.as_deref());

        let Some(snapshot) = self.snapshot.clone() else {
            epm_frame(epm_panel()).show(ui, |ui| {
                ui.label(epm_eyebrow("SOUND LAB"));
                ui.label(epm_body("waiting for engine snapshot"));
            });
            return;
        };

        self.render_sound_lab_export_panel(ui, &snapshot);
        ui.add_space(12.0);
        self.render_sound_lab_identity_panel(ui, &snapshot);
        ui.add_space(12.0);
        self.render_sound_lab_page_selector(ui);
        ui.add_space(12.0);
        self.render_sound_lab_midi_focus_panel(ui, input.last_control.as_ref(), active_source);
        ui.add_space(12.0);
        self.render_sound_lab_page_content(ui, &snapshot, active_source);
        ui.add_space(12.0);
        self.render_footer(ui, ctx);
    }

    pub(crate) fn render_sound_lab_page_selector(&mut self, ui: &mut egui::Ui) {
        epm_frame(epm_panel_deep()).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for page in SoundLabPage::ALL {
                    if ui
                        .add_sized(
                            [118.0, 34.0],
                            epm_command_button(page.label(), false, self.sound_lab_page == page),
                        )
                        .clicked()
                    {
                        self.sound_lab_page = page;
                    }
                }
            });
        });
    }

    pub(crate) fn render_sound_lab_midi_focus_panel(
        &self,
        ui: &mut egui::Ui,
        last_control: Option<&LastControlEvent>,
        active_source: Option<SoundLabMidiSource>,
    ) {
        epm_frame(epm_panel()).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                render_metric_tile(
                    ui,
                    true,
                    "MIDI FOCUS",
                    "Sound Lab",
                    self.sound_lab_page.label(),
                );
                if let Some(event) = last_control.filter(|event| event_is_recent(event)) {
                    render_metric_tile(ui, true, "LATEST", &event.label, &event.action);
                } else {
                    render_metric_tile(ui, false, "LATEST", "none", "idle");
                }
                render_metric_tile(
                    ui,
                    active_source.is_some(),
                    "ACTIVE",
                    active_source
                        .map(|source| source.badge())
                        .as_deref()
                        .unwrap_or("-"),
                    "pc4 source",
                );
                render_metric_tile(ui, false, "PAGE MAP", "K1-K7 K9", "primary controls");
                render_metric_tile(ui, false, "PAGE MAP", "S1-S8", "secondary controls");
                render_metric_tile(ui, false, "PAGE MACRO", "MW", "current page");
                render_metric_tile(ui, false, "GLOBAL", "K8 S9 SW9", "layers stay live");
            });
        });
    }

    pub(crate) fn render_sound_lab_page_content(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &EngineSnapshot,
        active_source: Option<SoundLabMidiSource>,
    ) {
        if self.sound_lab_page == SoundLabPage::Layers {
            self.render_sound_lab_layers_and_recorder(ui);
            return;
        }

        self.render_sound_lab_knob_bank(
            ui,
            snapshot,
            sound_lab_page_knob_bindings(self.sound_lab_page),
            active_source,
        );
        ui.add_space(6.0);
        self.render_sound_lab_slider_bank(
            ui,
            sound_lab_page_slider_bindings(self.sound_lab_page),
            snapshot,
            active_source,
        );
        ui.add_space(6.0);
        self.render_sound_lab_mod_wheel_panel(ui, snapshot, self.sound_lab_page, active_source);
    }

    pub(crate) fn render_sound_lab_knob_bank(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &EngineSnapshot,
        bindings: &[SoundLabMidiParamBinding],
        active_source: Option<SoundLabMidiSource>,
    ) {
        compact_epm_frame(epm_panel_deep()).show(ui, |ui| {
            ui.label(epm_eyebrow(format!(
                "{} KNOBS",
                self.sound_lab_page.label().to_ascii_uppercase()
            )));
            ui.add_space(4.0);
            if bindings.is_empty() {
                ui.label(epm_small("no knob controls on this page"));
                return;
            }

            let count = bindings.len().max(1) as f32;
            let spacing = 8.0;
            let width = ((ui.available_width() - spacing * (count - 1.0)) / count).max(78.0);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = spacing;
                for binding in bindings {
                    self.render_sound_lab_knob(
                        ui,
                        snapshot,
                        *binding,
                        active_source == Some(binding.source),
                        egui::vec2(width, 124.0),
                    );
                }
            });
        });
    }

    pub(crate) fn render_sound_lab_slider_bank(
        &mut self,
        ui: &mut egui::Ui,
        bindings: &[SoundLabMidiParamBinding],
        snapshot: &EngineSnapshot,
        active_source: Option<SoundLabMidiSource>,
    ) {
        compact_epm_frame(epm_panel()).show(ui, |ui| {
            ui.label(epm_eyebrow(format!(
                "{} SLIDERS",
                self.sound_lab_page.label().to_ascii_uppercase()
            )));
            ui.add_space(4.0);
            if bindings.is_empty() {
                ui.label(epm_small("no slider controls on this page"));
                return;
            }

            let count = bindings.len().max(1) as f32;
            let spacing = 8.0;
            let width = ((ui.available_width() - spacing * (count - 1.0)) / count).max(78.0);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = spacing;
                for binding in bindings {
                    let spec = param_spec(binding.id);
                    if spec.unit == ParamUnit::Boolean {
                        self.render_sound_lab_switch(
                            ui,
                            snapshot,
                            *binding,
                            active_source == Some(binding.source),
                            egui::vec2(width, 204.0),
                        );
                    } else {
                        self.render_sound_lab_slider(
                            ui,
                            snapshot,
                            *binding,
                            active_source == Some(binding.source),
                            egui::vec2(width, 204.0),
                        );
                    }
                }
            });
        });
    }

    pub(crate) fn render_sound_lab_mod_wheel_panel(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &EngineSnapshot,
        page: SoundLabPage,
        active_source: Option<SoundLabMidiSource>,
    ) {
        let params = sound_lab_page_mod_wheel_params(page);
        epm_frame(epm_panel_deep()).show(ui, |ui| {
            ui.label(epm_eyebrow("MOD WHEEL PAGE MACRO"));
            ui.add_space(6.0);
            if params.is_empty() {
                ui.label(epm_small("MW is unassigned on this page"));
                return;
            }
            ui.horizontal_wrapped(|ui| {
                for id in params {
                    self.render_sound_lab_mod_wheel_control(
                        ui,
                        snapshot,
                        *id,
                        active_source == Some(SoundLabMidiSource::ModWheel),
                        egui::vec2(154.0, 58.0),
                    );
                }
            });
        });
    }

    pub(crate) fn render_sound_lab_export_panel(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &EngineSnapshot,
    ) {
        let fallback_name = format!("{} Lab", snapshot.patch_name);
        epm_frame(epm_panel()).show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.label(epm_eyebrow("SOUND LAB"));
                    ui.label(epm_heading("Runtime Color Cockpit"));
                    ui.label(epm_body(format!("export copy fallback: {fallback_name}")));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    if ui
                        .add(epm_command_button("EXPORT COPY", false, false))
                        .clicked()
                    {
                        let requested_name = self.sound_lab_export_name.clone();
                        match self.session.export_sound_lab_patch(&requested_name) {
                            Ok(path) => {
                                self.last_status_message =
                                    Some(format!("sound lab export -> {}", path.display()));
                            }
                            Err(error) => {
                                self.last_status_message =
                                    Some(format!("sound lab export failed: {error}"));
                            }
                        }
                    }
                    ui.add(
                        egui::TextEdit::singleline(&mut self.sound_lab_export_name)
                            .desired_width(230.0)
                            .hint_text(fallback_name)
                            .font(egui::TextStyle::Monospace),
                    );
                });
            });
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                render_metric_tile(ui, false, "PATCH", &snapshot.patch_name, "runtime");
                render_metric_tile(
                    ui,
                    snapshot.gfm_layer.effective_amount > 0.001,
                    "GFM",
                    match snapshot.gfm_layer.mode {
                        GfmLayerMode::Enabled { .. } => "enabled",
                        GfmLayerMode::Disabled => "disabled",
                    },
                    &format!("amount {:.2}", snapshot.gfm_layer.effective_amount),
                );
                render_metric_tile(
                    ui,
                    snapshot.bcs_layer.effective_gain > 0.001,
                    "BCS",
                    match snapshot.bcs_layer.mode {
                        BcsLayerMode::Enabled { .. } => "enabled",
                        BcsLayerMode::Disabled => "disabled",
                    },
                    &format!("gain {:.2}", snapshot.bcs_layer.effective_gain),
                );
                render_metric_tile(
                    ui,
                    snapshot.clip_detected,
                    "PEAK",
                    &format!("{:.3}", snapshot.peak_output),
                    if snapshot.clip_detected {
                        "clip"
                    } else {
                        "clean"
                    },
                );
            });
        });
    }

    pub(crate) fn render_sound_lab_identity_panel(
        &self,
        ui: &mut egui::Ui,
        snapshot: &EngineSnapshot,
    ) {
        epm_frame(epm_panel_deep()).show(ui, |ui| {
            ui.label(epm_eyebrow("IDENTITY"));
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                render_metric_tile(
                    ui,
                    false,
                    "HORIZONT",
                    &format!("{:.2}", snapshot.identity.horizont_open),
                    &format!("air {:.2}", snapshot.identity.horizont_air),
                );
                render_metric_tile(
                    ui,
                    false,
                    "PEC",
                    &format!("{:.2}", snapshot.identity.pec_mass),
                    &format!("heat {:.2}", snapshot.identity.pec_heat),
                );
                render_metric_tile(
                    ui,
                    false,
                    "BAKLJA",
                    &format!("{:.2}", snapshot.identity.baklja_ready),
                    &format!("edge {:.2}", snapshot.identity.baklja_edge),
                );
                render_metric_tile(
                    ui,
                    false,
                    "GRAVITY",
                    &format!("{:.2}", snapshot.identity.grav_pull),
                    &format!("mass {:.2}", snapshot.derived.mass),
                );
                render_metric_tile(
                    ui,
                    false,
                    "STRAIN",
                    &format!("{:.2}", snapshot.derived.strain),
                    &format!("threshold {:.2}", snapshot.derived.rupture_threshold),
                );
                render_metric_tile(
                    ui,
                    false,
                    "BEND",
                    &format!("{} st", snapshot.performance_response.bend_range_semitones),
                    "readout",
                );
            });
        });
    }

    pub(crate) fn render_sound_lab_knob(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &EngineSnapshot,
        binding: SoundLabMidiParamBinding,
        highlighted: bool,
        size: egui::Vec2,
    ) {
        let Some(display) = sound_lab_control_display(snapshot, binding.id) else {
            render_sound_lab_disabled_control(ui, binding, size);
            return;
        };
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        let active = highlighted || response.hovered() || response.dragged();
        draw_pc4_control_shell(&painter, rect, active);

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
            egui::Stroke::new(5.0, if active { epm_ok() } else { epm_orange() }),
        );
        let marker = egui::pos2(
            center.x + marker_angle.cos() * (radius - 5.0),
            center.y + marker_angle.sin() * (radius - 5.0),
        );
        painter.line_segment(
            [center, marker],
            egui::Stroke::new(2.0, if active { epm_ok() } else { epm_text() }),
        );

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
            82.0,
            &display.short_action,
            10.0,
            epm_cyan(),
        );
        draw_centered_text(&painter, rect, 98.0, &display.value, 13.0, epm_text());
        draw_centered_text(&painter, rect, 114.0, "drag", 9.0, epm_muted());

        if (response.dragged() || response.clicked())
            && let Some(pointer) = response.interact_pointer_pos()
        {
            let normalized = ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
            self.send_sound_lab_param(
                binding.id,
                sound_lab_param_value_from_normalized(binding.id, normalized),
            );
        }
    }

    pub(crate) fn render_sound_lab_slider(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &EngineSnapshot,
        binding: SoundLabMidiParamBinding,
        highlighted: bool,
        size: egui::Vec2,
    ) {
        let Some(display) = sound_lab_control_display(snapshot, binding.id) else {
            render_sound_lab_disabled_control(ui, binding, size);
            return;
        };
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        let active = highlighted || response.hovered() || response.dragged();
        draw_pc4_control_shell(&painter, rect, active);

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
            egui::Stroke::new(8.0, if active { epm_ok() } else { epm_orange() }),
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
            24.0,
            &display.short_action,
            10.0,
            epm_cyan(),
        );
        draw_centered_text(
            &painter,
            rect,
            rect.height() - 34.0,
            &display.value,
            13.0,
            epm_text(),
        );
        draw_centered_text(
            &painter,
            rect,
            rect.height() - 18.0,
            "drag",
            9.0,
            epm_muted(),
        );

        if (response.dragged() || response.clicked())
            && let Some(pointer) = response.interact_pointer_pos()
        {
            let normalized =
                ((track_bottom - pointer.y) / (track_bottom - track_top)).clamp(0.0, 1.0);
            self.send_sound_lab_param(
                binding.id,
                sound_lab_param_value_from_normalized(binding.id, normalized),
            );
        }
    }

    pub(crate) fn render_sound_lab_switch(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &EngineSnapshot,
        binding: SoundLabMidiParamBinding,
        highlighted: bool,
        size: egui::Vec2,
    ) {
        let Some(display) = sound_lab_control_display(snapshot, binding.id) else {
            render_sound_lab_disabled_control(ui, binding, size);
            return;
        };
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
        let painter = ui.painter_at(rect);
        let active = highlighted || response.hovered();
        draw_pc4_control_shell(&painter, rect, active);

        let switch_rect = egui::Rect::from_center_size(
            egui::pos2(rect.center().x, rect.top() + 78.0),
            egui::vec2((rect.width() - 24.0).clamp(42.0, 76.0), 18.0),
        );
        painter.rect_filled(switch_rect, egui::CornerRadius::same(9), epm_panel());
        painter.rect_stroke(
            switch_rect,
            egui::CornerRadius::same(9),
            epm_stroke(),
            egui::StrokeKind::Inside,
        );
        let knob_x = if display.normalized >= 0.5 || active {
            switch_rect.right() - 10.0
        } else {
            switch_rect.left() + 10.0
        };
        painter.circle_filled(
            egui::pos2(knob_x, switch_rect.center().y),
            7.0,
            if active { epm_ok() } else { epm_orange_dim() },
        );
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
            30.0,
            &display.short_action,
            10.0,
            epm_cyan(),
        );
        draw_centered_text(
            &painter,
            rect,
            rect.height() - 34.0,
            &display.value,
            13.0,
            epm_text(),
        );
        draw_centered_text(
            &painter,
            rect,
            rect.height() - 18.0,
            "click",
            9.0,
            epm_muted(),
        );

        if response.clicked() {
            let value = if display.normalized >= 0.5 { 0.0 } else { 1.0 };
            self.send_sound_lab_param(binding.id, value);
        }
    }

    pub(crate) fn render_sound_lab_mod_wheel_control(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &EngineSnapshot,
        id: ParamId,
        highlighted: bool,
        size: egui::Vec2,
    ) {
        let Some(display) = sound_lab_control_display(snapshot, id) else {
            let binding = SoundLabMidiParamBinding {
                source: SoundLabMidiSource::ModWheel,
                id,
            };
            render_sound_lab_disabled_control(ui, binding, size);
            return;
        };
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        let painter = ui.painter_at(rect);
        draw_pc4_control_shell(&painter, rect, highlighted);
        draw_centered_text(&painter, rect, 8.0, "MW", 11.0, epm_orange());
        draw_centered_text(
            &painter,
            rect,
            23.0,
            &display.short_action,
            10.0,
            epm_cyan(),
        );
        draw_centered_text(&painter, rect, 39.0, &display.value, 13.0, epm_text());
    }

    pub(crate) fn send_sound_lab_param(&mut self, id: ParamId, value: f32) {
        self.run_action(
            |session| session.set_direct_param(id, value),
            param_spec(id).name,
        );
        self.last_snapshot_refresh = Instant::now()
            .checked_sub(PERFORMANCE_UI_REFRESH)
            .unwrap_or_else(Instant::now);
    }

    pub(crate) fn render_sound_lab_layers_and_recorder(&mut self, ui: &mut egui::Ui) {
        ui.columns(2, |columns| {
            self.render_gfm_layer_panel(&mut columns[0]);
            self.render_bcs_layer_panel(&mut columns[1]);
        });
        ui.add_space(12.0);
        self.render_output_capture_controls(ui);
    }

    pub(crate) fn render_output_capture_controls(&mut self, ui: &mut egui::Ui) {
        let recording = self.session.recording_metrics_snapshot();
        let active = recording.state == RecordingState::Active;
        let seconds_written = recording.frames_written as f32 / self.session.sample_rate_hz as f32;
        let target_seconds = recording
            .target_frames
            .map(|frames| frames as f32 / self.session.sample_rate_hz as f32);
        let selected_seconds = self
            .selected_record_seconds()
            .unwrap_or(DEFAULT_LIVE_TAKE_SECONDS);
        let preview_path = tagged_output_capture_preview(
            &self.session.patch_path,
            &self.take_tag,
            selected_seconds,
        );

        epm_frame(epm_panel()).show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.label(epm_eyebrow("TAKE RECORDER"));
                    ui.label(epm_value(recording.state.label().to_ascii_uppercase()));
                    let path = recording
                        .path
                        .as_ref()
                        .map_or(preview_path.as_path(), PathBuf::as_path);
                    ui.label(epm_small(path.display().to_string()));
                    ui.label(epm_small(format!(
                        "midi {}",
                        output_recording_midi_log_path(path).display()
                    )));
                    if let Some(error) = &recording.error {
                        ui.colored_label(epm_bad(), error);
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    if ui
                        .add_enabled(active, epm_command_button("STOP REC", true, false))
                        .clicked()
                    {
                        self.run_action(
                            |session| session.stop_output_recording().map(|_| ()),
                            "record stop",
                        );
                    }
                    if ui
                        .add_enabled(!active, epm_command_button("RECORD", false, false))
                        .clicked()
                    {
                        match self.selected_record_seconds() {
                            Ok(seconds) => {
                                let tag = self.take_tag.clone();
                                self.run_action(
                                    |session| {
                                        session
                                            .start_tagged_output_recording(seconds, &tag)
                                            .map(|_| ())
                                    },
                                    &format!("record {seconds}s"),
                                );
                            }
                            Err(error) => {
                                self.last_status_message =
                                    Some(format!("record duration failed: {error}"));
                            }
                        }
                    }
                });
            });

            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                for (preset, label) in [
                    (RecordDurationPreset::Thirty, "30S"),
                    (RecordDurationPreset::Sixty, "60S"),
                    (RecordDurationPreset::Custom, "CUSTOM"),
                ] {
                    if ui
                        .add_enabled(
                            !active,
                            epm_command_button(label, false, self.record_duration_preset == preset),
                        )
                        .clicked()
                    {
                        self.record_duration_preset = preset;
                    }
                }
                ui.label(epm_small("seconds"));
                ui.add_enabled(
                    !active && self.record_duration_preset == RecordDurationPreset::Custom,
                    egui::TextEdit::singleline(&mut self.record_custom_seconds)
                        .desired_width(72.0)
                        .font(egui::TextStyle::Monospace),
                );
                ui.label(epm_small("tag"));
                ui.add_enabled(
                    !active,
                    egui::TextEdit::singleline(&mut self.take_tag)
                        .desired_width(150.0)
                        .font(egui::TextStyle::Monospace),
                );
            });
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                render_metric_tile(
                    ui,
                    active,
                    "SECONDS",
                    &format!("{seconds_written:.1}"),
                    &target_seconds
                        .map(|seconds| format!("target {seconds:.0}"))
                        .unwrap_or_else(|| "manual stop".to_string()),
                );
                render_metric_tile(
                    ui,
                    recording.frames_dropped > 0,
                    "DROPPED",
                    &recording.frames_dropped.to_string(),
                    "writer queue",
                );
                render_metric_tile(
                    ui,
                    recording.state == RecordingState::Finished,
                    "WAV",
                    "f32 stereo",
                    &format!("{} Hz", self.session.sample_rate_hz),
                );
                render_metric_tile(ui, recording.path.is_some(), "MIDI", ".midi.log", "sidecar");
            });
        });
    }
}
