use super::*;

const ENGINE_PARAM_SECTIONS: [ParamSection; 15] = [
    ParamSection::Macro,
    ParamSection::Oscillator1,
    ParamSection::Oscillator2,
    ParamSection::Spectral,
    ParamSection::Source,
    ParamSection::Sub,
    ParamSection::Mixer,
    ParamSection::Filter,
    ParamSection::AmpEnvelope,
    ParamSection::FilterEnvelope,
    ParamSection::Voice,
    ParamSection::Performance,
    ParamSection::FinalStage,
    ParamSection::Chorus,
    ParamSection::Reverb,
];

impl PerformanceApp {
    pub(crate) fn render_engine_tab(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        let snapshot = self.snapshot.clone();
        self.render_engine_signal_flow(ui, snapshot.as_ref());
        ui.add_space(12.0);
        self.render_engine_scope_panel(ui, snapshot.as_ref());
        ui.add_space(12.0);

        ui.columns(2, |columns| {
            self.render_engine_voice_inspector(&mut columns[0], snapshot.as_ref());
            self.render_engine_diagnostics(&mut columns[1], snapshot.as_ref());
        });

        ui.add_space(12.0);
        self.render_engine_param_editor(ui, snapshot.as_ref());
    }

    pub(crate) fn render_engine_signal_flow(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: Option<&EngineSnapshot>,
    ) {
        let (voices, peak, gfm, bcs, clip) = snapshot
            .map(|snapshot| {
                (
                    snapshot.active_voice_count,
                    snapshot.peak_output,
                    snapshot.gfm_layer.effective_amount,
                    snapshot.bcs_layer.effective_gain,
                    snapshot.clip_detected,
                )
            })
            .unwrap_or((0, 0.0, 0.0, 0.0, false));

        epm_frame(epm_panel()).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                render_metric_tile_sized(
                    ui,
                    voices > 0,
                    "INPUT",
                    &voices.to_string(),
                    "voices",
                    108.0,
                );
                render_metric_tile_sized(ui, false, "MACROS", "live", "G/B/H/R/S", 118.0);
                render_metric_tile_sized(ui, false, "IDENTITY", "resolved", "H/P/B/G", 132.0);
                render_metric_tile_sized(ui, false, "SOURCE", "osc+noise", "direct params", 136.0);
                render_metric_tile_sized(ui, false, "FILTER", "SVF", "drive/env", 112.0);
                render_metric_tile_sized(
                    ui,
                    gfm > 0.001,
                    "GFM",
                    &format!("{gfm:.2}"),
                    "layer",
                    96.0,
                );
                render_metric_tile_sized(
                    ui,
                    bcs > 0.001,
                    "BCS",
                    &format!("{bcs:.2}"),
                    "layer",
                    96.0,
                );
                render_metric_tile_sized(
                    ui,
                    clip,
                    "SAFETY",
                    &format!("{peak:.3}"),
                    if clip { "clip" } else { "peak" },
                    112.0,
                );
            });
        });
    }

    pub(crate) fn render_engine_scope_panel(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: Option<&EngineSnapshot>,
    ) {
        self.render_output_scope_panel(ui, snapshot, "MASTER OSCILLOSCOPE", 280.0);
    }

    pub(crate) fn render_compact_scope_panel(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: Option<&EngineSnapshot>,
        eyebrow: &str,
    ) {
        self.render_output_scope_panel(ui, snapshot, eyebrow, 190.0);
    }

    pub(crate) fn render_output_scope_panel(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: Option<&EngineSnapshot>,
        eyebrow: &str,
        height: f32,
    ) {
        epm_frame(epm_panel_deep()).show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.label(epm_eyebrow(eyebrow));
                    ui.label(epm_heading("Stereo Output"));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    let freeze_label = if self.engine_scope_frozen {
                        "LIVE"
                    } else {
                        "FREEZE"
                    };
                    if ui
                        .add(epm_command_button(
                            freeze_label,
                            false,
                            self.engine_scope_frozen,
                        ))
                        .clicked()
                    {
                        self.engine_scope_frozen = !self.engine_scope_frozen;
                        self.session
                            .set_scope_enabled(self.scope_enabled_for_selected_tab());
                    }
                    if ui.add(epm_command_button("CLEAR", false, false)).clicked() {
                        self.engine_scope_frames.clear();
                    }
                    ui.add(
                        egui::Slider::new(&mut self.engine_scope_gain, 0.25..=8.0)
                            .text("gain")
                            .logarithmic(true),
                    );
                    ui.add(
                        egui::Slider::new(
                            &mut self.engine_scope_window_frames,
                            256..=ENGINE_SCOPE_BUFFER_FRAMES,
                        )
                        .text("frames"),
                    );
                });
            });

            ui.add_space(10.0);
            self.draw_engine_scope(ui, snapshot, height);
        });
    }

    pub(crate) fn draw_engine_scope(
        &self,
        ui: &mut egui::Ui,
        snapshot: Option<&EngineSnapshot>,
        height: f32,
    ) {
        let desired_size = egui::vec2(ui.available_width(), height);
        let (rect, _) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, egui::CornerRadius::same(4), epm_bg());
        painter.rect_stroke(
            rect,
            egui::CornerRadius::same(4),
            epm_stroke(),
            egui::StrokeKind::Inside,
        );

        let center_y = rect.center().y;
        let half_height = rect.height() * 0.44;
        for line in 0..=4 {
            let y = rect.top() + rect.height() * line as f32 / 4.0;
            let color = if line == 2 {
                epm_stroke().color
            } else {
                epm_panel()
            };
            painter.line_segment(
                [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                egui::Stroke::new(1.0, color),
            );
        }

        let sample_count = self
            .engine_scope_window_frames
            .min(self.engine_scope_frames.len());
        if sample_count < 2 {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "waiting for master output",
                egui::FontId::new(14.0, egui::FontFamily::Monospace),
                epm_muted(),
            );
            return;
        }

        let start = self.engine_scope_frames.len() - sample_count;
        let samples = &self.engine_scope_frames[start..];
        let step = (sample_count / rect.width().max(1.0) as usize).max(1);
        let point_count = sample_count.div_ceil(step);
        let mut left_points = Vec::with_capacity(point_count);
        let mut right_points = Vec::with_capacity(point_count);

        for (point_index, frame_index) in (0..sample_count).step_by(step).enumerate() {
            let frame = samples[frame_index];
            let x = if point_count <= 1 {
                rect.left()
            } else {
                rect.left()
                    + rect.width() * point_index as f32 / (point_count.saturating_sub(1)) as f32
            };
            left_points.push(egui::pos2(
                x,
                center_y - frame[0].clamp(-1.0, 1.0) * half_height * self.engine_scope_gain,
            ));
            right_points.push(egui::pos2(
                x,
                center_y - frame[1].clamp(-1.0, 1.0) * half_height * self.engine_scope_gain,
            ));
        }

        painter.add(egui::Shape::line(
            left_points,
            egui::Stroke::new(1.6, epm_cyan()),
        ));
        painter.add(egui::Shape::line(
            right_points,
            egui::Stroke::new(1.2, epm_orange()),
        ));

        let peak = snapshot
            .map(|snapshot| snapshot.peak_output)
            .unwrap_or_default();
        let label = format!(
            "L cyan / R orange   window {} frames   gain {:.2}x   peak {:.3}{}",
            sample_count,
            self.engine_scope_gain,
            peak,
            if self.engine_scope_frozen {
                "   frozen"
            } else {
                ""
            }
        );
        painter.text(
            rect.left_top() + egui::vec2(12.0, 10.0),
            egui::Align2::LEFT_TOP,
            label,
            egui::FontId::new(12.0, egui::FontFamily::Monospace),
            epm_muted(),
        );
    }

    pub(crate) fn render_engine_voice_inspector(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: Option<&EngineSnapshot>,
    ) {
        epm_frame(epm_panel()).show(ui, |ui| {
            ui.label(epm_eyebrow("VOICE INSPECTOR"));
            ui.add_space(6.0);
            egui::Grid::new("engine-voice-inspector")
                .num_columns(5)
                .striped(true)
                .spacing(egui::vec2(14.0, 6.0))
                .show(ui, |ui| {
                    ui.label(epm_small("slot"));
                    ui.label(epm_small("note"));
                    ui.label(epm_small("phase"));
                    ui.label(epm_small("velocity"));
                    ui.label(epm_small("age"));
                    ui.end_row();

                    if let Some(snapshot) = snapshot {
                        for voice in &snapshot.voices {
                            ui.label(format!("{:02}", voice.slot));
                            ui.label(
                                voice
                                    .note
                                    .map(|note| note.to_string())
                                    .unwrap_or_else(|| "-".to_string()),
                            );
                            ui.label(format!("{:?}", voice.phase));
                            ui.label(format!("{:.3}", voice.velocity));
                            ui.label(voice.age.to_string());
                            ui.end_row();
                        }
                    }
                });
        });
    }

    pub(crate) fn render_engine_diagnostics(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: Option<&EngineSnapshot>,
    ) {
        epm_frame(epm_panel()).show(ui, |ui| {
            ui.label(epm_eyebrow("MODULE DIAGNOSTICS"));
            ui.add_space(8.0);

            if let Some(snapshot) = snapshot {
                ui.horizontal_wrapped(|ui| {
                    render_metric_tile_sized(
                        ui,
                        false,
                        "IDENTITY",
                        &format!("{:.2}", snapshot.identity.horizont_open),
                        &format!(
                            "pec {:.2} baklja {:.2} grav {:.2}",
                            snapshot.identity.pec_mass,
                            snapshot.identity.baklja_ready,
                            snapshot.identity.grav_pull
                        ),
                        240.0,
                    );
                    render_metric_tile_sized(
                        ui,
                        snapshot.output_safety.safety_limiter_hits > 0,
                        "SAFETY",
                        &snapshot.output_safety.safety_limiter_hits.to_string(),
                        &format!(
                            "pre {:.3} post {:.3}",
                            snapshot.output_safety.pre_safety_peak,
                            snapshot.output_safety.post_safety_peak
                        ),
                        220.0,
                    );
                    render_metric_tile_sized(
                        ui,
                        snapshot.gfm_layer.effective_amount > 0.001,
                        "GFM",
                        &format!("{:.2}", snapshot.gfm_layer.effective_amount),
                        &format!("program {:?}", snapshot.gfm_layer.active_program_id),
                        220.0,
                    );
                    render_metric_tile_sized(
                        ui,
                        snapshot.bcs_layer.unsafe_state,
                        "BCS",
                        &format!("{:.2}", snapshot.bcs_layer.effective_gain),
                        &format!(
                            "pitch {:?} unsafe {}",
                            snapshot.bcs_layer.pitch_note, snapshot.bcs_layer.unsafe_events
                        ),
                        220.0,
                    );
                });

                if let Some(diagnostics) = snapshot.gfm_layer.diagnostics {
                    ui.add_space(8.0);
                    ui.label(epm_small(format!(
                        "GFM diagnostics: frame={} peak={:.3} strain={:.3} rupture={}/{} health h:{} s:{} q:{} r:{}",
                        diagnostics.frame_index,
                        diagnostics.peak_abs_output,
                        diagnostics.max_strain,
                        diagnostics.last_rupture_count,
                        diagnostics.max_rupture_count,
                        diagnostics.health.healthy,
                        diagnostics.health.suspect,
                        diagnostics.health.quarantined,
                        diagnostics.health.recovering
                    )));
                }
            } else {
                ui.label(epm_body("waiting for engine snapshot"));
            }
        });
    }

    pub(crate) fn render_engine_param_editor(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: Option<&EngineSnapshot>,
    ) {
        epm_frame(epm_panel()).show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.label(epm_eyebrow("DEEP PARAMETER EDITOR"));
                    ui.label(epm_subheading(engine_section_label(
                        self.engine_selected_section,
                    )));
                });
            });

            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                for section in ENGINE_PARAM_SECTIONS {
                    if ui
                        .add(epm_command_button(
                            engine_section_label(section),
                            false,
                            self.engine_selected_section == section,
                        ))
                        .clicked()
                    {
                        self.engine_selected_section = section;
                    }
                }
            });

            ui.add_space(12.0);
            egui::Grid::new("engine-param-editor")
                .num_columns(4)
                .striped(true)
                .spacing(egui::vec2(14.0, 8.0))
                .show(ui, |ui| {
                    ui.label(epm_small("parameter"));
                    ui.label(epm_small("value"));
                    ui.label(epm_small("control"));
                    ui.label(epm_small("range"));
                    ui.end_row();

                    let selected_section = self.engine_selected_section;
                    for spec in all_params()
                        .iter()
                        .filter(|spec| spec.section == selected_section)
                    {
                        self.render_engine_param_row(ui, snapshot, spec);
                    }
                });
        });
    }

    pub(crate) fn render_engine_param_row(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: Option<&EngineSnapshot>,
        spec: &ParamSpec,
    ) {
        let current = snapshot
            .and_then(|snapshot| direct_param_raw_value(snapshot, spec.id))
            .unwrap_or(spec.default);
        ui.label(epm_body(spec.name));
        ui.label(epm_value(format_param_value(spec.id, current)));

        match spec.unit {
            ParamUnit::Boolean => {
                let mut enabled = current >= 0.5;
                if ui.checkbox(&mut enabled, "").changed() {
                    self.apply_engine_param_value(spec.id, if enabled { 1.0 } else { 0.0 });
                }
            }
            ParamUnit::Indexed | ParamUnit::Semitones => {
                let mut value = current.round().clamp(spec.min, spec.max);
                if ui
                    .add(
                        egui::Slider::new(&mut value, spec.min..=spec.max)
                            .integer()
                            .show_value(false),
                    )
                    .changed()
                {
                    self.apply_engine_param_value(spec.id, value.round());
                }
            }
            ParamUnit::Hertz => {
                let mut value = current.clamp(spec.min, spec.max);
                if ui
                    .add(
                        egui::Slider::new(&mut value, spec.min..=spec.max)
                            .logarithmic(true)
                            .show_value(false),
                    )
                    .changed()
                {
                    self.apply_engine_param_value(spec.id, value);
                }
            }
            ParamUnit::Normalized
            | ParamUnit::Milliseconds
            | ParamUnit::Cents
            | ParamUnit::Decibels => {
                let mut value = current.clamp(spec.min, spec.max);
                if ui
                    .add(egui::Slider::new(&mut value, spec.min..=spec.max).show_value(false))
                    .changed()
                {
                    self.apply_engine_param_value(spec.id, value);
                }
            }
        }

        ui.label(epm_small(format!(
            "{}..{} {:?}",
            spec.min, spec.max, spec.unit
        )));
        ui.end_row();
    }

    pub(crate) fn apply_engine_param_value(&mut self, id: ParamId, value: f32) {
        let spec = param_spec(id);
        match self.session.set_direct_param(id, value) {
            Ok(()) => {
                self.last_status_message = Some(format!(
                    "{} -> {}",
                    spec.name,
                    format_param_value(id, value)
                ));
                self.last_snapshot_refresh = Instant::now()
                    .checked_sub(PERFORMANCE_UI_REFRESH)
                    .unwrap_or_else(Instant::now);
            }
            Err(error) => {
                self.last_status_message = Some(format!("{} failed: {error}", spec.name));
            }
        }
    }
}

fn engine_section_label(section: ParamSection) -> &'static str {
    match section {
        ParamSection::Macro => "Macros",
        ParamSection::Oscillator1 => "Osc 1",
        ParamSection::Oscillator2 => "Osc 2",
        ParamSection::Spectral => "Spectral",
        ParamSection::Source => "Source",
        ParamSection::Sub => "Sub",
        ParamSection::Mixer => "Mixer",
        ParamSection::Filter => "Filter",
        ParamSection::AmpEnvelope => "Amp Env",
        ParamSection::FilterEnvelope => "Filter Env",
        ParamSection::Voice => "Voice",
        ParamSection::Performance => "Performance",
        ParamSection::FinalStage => "Final",
        ParamSection::Chorus => "Chorus",
        ParamSection::Reverb => "Reverb",
    }
}
