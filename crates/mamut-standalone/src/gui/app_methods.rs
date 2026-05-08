use super::*;

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
            engine_selected_section: ParamSection::Oscillator1,
            engine_scope_frames: Vec::with_capacity(ENGINE_SCOPE_BUFFER_FRAMES),
            engine_scope_drain: Vec::with_capacity(ENGINE_RENDER_BLOCK_FRAMES),
            engine_scope_frozen: false,
            engine_scope_gain: 1.0,
            engine_scope_window_frames: ENGINE_SCOPE_DEFAULT_WINDOW_FRAMES,
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

        self.drain_engine_scope();
    }

    pub(crate) fn drain_engine_scope(&mut self) {
        if self.selected_tab != PerformanceTab::Engine || self.engine_scope_frozen {
            return;
        }
        self.engine_scope_drain.clear();
        if self
            .session
            .drain_scope_frames(&mut self.engine_scope_drain)
            == 0
        {
            return;
        }
        self.engine_scope_frames
            .extend(self.engine_scope_drain.iter().copied());
        let excess = self
            .engine_scope_frames
            .len()
            .saturating_sub(ENGINE_SCOPE_BUFFER_FRAMES);
        if excess > 0 {
            self.engine_scope_frames.drain(0..excess);
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
}
