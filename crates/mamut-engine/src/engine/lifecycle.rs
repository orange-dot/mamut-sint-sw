use super::*;

impl Engine {
    pub fn new(config: EngineConfig, patch: PatchFileV1) -> Result<Self, PatchValidationError> {
        validate_patch_v1(&patch)?;
        let config = EngineConfig {
            sample_rate_hz: config.sample_rate_hz.max(8_000.0),
            max_block_frames: config.max_block_frames.max(1),
            voice_count: config.voice_count.max(1),
        };
        let live_macros = MacroState::from_defaults(&patch.macros);
        let last_frame = resolve_identity(&patch, &live_macros);
        let last_direct = resolve_direct_parameters(&patch, last_frame, ControlState::default());
        let render_smoothers = RenderSmoothers::new(last_direct);
        let mut engine = Self {
            config,
            patch,
            live_macros,
            control: ControlState::default(),
            voices: (0..config.voice_count)
                .map(|slot| VoiceState::idle(config.sample_rate_hz, slot))
                .collect(),
            age_counter: 0,
            last_frame,
            last_direct,
            render_smoothers,
            control_smoothing_samples: control_smoothing_samples(config.sample_rate_hz),
            last_block_frames: 0,
            last_events_processed: 0,
            last_peak_output: 0.0,
            last_output_safety: OutputSafetySnapshot::default(),
            final_body_left: 0.0,
            final_body_right: 0.0,
            chorus: SimpleChorus::new(config.sample_rate_hz),
            reverb: SimpleReverb::new(config.sample_rate_hz),
            master_dc_blocker: StereoDcBlocker::new(
                config.sample_rate_hz,
                MASTER_DC_BLOCKER_CUTOFF_HZ,
            ),
            patch_switch_mute_frames: 0,
            gfm_layer_mode: GfmLayerMode::Disabled,
            gfm_layer_auto_armed: false,
            gfm_layer_auto_disarm_pending: false,
            gfm_layer_mix: LinearSmoother::new(0.0),
            gfm_layer_pressure: LinearSmoother::new(0.0),
            gfm_layer_selection: GfmVoiceProgramSelection::default(),
            gfm_layer_voice: None,
            bcs_layer_mode: BcsLayerMode::Disabled,
            bcs_layer_mix: LinearSmoother::new(0.0),
            bcs_layer_note: None,
            bcs_layer_frequency_hz: None,
            bcs_layer_voice: None,
        };
        engine.refresh_resolved_state();
        Ok(engine)
    }

    pub fn load_patch(&mut self, patch: PatchFileV1) -> Result<(), PatchValidationError> {
        validate_patch_v1(&patch)?;
        self.patch = patch;
        self.live_macros = MacroState::from_defaults(&self.patch.macros);
        self.reset_runtime_state();
        self.patch_switch_mute_frames = patch_switch_mute_frames(self.config.sample_rate_hz);
        self.last_frame = resolve_identity(&self.patch, &self.live_macros);
        self.last_direct = resolve_direct_parameters(&self.patch, self.last_frame, self.control);
        self.render_smoothers = RenderSmoothers::new(self.last_direct);
        self.rebuild_gfm_layer();
        self.rebuild_bcs_layer();
        Ok(())
    }

    pub fn export_patch(&self) -> PatchFileV1 {
        self.patch.clone()
    }

    pub fn panic(&mut self) {
        self.live_macros = MacroState::from_defaults(&self.patch.macros);
        self.reset_runtime_state();
        self.patch_switch_mute_frames = 0;
        self.refresh_resolved_state();
        self.rebuild_gfm_layer();
        self.rebuild_bcs_layer();
    }

    pub fn reset_controllers(&mut self) {
        let sustain_was_down = self.control.sustain_down;
        self.control.pitch_bend_semitones = 0.0;
        self.control.mod_wheel = 0.0;
        self.control.aftertouch = 0.0;
        self.control.gfm_layer_pressure = 0.0;
        self.control.gfm_layer_amount = 0.0;
        self.control.bcs_layer_enabled = false;
        self.control.bcs_layer_amount = 0.0;
        self.control.sustain_down = false;
        self.live_macros = MacroState::from_defaults(&self.patch.macros);
        self.force_disable_auto_armed_gfm_layer();
        self.reset_gfm_layer_smoothing();
        self.reset_bcs_layer_smoothing();

        if sustain_was_down {
            for voice in &mut self.voices {
                if voice.phase == VoicePhase::SustainedReleased {
                    voice.start_release();
                }
            }
        }

        self.refresh_resolved_state();
    }

    pub fn set_gfm_layer_mode(&mut self, mode: GfmLayerMode) -> GfmVoiceProgramSelection {
        self.gfm_layer_mode = mode;
        self.gfm_layer_auto_armed = false;
        self.gfm_layer_auto_disarm_pending = false;
        if matches!(mode, GfmLayerMode::Enabled { .. }) {
            self.control.gfm_layer_pressure = 0.0;
            self.control.gfm_layer_amount = 0.0;
        }
        self.reset_gfm_layer_smoothing();
        self.rebuild_gfm_layer();
        self.gfm_layer_selection
    }

    pub fn set_bcs_layer_mode(&mut self, mode: BcsLayerMode) -> BcsLayerSnapshot {
        self.bcs_layer_mode = mode;
        if matches!(mode, BcsLayerMode::Disabled) {
            self.reset_bcs_layer_smoothing();
        }
        self.rebuild_bcs_layer();
        self.update_bcs_layer_smoothing_targets();
        self.bcs_layer_snapshot()
    }

    pub(super) fn auto_enable_gfm_layer_for_momentary_control(&mut self) {
        if matches!(self.gfm_layer_mode, GfmLayerMode::Disabled) {
            self.gfm_layer_mode = GfmLayerMode::Enabled {
                seed: DEFAULT_GFM_LAYER_SEED,
            };
            self.gfm_layer_auto_armed = true;
            self.gfm_layer_auto_disarm_pending = false;
            self.rebuild_gfm_layer();
        }
    }

    pub(super) fn maybe_auto_enable_gfm_layer_for_momentary_control(&mut self) {
        if self.control.gfm_layer_amount > f32::EPSILON
            && self.control.gfm_layer_pressure > f32::EPSILON
        {
            self.auto_enable_gfm_layer_for_momentary_control();
            self.gfm_layer_auto_disarm_pending = false;
        }
    }

    pub(super) fn begin_auto_disarm_gfm_layer_if_momentary_control_released(&mut self) {
        if self.gfm_layer_auto_armed {
            self.gfm_layer_auto_disarm_pending = true;
        }
    }

    pub(super) fn finish_pending_auto_disarm_if_silent(&mut self) {
        if self.gfm_layer_auto_disarm_pending && self.gfm_layer_mix.current() <= f32::EPSILON {
            self.force_disable_auto_armed_gfm_layer();
        }
    }

    pub(super) fn force_disable_auto_armed_gfm_layer(&mut self) {
        if self.gfm_layer_auto_armed || self.gfm_layer_auto_disarm_pending {
            self.gfm_layer_mode = GfmLayerMode::Disabled;
            self.gfm_layer_auto_armed = false;
            self.gfm_layer_auto_disarm_pending = false;
            self.rebuild_gfm_layer();
        }
    }

    pub(super) fn reset_gfm_layer_smoothing(&mut self) {
        self.gfm_layer_mix = LinearSmoother::new(0.0);
        self.gfm_layer_pressure = LinearSmoother::new(0.0);
    }

    pub(super) fn update_gfm_layer_smoothing_targets(&mut self) {
        let mix_target = gfm_momentary_layer_amount(
            self.control.gfm_layer_amount,
            self.control.gfm_layer_pressure,
        );
        let pressure_target = if mix_target <= f32::EPSILON {
            0.0
        } else {
            self.control.gfm_layer_pressure
        };
        let mix_ms = if mix_target > self.gfm_layer_mix.current() {
            GFM_LAYER_MIX_ATTACK_MS
        } else {
            GFM_LAYER_MIX_RELEASE_MS
        };
        let pressure_ms = if pressure_target > self.gfm_layer_pressure.current() {
            GFM_LAYER_PRESSURE_ATTACK_MS
        } else {
            GFM_LAYER_PRESSURE_RELEASE_MS
        };
        self.gfm_layer_mix.set_target(
            mix_target,
            smoothing_sample_count(self.config.sample_rate_hz, mix_ms),
        );
        self.gfm_layer_pressure.set_target(
            pressure_target,
            smoothing_sample_count(self.config.sample_rate_hz, pressure_ms),
        );
    }

    pub(super) fn reset_bcs_layer_smoothing(&mut self) {
        self.bcs_layer_mix = LinearSmoother::new(0.0);
    }

    pub(super) fn update_bcs_layer_smoothing_targets(&mut self) {
        let amount = normalize_bcs_layer_control(self.control.bcs_layer_amount);
        let target = if matches!(self.bcs_layer_mode, BcsLayerMode::Enabled { .. })
            && self.control.bcs_layer_enabled
            && self.bcs_layer_note.is_some()
        {
            amount
        } else {
            0.0
        };
        let mix_ms = if target > self.bcs_layer_mix.current() {
            BCS_LAYER_MIX_ATTACK_MS
        } else {
            BCS_LAYER_MIX_RELEASE_MS
        };
        self.bcs_layer_mix.set_target(
            target,
            smoothing_sample_count(self.config.sample_rate_hz, mix_ms),
        );
    }

    pub const fn gfm_layer_mode(&self) -> GfmLayerMode {
        self.gfm_layer_mode
    }

    pub const fn bcs_layer_mode(&self) -> BcsLayerMode {
        self.bcs_layer_mode
    }
}
