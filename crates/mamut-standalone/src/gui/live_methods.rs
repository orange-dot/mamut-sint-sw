use super::*;

impl PerformanceApp {
    pub(crate) fn render_header(&mut self, ui: &mut egui::Ui) {
        let (patch_name, description, favorite) = self
            .snapshot
            .as_ref()
            .map(|snapshot| {
                (
                    snapshot.patch_name.clone(),
                    snapshot
                        .patch_description
                        .clone()
                        .unwrap_or_else(|| "no description".to_string()),
                    snapshot.patch_favorite,
                )
            })
            .unwrap_or_else(|| {
                (
                    "EPM1 Performance".to_string(),
                    "waiting for engine snapshot".to_string(),
                    false,
                )
            });
        let current_slot = self
            .session
            .current_live_slot()
            .map(|slot| slot.to_string())
            .unwrap_or_else(|| "-".to_string());
        let midi_color = if Instant::now() <= self.midi_hot_until {
            epm_ok()
        } else {
            epm_muted()
        };
        let transport = self.session.transport_metrics_snapshot();

        epm_frame(epm_panel()).show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.label(epm_eyebrow("CURRENT PATCH"));
                    ui.label(epm_heading(patch_name));
                    ui.label(epm_body(description));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    let input = self.session.input_metrics_snapshot();
                    render_metric_tile_sized(
                        ui,
                        false,
                        "TRANSPORT",
                        &format!("{}", transport.queued_frames),
                        &format!(
                            "target {} / xruns {}",
                            transport.queue_target_frames, transport.xrun_recoveries
                        ),
                        260.0,
                    );
                    render_metric_tile_sized(
                        ui,
                        Instant::now() <= self.midi_hot_until,
                        "MIDI",
                        &format!("{}", input.midi_messages),
                        &format!(
                            "acc {} / drop {}",
                            input.midi_messages_accepted,
                            input.midi_messages_dropped + input.runtime_controls_dropped
                        ),
                        330.0,
                    );
                    render_metric_tile_sized(
                        ui,
                        favorite,
                        "LIVE SLOT",
                        &current_slot,
                        &format!("0..{}", LIVE_SET_STEMS.len().saturating_sub(1)),
                        140.0,
                    );
                });
            });

            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(
                    epm_muted(),
                    format!(
                        "Audio: {} @ {} Hz / {} ch",
                        self.session.audio_device_name,
                        self.session.sample_rate_hz,
                        self.session.channels
                    ),
                );
                if let Some(message) = &self.last_status_message {
                    ui.colored_label(epm_orange(), message);
                }
                if let Some(error) = &self.last_snapshot_error {
                    ui.colored_label(epm_bad(), error);
                }
                ui.colored_label(midi_color, "midi");
            });
        });
    }

    pub(crate) fn render_performance_status_strip(&mut self, ui: &mut egui::Ui) {
        let transport = self.session.transport_metrics_snapshot();
        let input = self.session.input_metrics_snapshot();
        let recording = self.session.recording_metrics_snapshot();
        let (voices, peak, clip, macros) = self
            .snapshot
            .as_ref()
            .map(|snapshot| {
                (
                    snapshot.active_voice_count,
                    snapshot.peak_output,
                    snapshot.clip_detected,
                    Some(snapshot.effective_macros),
                )
            })
            .unwrap_or((0, 0.0, false, None));
        let midi = self.session.midi_selector.as_deref().unwrap_or("no midi");
        let midi_channel = self
            .session
            .midi_channel
            .map(|channel| channel.to_string())
            .unwrap_or_else(|| "all".to_string());

        epm_frame(epm_panel_deep()).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                render_metric_tile_sized(
                    ui,
                    false,
                    "PATCH",
                    &self.session.patch_name,
                    "current",
                    245.0,
                );
                render_metric_tile_sized(
                    ui,
                    false,
                    "AUDIO",
                    &format!("{} Hz", self.session.sample_rate_hz),
                    &self.session.audio_device_name,
                    325.0,
                );
                render_metric_tile_sized(
                    ui,
                    Instant::now() <= self.midi_hot_until,
                    "MIDI",
                    &format!("ch {midi_channel}"),
                    midi,
                    175.0,
                );
                render_metric_tile_sized(
                    ui,
                    voices > 0,
                    "VOICES",
                    &voices.to_string(),
                    &format!("peak {peak:.3} clip {}", on_off_bool(clip)),
                    205.0,
                );
                render_metric_tile_sized(
                    ui,
                    transport.xrun_recoveries > 0,
                    "XRUN",
                    &transport.xrun_recoveries.to_string(),
                    &format!(
                        "und {} / {}f",
                        transport.underrun_batches, transport.underrun_frames
                    ),
                    205.0,
                );
                render_metric_tile_sized(
                    ui,
                    recording.state == RecordingState::Active,
                    "REC",
                    recording.state.label(),
                    &format!("drop {}", recording.frames_dropped),
                    130.0,
                );
                render_metric_tile_sized(
                    ui,
                    input.midi_messages_dropped > 0 || input.runtime_controls_dropped > 0,
                    "MIDI IN",
                    &input.midi_messages_accepted.to_string(),
                    &format!(
                        "rx {} drop {}",
                        input.midi_messages,
                        input.midi_messages_dropped + input.runtime_controls_dropped
                    ),
                    165.0,
                );
                render_metric_tile_sized(
                    ui,
                    input.trace_records_dropped > 0,
                    "TRACE",
                    &input.trace_records_dropped.to_string(),
                    &format!("coal {}", input.controllers_coalesced),
                    130.0,
                );
                if let Some(macros) = macros {
                    render_metric_tile_sized(
                        ui,
                        false,
                        "G/B/H",
                        &format!(
                            "{:.2}/{:.2}/{:.2}",
                            macros.gravitacija, macros.bloom, macros.heat
                        ),
                        &format!("R {:.2} S {:.2}", macros.ruin, macros.swarm),
                        210.0,
                    );
                }
            });
        });
    }

    pub(crate) fn render_gfm_layer_panel(&mut self, ui: &mut egui::Ui) {
        let snapshot = self.snapshot.as_ref().map(|snapshot| snapshot.gfm_layer);
        let current_seed = self.session.gfm_layer_seed;
        let mut enabled = current_seed.is_some();

        epm_frame(epm_panel()).show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.label(epm_eyebrow("GFM LAYER"));
                    if let Some(snapshot) = snapshot {
                        ui.label(epm_value(match snapshot.mode {
                            GfmLayerMode::Enabled { .. } => "READY",
                            GfmLayerMode::Disabled => "OFF",
                        }));
                    } else {
                        ui.label(epm_value("WAITING"));
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    if ui
                        .add_enabled(true, epm_command_button("TAKE PRESET", false, false))
                        .clicked()
                    {
                        self.apply_live_take_preset();
                    }
                    if ui
                        .add_enabled(enabled, epm_command_button("APPLY SEED", false, false))
                        .clicked()
                    {
                        self.apply_gfm_seed_from_ui(true);
                    }
                    if ui.checkbox(&mut enabled, "enabled").changed() {
                        self.apply_gfm_seed_from_ui(enabled);
                    }
                    ui.add(
                        egui::TextEdit::singleline(&mut self.gfm_seed_text)
                            .desired_width(138.0)
                            .font(egui::TextStyle::Monospace),
                    );
                });
            });
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                if let Some(snapshot) = snapshot {
                    let seed = match snapshot.mode {
                        GfmLayerMode::Enabled { seed } => format_gfm_seed(seed),
                        GfmLayerMode::Disabled => "off".to_string(),
                    };
                    render_metric_tile(
                        ui,
                        matches!(snapshot.mode, GfmLayerMode::Enabled { .. }),
                        "MODE",
                        match snapshot.mode {
                            GfmLayerMode::Enabled { .. } => "enabled",
                            GfmLayerMode::Disabled => "disabled",
                        },
                        &seed,
                    );
                    render_metric_tile(
                        ui,
                        snapshot.effective_amount > 0.001,
                        "MIDI",
                        &format!("{:.2}", snapshot.effective_amount),
                        &format!(
                            "K8 gate {:.2} AT amt {:.2}",
                            snapshot.pressure, snapshot.amount
                        ),
                    );
                    render_metric_tile(
                        ui,
                        false,
                        "SELECTED",
                        &format_optional_program_id(snapshot.selection.program_id),
                        "program",
                    );
                    render_metric_tile(
                        ui,
                        false,
                        "ACTIVE",
                        &format_optional_program_id(snapshot.active_program_id),
                        "program",
                    );
                    render_metric_tile(
                        ui,
                        false,
                        "SCORES",
                        &format!(
                            "{:.2}/{:.2}/{:.2}",
                            snapshot.selection.horizont_score,
                            snapshot.selection.pec_score,
                            snapshot.selection.baklja_score
                        ),
                        "H/P/B",
                    );
                    render_metric_tile(
                        ui,
                        snapshot
                            .diagnostics
                            .is_some_and(|diagnostics| diagnostics.max_rupture_count > 0),
                        "RUPTURES",
                        &snapshot
                            .diagnostics
                            .map(|diagnostics| diagnostics.max_rupture_count.to_string())
                            .unwrap_or_else(|| "-".to_string()),
                        "max",
                    );
                } else {
                    render_metric_tile(ui, false, "MODE", "unknown", "snapshot pending");
                }
            });
        });
    }

    pub(crate) fn render_bcs_layer_panel(&mut self, ui: &mut egui::Ui) {
        let snapshot = self.snapshot.as_ref().map(|snapshot| snapshot.bcs_layer);
        let mut enabled = self.session.bcs_layer_scenario.is_some();

        epm_frame(epm_panel()).show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.label(epm_eyebrow("BCS LAYER"));
                    if let Some(snapshot) = snapshot {
                        ui.label(epm_value(match snapshot.mode {
                            BcsLayerMode::Enabled { .. } => "READY",
                            BcsLayerMode::Disabled => "OFF",
                        }));
                    } else {
                        ui.label(epm_value("WAITING"));
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    if ui.checkbox(&mut enabled, "enabled").changed() {
                        self.apply_bcs_scenario_from_ui(enabled);
                    }
                });
            });
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                for scenario in BcsScenario::ALL {
                    let selected = self.bcs_selected_scenario == scenario;
                    let label = format_bcs_scenario(scenario).to_ascii_uppercase();
                    if ui
                        .add(epm_command_button(&label, false, selected))
                        .clicked()
                    {
                        self.bcs_selected_scenario = scenario;
                        if enabled {
                            self.apply_bcs_scenario_from_ui(true);
                        }
                    }
                }
            });
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                if let Some(snapshot) = snapshot {
                    let scenario = match snapshot.mode {
                        BcsLayerMode::Enabled { scenario } => format_bcs_scenario(scenario),
                        BcsLayerMode::Disabled => "off",
                    };
                    let pitch = snapshot
                        .pitch_note
                        .map(|note| {
                            format!(
                                "{} / {:.2}Hz",
                                note,
                                snapshot.pitch_frequency_hz.unwrap_or_default()
                            )
                        })
                        .unwrap_or_else(|| "-".to_string());
                    render_metric_tile(
                        ui,
                        matches!(snapshot.mode, BcsLayerMode::Enabled { .. }),
                        "MODE",
                        match snapshot.mode {
                            BcsLayerMode::Enabled { .. } => "enabled",
                            BcsLayerMode::Disabled => "disabled",
                        },
                        scenario,
                    );
                    render_metric_tile(
                        ui,
                        snapshot.effective_gain > 0.001,
                        "GAIN",
                        &format!("{:.2}", snapshot.effective_gain),
                        &format!(
                            "SW9 {} S9 {:.2}",
                            on_off_bool(snapshot.enabled),
                            snapshot.gain
                        ),
                    );
                    render_metric_tile(ui, snapshot.pitch_note.is_some(), "PITCH", &pitch, "note");
                    render_metric_tile(
                        ui,
                        snapshot.unsafe_state || snapshot.unsafe_events > 0,
                        "STATE",
                        &format!("{:.3}", snapshot.max_state_abs),
                        &format!(
                            "unsafe {} / {}",
                            on_off_bool(snapshot.unsafe_state),
                            snapshot.unsafe_events
                        ),
                    );
                } else {
                    render_metric_tile(ui, false, "MODE", "unknown", "snapshot pending");
                }
            });
        });
    }

    pub(crate) fn render_slot_buttons(&mut self, ui: &mut egui::Ui) {
        epm_frame(epm_panel_deep()).show(ui, |ui| {
            ui.label(epm_eyebrow("LIVE SET"));
            ui.add_space(4.0);
            egui::Grid::new("live-set-slots")
                .num_columns(4)
                .spacing([8.0, 8.0])
                .show(ui, |ui| {
                    for (slot, stem) in LIVE_SET_STEMS.iter().enumerate() {
                        let selected = self.session.current_live_slot() == Some(slot);
                        let label = egui::RichText::new(format!("{slot:02}\n{stem}"))
                            .monospace()
                            .size(12.0)
                            .strong()
                            .color(if selected { epm_text() } else { epm_muted() });
                        let button = egui::Button::new(label)
                            .fill(if selected { epm_tile_hot() } else { epm_tile() })
                            .stroke(if selected {
                                epm_accent_stroke()
                            } else {
                                epm_stroke()
                            })
                            .corner_radius(egui::CornerRadius::same(4));
                        if ui.add_sized([118.0, 46.0], button).clicked() {
                            self.run_action(
                                |session| session.load_favorite_slot(slot),
                                &format!("live slot {slot}"),
                            );
                        }
                        if slot % 4 == 3 {
                            ui.end_row();
                        }
                    }
                });
        });
    }

    pub(crate) fn render_factory_bank(&mut self, ui: &mut egui::Ui) {
        epm_frame(epm_panel_deep()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(epm_eyebrow("FACTORY BANK"));
                ui.colored_label(epm_muted(), "manual patch selection");
            });
            ui.add_space(6.0);

            if let Some(error) = &self.factory_patch_error {
                ui.colored_label(epm_bad(), error);
                return;
            }

            let current_stem = self
                .session
                .patch_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map(str::to_string);
            let entries = self.factory_patches.clone();

            egui::Grid::new("factory-bank-patches")
                .num_columns(3)
                .spacing([8.0, 8.0])
                .show(ui, |ui| {
                    for (index, entry) in entries.iter().enumerate() {
                        let selected = current_stem.as_deref() == Some(entry.stem.as_str());
                        let label =
                            egui::RichText::new(format!("{}\n{}", entry.patch_name, entry.stem))
                                .monospace()
                                .size(12.0)
                                .strong()
                                .color(if selected { epm_text() } else { epm_muted() });
                        let button = egui::Button::new(label)
                            .fill(if selected { epm_tile_hot() } else { epm_tile() })
                            .stroke(if selected {
                                epm_accent_stroke()
                            } else {
                                epm_stroke()
                            })
                            .corner_radius(egui::CornerRadius::same(4));
                        if ui.add_sized([190.0, 50.0], button).clicked() {
                            let path = entry.path.clone();
                            self.run_action(
                                |session| session.switch_patch(path),
                                &format!("factory patch {}", entry.stem),
                            );
                        }
                        if index % 3 == 2 {
                            ui.end_row();
                        }
                    }
                });
        });
    }

    pub(crate) fn render_macro_meter(
        ui: &mut egui::Ui,
        label: &str,
        live_value: f32,
        effective_value: f32,
    ) {
        epm_tile_frame(false).show(ui, |ui| {
            ui.set_min_width(150.0);
            ui.label(epm_eyebrow(label.to_ascii_uppercase()));
            ui.add(
                egui::ProgressBar::new(live_value.clamp(0.0, 1.0))
                    .desired_width(132.0)
                    .fill(epm_orange())
                    .text(format!("{live_value:.2}")),
            );
            ui.label(epm_small(format!("effective {effective_value:.2}")));
        });
    }

    pub(crate) fn render_macro_controls(&mut self, ui: &mut egui::Ui) {
        let Some(snapshot) = self.snapshot.as_ref() else {
            epm_frame(epm_panel()).show(ui, |ui| {
                ui.label(epm_eyebrow("MACROS"));
                ui.label(epm_body("waiting for engine snapshot"));
            });
            return;
        };

        let macro_values = [
            (
                "Gravitacija",
                snapshot.live_macros.gravitacija,
                snapshot.effective_macros.gravitacija,
            ),
            (
                "Bloom",
                snapshot.live_macros.bloom,
                snapshot.effective_macros.bloom,
            ),
            (
                "Heat",
                snapshot.live_macros.heat,
                snapshot.effective_macros.heat,
            ),
            (
                "Ruin",
                snapshot.live_macros.ruin,
                snapshot.effective_macros.ruin,
            ),
            (
                "Swarm",
                snapshot.live_macros.swarm,
                snapshot.effective_macros.swarm,
            ),
        ];

        epm_frame(epm_panel()).show(ui, |ui| {
            ui.label(epm_eyebrow("MACROS"));
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                for (label, live_value, effective_value) in macro_values {
                    Self::render_macro_meter(ui, label, live_value, effective_value);
                }
            });
        });
    }

    pub(crate) fn render_tabs(&mut self, ui: &mut egui::Ui) {
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), 54.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                let logo = egui::Button::new(
                    egui::RichText::new("M")
                        .monospace()
                        .size(18.0)
                        .strong()
                        .color(epm_orange()),
                )
                .fill(epm_panel_deep())
                .stroke(epm_stroke())
                .corner_radius(egui::CornerRadius::same(4));
                ui.add_sized([34.0, 40.0], logo);

                ui.allocate_ui_with_layout(
                    egui::vec2(190.0, 42.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.label(epm_small("CURRENT BUILD"));
                        ui.label(epm_subheading("Mamut EPM"));
                    },
                );

                let tab_count = PerformanceTab::ALL.len() as f32;
                let tab_button_width = 92.0;
                let tab_width =
                    tab_count * tab_button_width + (tab_count - 1.0) * ui.spacing().item_spacing.x;
                ui.add_space((ui.available_width() - tab_width).max(12.0));

                for tab in PerformanceTab::ALL {
                    let selected = self.selected_tab == tab;
                    let button = egui::Button::new(
                        egui::RichText::new(tab.label().to_ascii_uppercase())
                            .monospace()
                            .size(12.0)
                            .strong()
                            .color(if selected { epm_text() } else { epm_muted() }),
                    )
                    .fill(if selected { epm_tile_hot() } else { epm_bg() })
                    .stroke(if selected {
                        epm_accent_stroke()
                    } else {
                        epm_stroke()
                    })
                    .corner_radius(egui::CornerRadius::same(0));
                    if ui.add_sized([tab_button_width, 38.0], button).clicked() {
                        self.selected_tab = tab;
                    }
                }
            },
        );
    }

    pub(crate) fn render_live_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        self.render_header(ui);
        ui.add_space(12.0);
        self.render_performance_status_strip(ui);
        ui.add_space(12.0);
        self.render_gfm_layer_panel(ui);
        ui.add_space(12.0);
        self.render_bcs_layer_panel(ui);
        ui.add_space(12.0);
        self.render_factory_bank(ui);
        ui.add_space(12.0);
        self.render_output_capture_controls(ui);
        ui.add_space(12.0);
        ui.columns(2, |columns| {
            self.render_slot_buttons(&mut columns[0]);
            self.render_macro_controls(&mut columns[1]);
        });
        ui.add_space(12.0);
        self.render_footer(ui, ctx);
    }
}
