use super::*;

pub(crate) struct PerformanceApp {
    pub(crate) session: RuntimeSession,
    pub(crate) snapshot: Option<EngineSnapshot>,
    pub(crate) factory_patches: Vec<FactoryPatchEntry>,
    pub(crate) factory_patch_error: Option<String>,
    pub(crate) selected_tab: PerformanceTab,
    pub(crate) sound_lab_page: SoundLabPage,
    pub(crate) engine_selected_section: ParamSection,
    pub(crate) engine_scope_frames: Vec<StereoFrame>,
    pub(crate) engine_scope_drain: Vec<StereoFrame>,
    pub(crate) engine_scope_frozen: bool,
    pub(crate) engine_scope_gain: f32,
    pub(crate) engine_scope_window_frames: usize,
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
    Engine,
    Pc4,
    Debug,
}

impl PerformanceTab {
    pub(crate) const ALL: [Self; 5] = [
        Self::Live,
        Self::SoundLab,
        Self::Engine,
        Self::Pc4,
        Self::Debug,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Live => "Live",
            Self::SoundLab => "Sound Lab",
            Self::Engine => "Engine",
            Self::Pc4 => "PC4",
            Self::Debug => "Debug",
        }
    }

    pub(crate) fn uses_live_scope(self) -> bool {
        matches!(self, Self::Live | Self::SoundLab | Self::Engine)
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
    pub(crate) fn scope_enabled_for_selected_tab(&self) -> bool {
        self.selected_tab.uses_live_scope() && !self.engine_scope_frozen
    }
}

impl eframe::App for PerformanceApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.session
            .set_scope_enabled(self.scope_enabled_for_selected_tab());
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
                ui.add_space(10.0);
                if self.selected_tab.uses_live_scope() {
                    self.render_sticky_scope_panel(ui);
                    ui.add_space(12.0);
                } else {
                    ui.add_space(4.0);
                }
                match self.selected_tab {
                    PerformanceTab::Pc4 => self.render_pc4_tab(ui),
                    PerformanceTab::Live
                    | PerformanceTab::SoundLab
                    | PerformanceTab::Engine
                    | PerformanceTab::Debug => {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| match self.selected_tab {
                                PerformanceTab::Live => self.render_live_tab(ui, ctx),
                                PerformanceTab::SoundLab => self.render_sound_lab_tab(ui, ctx),
                                PerformanceTab::Engine => self.render_engine_tab(ui, ctx),
                                PerformanceTab::Debug => self.render_debug_tab(ui),
                                PerformanceTab::Pc4 => unreachable!(),
                            });
                    }
                }
            });

        self.session.set_sound_lab_midi_focus(
            (self.selected_tab == PerformanceTab::SoundLab).then_some(self.sound_lab_page),
        );
        self.session
            .set_scope_enabled(self.scope_enabled_for_selected_tab());
        ctx.request_repaint_after(PERFORMANCE_UI_REFRESH);
    }
}
