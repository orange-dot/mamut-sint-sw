use super::*;

impl PerformanceApp {
    pub(crate) fn render_debug_tab(&mut self, ui: &mut egui::Ui) {
        let input = self.session.input_metrics_snapshot();
        epm_frame(epm_panel()).show(ui, |ui| {
            ui.label(epm_eyebrow("DEBUG"));
            ui.add_space(6.0);
            egui::Grid::new("debug-runtime-grid")
                .num_columns(2)
                .spacing([18.0, 8.0])
                .show(ui, |ui| {
                    ui.label(epm_small("DRIVER"));
                    ui.label(epm_body(self.session.driver.detail()));
                    ui.end_row();

                    ui.label(epm_small("CHANNEL"));
                    match self.session.midi_channel {
                        Some(channel) => ui.label(epm_body(format!("channel {channel}"))),
                        None => ui.label(epm_body("all channels")),
                    };
                    ui.end_row();

                    ui.label(epm_small("PROFILE"));
                    if let Some(profile) = &self.session.controller_profile {
                        ui.label(epm_body(format!(
                            "{} ({})",
                            profile.name,
                            profile.path.display()
                        )));
                    } else {
                        ui.label(epm_body("legacy live map"));
                    }
                    ui.end_row();

                    ui.label(epm_small("MESSAGES"));
                    ui.label(epm_value(input.midi_messages.to_string()));
                    ui.end_row();

                    ui.label(epm_small("ACCEPTED"));
                    ui.label(epm_value(input.midi_messages_accepted.to_string()));
                    ui.end_row();

                    ui.label(epm_small("DROPS"));
                    ui.label(epm_body(format!(
                        "midi {} / runtime {} / trace {}",
                        input.midi_messages_dropped,
                        input.runtime_controls_dropped,
                        input.trace_records_dropped
                    )));
                    ui.end_row();

                    ui.label(epm_small("COALESCED"));
                    ui.label(epm_value(input.controllers_coalesced.to_string()));
                    ui.end_row();
                });
        });

        ui.add_space(12.0);
        if let Some(event) = &input.last_control {
            epm_frame(epm_panel_deep()).show(ui, |ui| {
                ui.label(epm_eyebrow("LATEST MIDI"));
                ui.label(epm_value(format!("{} -> {}", event.label, event.action)));
                ui.label(epm_small(format!(
                    "status 0x{:02X} / channel {} / {}",
                    event.raw_status,
                    event
                        .channel
                        .map(|channel| channel.to_string())
                        .unwrap_or_else(|| "system".to_string()),
                    verdict_label(event.verdict)
                )));
                if let Some(value) = event.raw_value {
                    ui.label(epm_body(format!("raw value {value:.3}")));
                }
                if let Some(program) = event.program {
                    ui.label(epm_body(format!("program {program}")));
                }
            });
        }

        ui.add_space(12.0);
        epm_frame(epm_panel()).show(ui, |ui| {
            ui.label(epm_eyebrow("RUNTIME"));
            let transport = self.session.transport_metrics_snapshot();
            egui::Grid::new("debug-transport-grid")
                .num_columns(2)
                .spacing([18.0, 8.0])
                .show(ui, |ui| {
                    ui.label(epm_small("ALSA QUEUE"));
                    ui.label(epm_body(format!(
                        "queued {} / target {} / write {}",
                        transport.queued_frames,
                        transport.queue_target_frames,
                        transport.write_frames_hint
                    )));
                    ui.end_row();

                    ui.label(epm_small("UNDERRUNS"));
                    ui.label(epm_body(format!(
                        "batches {} / frames {}",
                        transport.underrun_batches, transport.underrun_frames
                    )));
                    ui.end_row();

                    ui.label(epm_small("XRUNS"));
                    ui.label(epm_body(transport.xrun_recoveries.to_string()));
                    ui.end_row();

                    ui.label(epm_small("OVERFLOWS"));
                    ui.label(epm_body(format!(
                        "batches {} / frames {}",
                        transport.overflow_batches, transport.overflow_frames
                    )));
                    ui.end_row();
                });
            if let Some(message) = &self.last_status_message {
                ui.add_space(6.0);
                ui.label(epm_body(format!("runtime status: {message}")));
            }
            if let Some(error) = &self.last_snapshot_error {
                ui.add_space(6.0);
                ui.colored_label(epm_bad(), error);
            }
        });
    }

    pub(crate) fn render_footer(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        epm_frame(epm_panel_deep()).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.add(epm_command_button("PREV", false, false)).clicked() {
                    self.run_action(|session| session.switch_favorite(-1), "previous live slot");
                }
                if ui.add(epm_command_button("NEXT", false, false)).clicked() {
                    self.run_action(|session| session.switch_favorite(1), "next live slot");
                }
                if ui.add(epm_command_button("PANIC", true, false)).clicked() {
                    self.run_action(|session| session.panic(), "panic");
                }
                if ui
                    .add(epm_command_button("RESET CTRLS", false, false))
                    .clicked()
                {
                    self.run_action(|session| session.reset_controllers(), "reset controllers");
                }
                if ui.add(epm_command_button("QUIT", false, false)).clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            if let Some(snapshot) = &self.snapshot {
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    render_metric_tile(
                        ui,
                        snapshot.active_voice_count > 0,
                        "VOICES",
                        &snapshot.active_voice_count.to_string(),
                        &format!("held {:?}", snapshot.held_notes),
                    );
                    render_metric_tile(
                        ui,
                        snapshot.sustain_down,
                        "SUSTAIN",
                        if snapshot.sustain_down { "down" } else { "up" },
                        "",
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
            }
        });
    }
}
