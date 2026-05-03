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

pub(crate) struct PerformanceApp {
    pub(crate) session: RuntimeSession,
    pub(crate) snapshot: Option<EngineSnapshot>,
    pub(crate) factory_patches: Vec<FactoryPatchEntry>,
    pub(crate) factory_patch_error: Option<String>,
    pub(crate) selected_tab: PerformanceTab,
    pub(crate) sound_lab_page: SoundLabPage,
    pub(crate) gfm_seed_text: String,
    pub(crate) bcs_selected_scenario: BcsScenario,
    pub(crate) record_duration_preset: RecordDurationPreset,
    pub(crate) record_custom_seconds: String,
    pub(crate) take_tag: String,
    pub(crate) sound_lab_export_name: String,
    pub(crate) last_snapshot_error: Option<String>,
    pub(crate) last_status_message: Option<String>,
    pub(crate) last_snapshot_refresh: Instant,
    pub(crate) last_midi_message_count: u64,
    pub(crate) midi_hot_until: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PerformanceTab {
    Live,
    SoundLab,
    Pc4,
    Debug,
}

impl PerformanceTab {
    pub(crate) const ALL: [Self; 4] = [Self::Live, Self::SoundLab, Self::Pc4, Self::Debug];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Live => "Live",
            Self::SoundLab => "Sound Lab",
            Self::Pc4 => "PC4",
            Self::Debug => "Debug",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecordDurationPreset {
    Thirty,
    Sixty,
    Custom,
}

impl RecordDurationPreset {
    pub(crate) fn seconds(self, custom_seconds: &str) -> Result<u64> {
        match self {
            Self::Thirty => Ok(30),
            Self::Sixty => Ok(60),
            Self::Custom => {
                let seconds = custom_seconds
                    .trim()
                    .parse::<u64>()
                    .with_context(|| format!("invalid record duration `{custom_seconds}`"))?;
                if seconds == 0 {
                    return Err(anyhow!("record duration must be greater than zero seconds"));
                }
                Ok(seconds)
            }
        }
    }
}

impl PerformanceApp {
    pub(crate) fn new(session: RuntimeSession) -> Self {
        let snapshot = session.request_snapshot().ok();
        let last_midi_message_count = session.input_metrics_snapshot().midi_messages;
        let gfm_seed_text = session
            .gfm_layer_seed
            .map(format_gfm_seed)
            .unwrap_or_else(|| format_gfm_seed(DEFAULT_GFM_UI_SEED));
        let bcs_selected_scenario = session
            .bcs_layer_scenario
            .unwrap_or(BcsScenario::SubharmonicPressure);
        let (factory_patches, factory_patch_error) = match factory_patch_entries() {
            Ok(mut entries) => {
                entries.sort_by_key(|entry| {
                    (
                        LIVE_SET_STEMS.contains(&entry.stem.as_str()),
                        entry.stem.clone(),
                    )
                });
                (entries, None)
            }
            Err(error) => (Vec::new(), Some(error.to_string())),
        };
        Self {
            session,
            snapshot,
            factory_patches,
            factory_patch_error,
            selected_tab: PerformanceTab::Live,
            sound_lab_page: SoundLabPage::Osc1,
            gfm_seed_text,
            bcs_selected_scenario,
            record_duration_preset: RecordDurationPreset::Thirty,
            record_custom_seconds: DEFAULT_LIVE_TAKE_SECONDS.to_string(),
            take_tag: DEFAULT_LIVE_TAKE_TAG.to_string(),
            sound_lab_export_name: String::new(),
            last_snapshot_error: None,
            last_status_message: None,
            last_snapshot_refresh: Instant::now()
                .checked_sub(PERFORMANCE_UI_REFRESH)
                .unwrap_or_else(Instant::now),
            last_midi_message_count,
            midi_hot_until: Instant::now(),
        }
    }

    pub(crate) fn poll_runtime(&mut self) {
        match self.session.poll_runtime_control_messages() {
            Ok(messages) => {
                if let Some(message) = messages.last() {
                    self.last_status_message = Some(message.clone());
                }
            }
            Err(error) => {
                self.last_status_message = Some(format!("runtime control error: {error}"));
            }
        }

        let input = self.session.input_metrics_snapshot();
        if input.midi_messages != self.last_midi_message_count {
            self.last_midi_message_count = input.midi_messages;
            self.midi_hot_until = Instant::now() + MIDI_ACTIVITY_FLASH;
        }

        if self.last_snapshot_refresh.elapsed() >= PERFORMANCE_UI_REFRESH {
            match self.session.request_snapshot() {
                Ok(snapshot) => {
                    self.sync_gfm_seed_from_snapshot(&snapshot);
                    self.sync_bcs_scenario_from_snapshot(&snapshot);
                    self.snapshot = Some(snapshot);
                    self.last_snapshot_error = None;
                }
                Err(error) => {
                    self.last_snapshot_error = Some(error.to_string());
                }
            }
            self.last_snapshot_refresh = Instant::now();
        }
    }

    pub(crate) fn sync_gfm_seed_from_snapshot(&mut self, snapshot: &EngineSnapshot) {
        let snapshot_seed = match snapshot.gfm_layer.mode {
            GfmLayerMode::Enabled { seed } => Some(seed),
            GfmLayerMode::Disabled => None,
        };
        if self.session.gfm_layer_seed == snapshot_seed {
            return;
        }
        self.session.gfm_layer_seed = snapshot_seed;
        if let Some(seed) = snapshot_seed {
            self.gfm_seed_text = format_gfm_seed(seed);
        }
    }

    pub(crate) fn sync_bcs_scenario_from_snapshot(&mut self, snapshot: &EngineSnapshot) {
        let snapshot_scenario = match snapshot.bcs_layer.mode {
            BcsLayerMode::Enabled { scenario } => Some(scenario),
            BcsLayerMode::Disabled => None,
        };
        if self.session.bcs_layer_scenario == snapshot_scenario {
            return;
        }
        self.session.bcs_layer_scenario = snapshot_scenario;
        if let Some(scenario) = snapshot_scenario {
            self.bcs_selected_scenario = scenario;
        }
    }

    pub(crate) fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.input(|input| {
            if input.key_pressed(egui::Key::ArrowLeft) {
                self.run_action(|session| session.switch_favorite(-1), "previous live slot");
            }
            if input.key_pressed(egui::Key::ArrowRight) {
                self.run_action(|session| session.switch_favorite(1), "next live slot");
            }
            if input.key_pressed(egui::Key::P) {
                self.run_action(|session| session.panic(), "panic");
            }
            if input.key_pressed(egui::Key::R) {
                self.run_action(|session| session.reset_controllers(), "reset controllers");
            }
            for (key, slot) in [
                (egui::Key::Num0, 0_usize),
                (egui::Key::Num1, 1),
                (egui::Key::Num2, 2),
                (egui::Key::Num3, 3),
                (egui::Key::Num4, 4),
                (egui::Key::Num5, 5),
                (egui::Key::Num6, 6),
                (egui::Key::Num7, 7),
            ] {
                if input.key_pressed(key) {
                    self.run_action(
                        |session| session.load_favorite_slot(slot),
                        &format!("live slot {slot}"),
                    );
                }
            }
        });
    }

    pub(crate) fn run_action<F>(&mut self, action: F, label: &str) -> bool
    where
        F: FnOnce(&mut RuntimeSession) -> Result<()>,
    {
        match action(&mut self.session) {
            Ok(()) => {
                self.last_status_message = Some(format!("{label} ok"));
                true
            }
            Err(error) => {
                self.last_status_message = Some(format!("{label} failed: {error}"));
                false
            }
        }
    }

    pub(crate) fn selected_record_seconds(&self) -> Result<u64> {
        self.record_duration_preset
            .seconds(&self.record_custom_seconds)
    }

    pub(crate) fn apply_gfm_seed_from_ui(&mut self, enabled: bool) {
        let seed = match gfm_layer_seed_from_ui_text(enabled, &self.gfm_seed_text) {
            Ok(seed) => seed,
            Err(error) => {
                self.last_status_message = Some(format!("gfm seed failed: {error}"));
                return;
            }
        };
        if self.run_action(
            |session| session.set_gfm_layer_seed(seed).map(|_| ()),
            if enabled { "gfm enable" } else { "gfm disable" },
        ) {
            if let Some(seed) = seed {
                self.gfm_seed_text = format_gfm_seed(seed);
            }
            self.last_snapshot_refresh = Instant::now()
                .checked_sub(PERFORMANCE_UI_REFRESH)
                .unwrap_or_else(Instant::now);
        }
    }

    pub(crate) fn apply_bcs_scenario_from_ui(&mut self, enabled: bool) {
        let scenario = enabled.then_some(self.bcs_selected_scenario);
        if self.run_action(
            |session| session.set_bcs_layer_scenario(scenario).map(|_| ()),
            if enabled { "bcs enable" } else { "bcs disable" },
        ) {
            self.last_snapshot_refresh = Instant::now()
                .checked_sub(PERFORMANCE_UI_REFRESH)
                .unwrap_or_else(Instant::now);
        }
    }

    pub(crate) fn apply_live_take_preset(&mut self) {
        self.take_tag = DEFAULT_LIVE_TAKE_TAG.to_string();
        self.record_duration_preset = RecordDurationPreset::Thirty;
        self.record_custom_seconds = DEFAULT_LIVE_TAKE_SECONDS.to_string();
        self.gfm_seed_text = format_gfm_seed(DEFAULT_GFM_UI_SEED);
        let path = match resolve_patch_argument(Some("cathedral-bloom")) {
            Ok(path) => path,
            Err(error) => {
                self.last_status_message = Some(format!("preset failed: {error}"));
                return;
            }
        };
        if !self.run_action(
            |session| session.switch_patch(path),
            "preset patch cathedral-bloom",
        ) {
            return;
        }
        self.apply_gfm_seed_from_ui(true);
    }

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
                    render_metric_tile(
                        ui,
                        false,
                        "TRANSPORT",
                        &format!("{}", transport.queued_frames),
                        &format!(
                            "target {} / xruns {}",
                            transport.queue_target_frames, transport.xrun_recoveries
                        ),
                    );
                    render_metric_tile(
                        ui,
                        Instant::now() <= self.midi_hot_until,
                        "MIDI",
                        &format!("{}", self.session.input_metrics_snapshot().midi_messages),
                        &self.session.driver.detail(),
                    );
                    render_metric_tile(
                        ui,
                        favorite,
                        "LIVE SLOT",
                        &current_slot,
                        &format!("0..{}", LIVE_SET_STEMS.len().saturating_sub(1)),
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
                render_metric_tile(ui, false, "PATCH", &self.session.patch_name, "current");
                render_metric_tile(
                    ui,
                    false,
                    "AUDIO",
                    &format!("{} Hz", self.session.sample_rate_hz),
                    &self.session.audio_device_name,
                );
                render_metric_tile(
                    ui,
                    Instant::now() <= self.midi_hot_until,
                    "MIDI",
                    &format!("ch {midi_channel}"),
                    midi,
                );
                render_metric_tile(
                    ui,
                    voices > 0,
                    "VOICES",
                    &voices.to_string(),
                    &format!("peak {peak:.3} clip {}", on_off_bool(clip)),
                );
                render_metric_tile(
                    ui,
                    transport.xrun_recoveries > 0,
                    "XRUN",
                    &transport.xrun_recoveries.to_string(),
                    &format!(
                        "und {} / {}f",
                        transport.underrun_batches, transport.underrun_frames
                    ),
                );
                render_metric_tile(
                    ui,
                    recording.state == RecordingState::Active,
                    "REC",
                    recording.state.label(),
                    &format!("drop {}", recording.frames_dropped),
                );
                render_metric_tile(ui, false, "MIDI IN", &input.midi_messages.to_string(), "");
                if let Some(macros) = macros {
                    render_metric_tile(
                        ui,
                        false,
                        "G/B/H",
                        &format!(
                            "{:.2}/{:.2}/{:.2}",
                            macros.gravitacija, macros.bloom, macros.heat
                        ),
                        &format!("R {:.2} S {:.2}", macros.ruin, macros.swarm),
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

impl eframe::App for PerformanceApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_shortcuts(ctx);
        self.poll_runtime();

        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(epm_bg())
                    .inner_margin(egui::Margin::symmetric(18, 14)),
            )
            .show(ctx, |ui| {
                self.render_tabs(ui);
                ui.add_space(14.0);
                match self.selected_tab {
                    PerformanceTab::Pc4 => self.render_pc4_tab(ui),
                    PerformanceTab::Live | PerformanceTab::SoundLab | PerformanceTab::Debug => {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| match self.selected_tab {
                                PerformanceTab::Live => self.render_live_tab(ui, ctx),
                                PerformanceTab::SoundLab => self.render_sound_lab_tab(ui, ctx),
                                PerformanceTab::Debug => self.render_debug_tab(ui),
                                PerformanceTab::Pc4 => unreachable!(),
                            });
                    }
                }
            });

        self.session.set_sound_lab_midi_focus(
            (self.selected_tab == PerformanceTab::SoundLab).then_some(self.sound_lab_page),
        );
        ctx.request_repaint_after(Duration::from_millis(16));
    }
}

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
    epm_tile_frame(highlighted).show(ui, |ui| {
        ui.set_min_width(112.0);
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
        ControllerBindingAction::Runtime(_) | ControllerBindingAction::Reserved => 0.0,
    }
    .clamp(0.0, 1.0)
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
        ControllerBindingAction::Runtime(_) => ("trigger".to_string(), None),
        ControllerBindingAction::ToggleParam(id) => {
            (toggle_param_display_value(snapshot, id), None)
        }
        ControllerBindingAction::Reserved => ("Reserved".to_string(), None),
    }
}

pub(crate) fn macro_snapshot_value(snapshot: &EngineSnapshot, id: MacroId, effective: bool) -> f32 {
    let macros = if effective {
        snapshot.effective_macros
    } else {
        snapshot.live_macros
    };
    match id {
        MacroId::Gravitacija => macros.gravitacija,
        MacroId::Bloom => macros.bloom,
        MacroId::Heat => macros.heat,
        MacroId::Ruin => macros.ruin,
        MacroId::Swarm => macros.swarm,
    }
}

pub(crate) fn toggle_param_display_value(snapshot: &EngineSnapshot, id: ParamId) -> String {
    match id {
        ParamId::ChorusEnabled => on_off(snapshot.direct.chorus_enabled),
        ParamId::ReverbEnabled => on_off(snapshot.direct.reverb_enabled),
        _ => "unsupported".to_string(),
    }
}

pub(crate) fn on_off(value: bool) -> String {
    if value { "on" } else { "off" }.to_string()
}

pub(crate) fn direct_param_raw_value(snapshot: &EngineSnapshot, id: ParamId) -> Option<f32> {
    let value = match id {
        ParamId::GravitacijaMacro => snapshot.live_macros.gravitacija,
        ParamId::BloomMacro => snapshot.live_macros.bloom,
        ParamId::HeatMacro => snapshot.live_macros.heat,
        ParamId::RuinMacro => snapshot.live_macros.ruin,
        ParamId::SwarmMacro => snapshot.live_macros.swarm,
        ParamId::Osc1SawLevel => snapshot.direct.osc1_wave_mix[0],
        ParamId::Osc1PulseLevel => snapshot.direct.osc1_wave_mix[1],
        ParamId::Osc1TriangleLevel => snapshot.direct.osc1_wave_mix[2],
        ParamId::Osc1NoiseLevel => snapshot.direct.osc1_wave_mix[3],
        ParamId::Osc1FineTuneCents => snapshot.direct.osc1_fine_tune_cents,
        ParamId::Osc1PulseWidth => snapshot.direct.osc1_pulse_width,
        ParamId::Osc1PwmDepth => snapshot.direct.osc1_pwm_depth,
        ParamId::Osc1PhaseMode => snapshot.direct.osc1_phase_mode,
        ParamId::Osc1StartPhase => snapshot.direct.osc1_start_phase,
        ParamId::Osc1SawBend => snapshot.direct.osc1_saw_bend,
        ParamId::Osc1TriangleFold => snapshot.direct.osc1_triangle_fold,
        ParamId::Osc1PulseEdge => snapshot.direct.osc1_pulse_edge,
        ParamId::Osc2SawLevel => snapshot.direct.osc2_wave_mix[0],
        ParamId::Osc2PulseLevel => snapshot.direct.osc2_wave_mix[1],
        ParamId::Osc2TriangleLevel => snapshot.direct.osc2_wave_mix[2],
        ParamId::Osc2IntervalSemitones => snapshot.direct.osc2_interval_semitones,
        ParamId::Osc2FineTuneCents => snapshot.direct.osc2_fine_tune_cents,
        ParamId::Osc2SyncAmount => snapshot.direct.sync_amount,
        ParamId::Osc2CrossmodAmount => snapshot.direct.crossmod_amount,
        ParamId::Osc2PulseWidth => snapshot.direct.osc2_pulse_width,
        ParamId::Osc2PwmDepth => snapshot.direct.osc2_pwm_depth,
        ParamId::Osc2PhaseMode => snapshot.direct.osc2_phase_mode,
        ParamId::Osc2StartPhase => snapshot.direct.osc2_start_phase,
        ParamId::Osc2Level => snapshot.direct.osc2_level,
        ParamId::Osc2PitchMode => snapshot.direct.osc2_pitch_mode,
        ParamId::Osc2Ratio => snapshot.direct.osc2_ratio,
        ParamId::Osc2SawBend => snapshot.direct.osc2_saw_bend,
        ParamId::Osc2TriangleFold => snapshot.direct.osc2_triangle_fold,
        ParamId::Osc2PulseEdge => snapshot.direct.osc2_pulse_edge,
        ParamId::SpectralLevel => snapshot.direct.spectral_level,
        ParamId::SpectralTable => snapshot.direct.spectral_table,
        ParamId::SpectralPosition => snapshot.direct.spectral_position,
        ParamId::SpectralMorph => snapshot.direct.spectral_morph,
        ParamId::SpectralRatio => snapshot.direct.spectral_ratio,
        ParamId::SpectralFineTuneCents => snapshot.direct.spectral_fine_tune_cents,
        ParamId::AdditiveLevel => snapshot.direct.additive_level,
        ParamId::AdditivePartialCount => snapshot.direct.additive_partial_count,
        ParamId::AdditiveHarmonicSpread => snapshot.direct.additive_harmonic_spread,
        ParamId::AdditiveOddEvenBalance => snapshot.direct.additive_odd_even_balance,
        ParamId::AdditiveInharmonicity => snapshot.direct.additive_inharmonicity,
        ParamId::AdditiveSpectralTilt => snapshot.direct.additive_spectral_tilt,
        ParamId::AdditiveRandomDetuneCents => snapshot.direct.additive_random_detune_cents,
        ParamId::SourcePwmRateHz => snapshot.direct.source_pwm_rate_hz,
        ParamId::NoiseColor => snapshot.direct.noise_color,
        ParamId::NoiseFilterLevel => snapshot.direct.noise_filter_level,
        ParamId::NoiseBodyLevel => snapshot.direct.noise_body_level,
        ParamId::AnalogDrift => snapshot.direct.analog_drift,
        ParamId::MicroJitter => snapshot.direct.micro_jitter,
        ParamId::FmAmount => snapshot.direct.fm_amount,
        ParamId::FmDirection => snapshot.direct.fm_direction,
        ParamId::PhaseModAmount => snapshot.direct.phase_mod_amount,
        ParamId::PhaseModDirection => snapshot.direct.phase_mod_direction,
        ParamId::RingModAmount => snapshot.direct.ring_mod_amount,
        ParamId::AmAmount => snapshot.direct.am_amount,
        ParamId::SyncDirection => snapshot.direct.sync_direction,
        ParamId::SyncSoftness => snapshot.direct.sync_softness,
        ParamId::CrossMixMode => snapshot.direct.cross_mix_mode,
        ParamId::CrossMixAmount => snapshot.direct.cross_mix_amount,
        ParamId::SubLevel => snapshot.direct.sub_level,
        ParamId::SubOctaveOffset => snapshot.direct.sub_octave_offset,
        ParamId::MixerPreFilterDrive => snapshot.direct.mixer_pre_filter_drive,
        ParamId::MixerBodyMix => snapshot.direct.mixer_body_mix,
        ParamId::FilterCutoffHz => snapshot.direct.cutoff_hz,
        ParamId::FilterResonance => snapshot.direct.resonance,
        ParamId::FilterDrive => snapshot.direct.filter_drive,
        ParamId::FilterKeytrack => snapshot.direct.filter_tracking,
        ParamId::AmpEnvAttackMs => snapshot.direct.amp_env.attack_ms,
        ParamId::AmpEnvDecayMs => snapshot.direct.amp_env.decay_ms,
        ParamId::AmpEnvSustain => snapshot.direct.amp_env.sustain,
        ParamId::AmpEnvReleaseMs => snapshot.direct.amp_env.release_ms,
        ParamId::FilterEnvAttackMs => snapshot.direct.filter_env.attack_ms,
        ParamId::FilterEnvDecayMs => snapshot.direct.filter_env.decay_ms,
        ParamId::FilterEnvSustain => snapshot.direct.filter_env.sustain,
        ParamId::FilterEnvReleaseMs => snapshot.direct.filter_env.release_ms,
        ParamId::FilterEnvDepth => snapshot.direct.filter_env_depth,
        ParamId::VoiceStereoWidth => snapshot.direct.stereo_width,
        ParamId::VoiceDetuneSpreadCents => snapshot.direct.detune_spread_cents,
        ParamId::VoiceVelocityToLevel => snapshot.direct.voice_velocity_to_level,
        ParamId::VoiceVelocityToFilter => snapshot.direct.voice_velocity_to_filter,
        ParamId::FinalStageBodyDrive => snapshot.direct.body_drive,
        ParamId::FinalStageAsymmetry => snapshot.direct.final_asymmetry,
        ParamId::FinalStageLowMidEmphasis => snapshot.direct.low_mid_emphasis,
        ParamId::FinalStageOutputTrimDb => snapshot.direct.output_trim_db,
        ParamId::ChorusEnabled => {
            if snapshot.direct.chorus_enabled {
                1.0
            } else {
                0.0
            }
        }
        ParamId::ChorusMix => snapshot.direct.chorus_mix,
        ParamId::ChorusDepth => snapshot.direct.chorus_depth,
        ParamId::ChorusRateHz => snapshot.direct.chorus_rate_hz,
        ParamId::ReverbEnabled => {
            if snapshot.direct.reverb_enabled {
                1.0
            } else {
                0.0
            }
        }
        ParamId::ReverbMix => snapshot.direct.reverb_mix,
        ParamId::ReverbSize => snapshot.direct.reverb_size,
        ParamId::ReverbDamping => snapshot.direct.reverb_damping,
        ParamId::PerformanceVelocityToLevel => snapshot.performance_response.velocity_to_level,
        ParamId::PerformanceVelocityToFilter => snapshot.performance_response.velocity_to_filter,
        ParamId::PerformanceAftertouchToGravitacija => {
            snapshot.performance_response.aftertouch_to_gravitacija
        }
        ParamId::PerformanceAftertouchToBaklja => {
            snapshot.performance_response.aftertouch_to_baklja
        }
        ParamId::PerformanceModWheelToBloom => snapshot.performance_response.mod_wheel_to_bloom,
        ParamId::PerformanceModWheelToSwarm => snapshot.performance_response.mod_wheel_to_swarm,
    };
    Some(value)
}

pub(crate) fn direct_param_display_value(snapshot: &EngineSnapshot, id: ParamId) -> Option<String> {
    let value = direct_param_raw_value(snapshot, id)?;
    Some(format_param_value(id, value))
}

pub(crate) fn format_param_value(id: ParamId, value: f32) -> String {
    match param_spec(id).unit {
        ParamUnit::Hertz => format!("{value:.1} Hz"),
        ParamUnit::Milliseconds => format!("{value:.1} ms"),
        ParamUnit::Decibels => format!("{value:.1} dB"),
        ParamUnit::Cents => format!("{value:.1} cents"),
        ParamUnit::Semitones => format!("{value:.1} st"),
        ParamUnit::Boolean => {
            if value >= 0.5 {
                "on".to_string()
            } else {
                "off".to_string()
            }
        }
        ParamUnit::Indexed => indexed_param_value_label(id, value),
        ParamUnit::Normalized => format!("{value:.2}"),
    }
}

fn indexed_param_value_label(id: ParamId, value: f32) -> String {
    let index = value.round() as i32;
    let label = match id {
        ParamId::Osc1PhaseMode | ParamId::Osc2PhaseMode => match index {
            1 => "fixed",
            2 => "free",
            _ => "reset",
        },
        ParamId::Osc2PitchMode => {
            if index >= 1 {
                "ratio"
            } else {
                "semitone"
            }
        }
        ParamId::NoiseColor => match index {
            1 => "pink",
            2 => "dark",
            3 => "bright",
            _ => "white",
        },
        ParamId::SpectralTable => match index {
            1 => "vocal",
            2 => "metal",
            3 => "hollow",
            4 => "formant",
            _ => "sine",
        },
        ParamId::FmDirection | ParamId::PhaseModDirection | ParamId::SyncDirection => {
            if index >= 1 { "2->1" } else { "1->2" }
        }
        ParamId::CrossMixMode => match index {
            1 => "multiply",
            2 => "fold",
            3 => "max",
            4 => "diff",
            _ => "sum",
        },
        _ => return format!("{index}"),
    };
    label.to_string()
}

pub(crate) fn event_matches_binding(event: &LastControlEvent, binding: &ControllerBinding) -> bool {
    event_is_recent(event)
        && matches!(
            event.kind,
            LastControlKind::ProfileCc(cc) | LastControlKind::LegacyCc(cc) if cc == binding.cc
        )
}

pub(crate) fn event_matches_legacy_cc(event: &LastControlEvent, cc: u8) -> bool {
    event_is_recent(event)
        && matches!(event.kind, LastControlKind::LegacyCc(event_cc) if event_cc == cc)
}

pub(crate) fn event_matches_program_change(event: &LastControlEvent, slot: u8) -> bool {
    event_is_recent(event)
        && matches!(event.kind, LastControlKind::ProgramChange(program) if program == slot)
}

pub(crate) fn event_is_recent(event: &LastControlEvent) -> bool {
    Instant::now() <= event.received_at + MIDI_ACTIVITY_FLASH
}

pub(crate) fn verdict_label(verdict: LastControlVerdict) -> &'static str {
    match verdict {
        LastControlVerdict::Accepted => "accepted",
        LastControlVerdict::Filtered => "filtered",
        LastControlVerdict::StartupSuppressed => "startup suppressed",
        LastControlVerdict::Reserved => "reserved",
        LastControlVerdict::ReleaseIgnored => "release ignored",
        LastControlVerdict::Ignored => "ignored",
    }
}

pub(crate) fn run_performance_window(session: RuntimeSession) -> Result<()> {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("EPM1 Performance Rig")
            .with_inner_size([1140.0, 720.0])
            .with_min_inner_size([960.0, 640.0]),
        ..Default::default()
    };

    eframe::run_native(
        "EPM1 Performance Rig",
        options,
        Box::new(move |cc| {
            apply_epm_gui_style(&cc.egui_ctx);
            Ok(Box::new(PerformanceApp::new(session)))
        }),
    )
    .map_err(|error| anyhow!("failed to launch performance window: {error}"))
}
