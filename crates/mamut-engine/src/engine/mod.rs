use super::*;

#[derive(Debug, Clone)]
pub struct Engine {
    config: EngineConfig,
    pub(crate) patch: PatchFileV1,
    live_macros: MacroState,
    control: ControlState,
    voices: Vec<VoiceState>,
    age_counter: u64,
    last_frame: ResolvedIdentityFrame,
    last_direct: DirectParameters,
    render_smoothers: RenderSmoothers,
    control_smoothing_samples: usize,
    last_block_frames: usize,
    last_events_processed: usize,
    last_peak_output: f32,
    last_output_safety: OutputSafetySnapshot,
    final_body_left: f32,
    final_body_right: f32,
    chorus: SimpleChorus,
    reverb: SimpleReverb,
    master_dc_blocker: StereoDcBlocker,
    patch_switch_mute_frames: usize,
    gfm_layer_mode: GfmLayerMode,
    gfm_layer_auto_armed: bool,
    gfm_layer_auto_disarm_pending: bool,
    gfm_layer_mix: LinearSmoother,
    gfm_layer_pressure: LinearSmoother,
    gfm_layer_selection: GfmVoiceProgramSelection,
    gfm_layer_voice: Option<GfmFieldVoice>,
    bcs_layer_mode: BcsLayerMode,
    bcs_layer_mix: LinearSmoother,
    bcs_layer_note: Option<u8>,
    bcs_layer_frequency_hz: Option<f32>,
    bcs_layer_voice: Option<BcsVoice>,
}

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

    fn auto_enable_gfm_layer_for_momentary_control(&mut self) {
        if matches!(self.gfm_layer_mode, GfmLayerMode::Disabled) {
            self.gfm_layer_mode = GfmLayerMode::Enabled {
                seed: DEFAULT_GFM_LAYER_SEED,
            };
            self.gfm_layer_auto_armed = true;
            self.gfm_layer_auto_disarm_pending = false;
            self.rebuild_gfm_layer();
        }
    }

    fn maybe_auto_enable_gfm_layer_for_momentary_control(&mut self) {
        if self.control.gfm_layer_amount > f32::EPSILON
            && self.control.gfm_layer_pressure > f32::EPSILON
        {
            self.auto_enable_gfm_layer_for_momentary_control();
            self.gfm_layer_auto_disarm_pending = false;
        }
    }

    fn begin_auto_disarm_gfm_layer_if_momentary_control_released(&mut self) {
        if self.gfm_layer_auto_armed {
            self.gfm_layer_auto_disarm_pending = true;
        }
    }

    fn finish_pending_auto_disarm_if_silent(&mut self) {
        if self.gfm_layer_auto_disarm_pending && self.gfm_layer_mix.current() <= f32::EPSILON {
            self.force_disable_auto_armed_gfm_layer();
        }
    }

    fn force_disable_auto_armed_gfm_layer(&mut self) {
        if self.gfm_layer_auto_armed || self.gfm_layer_auto_disarm_pending {
            self.gfm_layer_mode = GfmLayerMode::Disabled;
            self.gfm_layer_auto_armed = false;
            self.gfm_layer_auto_disarm_pending = false;
            self.rebuild_gfm_layer();
        }
    }

    fn reset_gfm_layer_smoothing(&mut self) {
        self.gfm_layer_mix = LinearSmoother::new(0.0);
        self.gfm_layer_pressure = LinearSmoother::new(0.0);
    }

    fn update_gfm_layer_smoothing_targets(&mut self) {
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

    fn reset_bcs_layer_smoothing(&mut self) {
        self.bcs_layer_mix = LinearSmoother::new(0.0);
    }

    fn update_bcs_layer_smoothing_targets(&mut self) {
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

    pub fn gfm_layer_diagnostics(&self) -> Option<GfmDiagnostics> {
        self.gfm_layer_voice
            .as_ref()
            .map(GfmFieldVoice::diagnostics)
    }

    pub fn process_block(&mut self, block: ProcessBlock<'_>) {
        let note_events = block.note_events;
        let controller_events = block.controller_events;
        debug_assert!(is_sorted_by_frame(note_events));
        debug_assert!(is_sorted_by_frame(controller_events));

        let requested_frames = block.frame_count.min(self.config.max_block_frames);
        let mut output = block.output;
        let rendered_frames = output
            .as_ref()
            .map(|buffer| requested_frames.min(buffer.frames()))
            .unwrap_or(0);
        if let Some(buffer) = output.as_mut() {
            buffer.clear();
        }

        if let Some(macro_state) = block.macro_state {
            self.live_macros = macro_state.clamped();
        }
        self.refresh_resolved_state();

        let mut note_index = 0;
        let mut controller_index = 0;
        let mut events_processed = 0;
        let mut peak_output: f32 = 0.0;
        let mut output_safety = OutputSafetySnapshot::default();

        for frame in 0..requested_frames {
            while controller_index < controller_events.len()
                && controller_events[controller_index].frame_offset <= frame
            {
                self.handle_controller_event(controller_events[controller_index].event);
                controller_index += 1;
                events_processed += 1;
                self.refresh_resolved_state();
            }

            while note_index < note_events.len() && note_events[note_index].frame_offset <= frame {
                self.handle_note_event(note_events[note_index].event);
                note_index += 1;
                events_processed += 1;
            }

            let (left, right) = self.render_frame();
            let (left, right) = self.master_dc_blocker.process(left, right);
            output_safety.pre_safety_peak = output_safety
                .pre_safety_peak
                .max(left.abs().max(right.abs()));

            let limited_left = master_safety_limit(left);
            let limited_right = master_safety_limit(right);
            output_safety.observe_channel(left, limited_left);
            output_safety.observe_channel(right, limited_right);

            let left = sanitize_sample(limited_left);
            let right = sanitize_sample(limited_right);
            peak_output = peak_output.max(left.abs().max(right.abs()));
            output_safety.post_safety_peak = output_safety
                .post_safety_peak
                .max(left.abs().max(right.abs()));

            if frame < rendered_frames {
                let Some(buffer) = output.as_mut() else {
                    continue;
                };
                buffer.left[frame] = left;
                buffer.right[frame] = right;
            }
        }

        self.last_block_frames = requested_frames;
        self.last_events_processed = events_processed;
        self.last_peak_output = peak_output;
        self.last_output_safety = output_safety;
    }

    pub fn snapshot(&self) -> EngineSnapshot {
        let effective_macros = self.effective_macro_state();
        let voices: Vec<VoiceSnapshot> = self
            .voices
            .iter()
            .enumerate()
            .map(|(slot, voice)| VoiceSnapshot {
                slot,
                note: voice.note,
                velocity: voice.velocity,
                phase: voice.phase,
                age: voice.age,
            })
            .collect();

        let held_notes = voices
            .iter()
            .filter_map(|voice| match voice.phase {
                VoicePhase::Held | VoicePhase::Released | VoicePhase::SustainedReleased => {
                    voice.note
                }
                VoicePhase::Idle => None,
            })
            .collect();

        EngineSnapshot {
            patch_name: self.patch.meta.patch_name.clone(),
            patch_description: self.patch.meta.description.clone(),
            patch_tags: self.patch.meta.tags.clone().unwrap_or_default(),
            patch_favorite: self
                .patch
                .ui
                .as_ref()
                .and_then(|ui| ui.favorite)
                .unwrap_or(false),
            sample_rate_hz: self.config.sample_rate_hz,
            last_block_frames: self.last_block_frames,
            events_processed: self.last_events_processed,
            active_voice_count: self
                .voices
                .iter()
                .filter(|voice| voice.phase != VoicePhase::Idle)
                .count(),
            sustain_down: self.control.sustain_down,
            live_macros: self.live_macros,
            effective_macros,
            identity: self.last_frame.identity,
            derived: self.last_frame.derived,
            direct: self.last_direct,
            performance_response: PerformanceResponseSnapshot {
                velocity_to_level: self.patch.performance_response.velocity_to_level,
                velocity_to_filter: self.patch.performance_response.velocity_to_filter,
                aftertouch_to_gravitacija: self
                    .patch
                    .performance_response
                    .aftertouch_to_gravitacija,
                aftertouch_to_baklja: self.patch.performance_response.aftertouch_to_baklja,
                mod_wheel_to_bloom: self.patch.performance_response.mod_wheel_to_bloom,
                mod_wheel_to_swarm: self.patch.performance_response.mod_wheel_to_swarm,
                bend_range_semitones: self.patch.performance_response.bend_range_semitones,
            },
            gfm_layer: self.gfm_layer_snapshot(),
            bcs_layer: self.bcs_layer_snapshot(),
            voices,
            held_notes,
            output_safety: self.last_output_safety,
            peak_output: self.last_peak_output,
            clip_detected: self.last_peak_output >= 0.98,
        }
    }

    fn reset_runtime_state(&mut self) {
        self.control = ControlState::default();
        self.age_counter = 0;
        self.last_block_frames = 0;
        self.last_events_processed = 0;
        self.last_peak_output = 0.0;
        self.last_output_safety = OutputSafetySnapshot::default();
        self.final_body_left = 0.0;
        self.final_body_right = 0.0;
        self.chorus = SimpleChorus::new(self.config.sample_rate_hz);
        self.reverb = SimpleReverb::new(self.config.sample_rate_hz);
        self.master_dc_blocker.reset();
        self.voices = (0..self.config.voice_count)
            .map(|slot| VoiceState::idle(self.config.sample_rate_hz, slot))
            .collect();
        if self.gfm_layer_auto_armed || self.gfm_layer_auto_disarm_pending {
            self.gfm_layer_mode = GfmLayerMode::Disabled;
            self.gfm_layer_auto_armed = false;
            self.gfm_layer_auto_disarm_pending = false;
        }
        self.reset_gfm_layer_smoothing();
        self.gfm_layer_selection = GfmVoiceProgramSelection::default();
        self.gfm_layer_voice = None;
        self.reset_bcs_layer_smoothing();
        self.bcs_layer_note = None;
        self.bcs_layer_frequency_hz = None;
        self.bcs_layer_voice = None;
    }

    fn refresh_resolved_state(&mut self) {
        let effective_macros = self.effective_macro_state();
        self.last_frame = resolve_identity(&self.patch, &effective_macros);
        self.last_direct = resolve_direct_parameters(&self.patch, self.last_frame, self.control);
        self.render_smoothers
            .set_targets(self.last_direct, self.control_smoothing_samples);

        for voice in &mut self.voices {
            voice.amp_env.set_timing(self.last_direct.amp_env);
            voice.filter_env.set_timing(self.last_direct.filter_env);
        }
    }

    fn rebuild_gfm_layer(&mut self) {
        match self.gfm_layer_mode {
            GfmLayerMode::Disabled => {
                self.gfm_layer_selection = GfmVoiceProgramSelection::default();
                self.gfm_layer_voice = None;
            }
            GfmLayerMode::Enabled { seed } => {
                let config = GfmPatchVoiceConfig::new(
                    seed,
                    engine_gfm_sample_rate_hz(self.config.sample_rate_hz),
                );
                let decision = GfmFieldVoice::from_patch(&self.patch, config);
                self.gfm_layer_selection = decision.selection();
                self.gfm_layer_voice = decision.into_voice();
            }
        }
    }

    fn rebuild_bcs_layer(&mut self) {
        match self.bcs_layer_mode {
            BcsLayerMode::Disabled => {
                self.bcs_layer_voice = None;
            }
            BcsLayerMode::Enabled { scenario } => {
                let mut params = BcsParams::for_scenario(scenario);
                params.sample_rate_hz = self.config.sample_rate_hz;
                if let Some(frequency_hz) = self.bcs_layer_frequency_hz {
                    params.base_frequency_hz = frequency_hz;
                }
                self.bcs_layer_voice = Some(BcsVoice::new(params));
            }
        }
    }

    fn refresh_bcs_layer_pitch(&mut self) {
        let desired = self.current_bcs_layer_pitch();
        match desired {
            Some((note, frequency_hz)) => {
                let changed = self.bcs_layer_note != Some(note)
                    || self
                        .bcs_layer_frequency_hz
                        .map_or(true, |current| (current - frequency_hz).abs() > 0.001);
                self.bcs_layer_note = Some(note);
                self.bcs_layer_frequency_hz = Some(frequency_hz);
                if changed {
                    if let Some(voice) = self.bcs_layer_voice.as_mut() {
                        voice.set_base_frequency_hz(frequency_hz);
                    } else {
                        self.rebuild_bcs_layer();
                    }
                }
            }
            None => {
                self.bcs_layer_note = None;
                self.bcs_layer_frequency_hz = None;
            }
        }
        self.update_bcs_layer_smoothing_targets();
    }

    fn current_bcs_layer_pitch(&self) -> Option<(u8, f32)> {
        self.voices
            .iter()
            .filter_map(|voice| match voice.phase {
                VoicePhase::Held => voice.note,
                VoicePhase::Idle | VoicePhase::Released | VoicePhase::SustainedReleased => None,
            })
            .min()
            .map(|note| {
                (
                    note,
                    midi_note_hz(note as f32 + self.control.pitch_bend_semitones),
                )
            })
    }

    fn gfm_layer_snapshot(&self) -> GfmLayerSnapshot {
        GfmLayerSnapshot {
            mode: self.gfm_layer_mode,
            selection: self.gfm_layer_selection,
            active_program_id: self.gfm_layer_voice.as_ref().map(GfmFieldVoice::program_id),
            diagnostics: self.gfm_layer_diagnostics(),
            amount: self.control.gfm_layer_amount,
            pressure: self.control.gfm_layer_pressure,
            effective_amount: self.gfm_layer_mix.current(),
        }
    }

    fn bcs_layer_snapshot(&self) -> BcsLayerSnapshot {
        let mode_enabled = matches!(self.bcs_layer_mode, BcsLayerMode::Enabled { .. });
        let active_scenario = match (self.bcs_layer_mode, self.bcs_layer_voice.as_ref()) {
            (BcsLayerMode::Enabled { scenario }, Some(_)) => Some(scenario),
            _ => None,
        };
        let sample_rate_hz = self.bcs_layer_voice.as_ref().map(|voice| {
            voice
                .params()
                .sample_rate_hz
                .round()
                .clamp(1.0, u32::MAX as f32) as u32
        });

        let amount = self.control.bcs_layer_amount;
        let effective_amount = self.bcs_layer_mix.current();

        BcsLayerSnapshot {
            mode: self.bcs_layer_mode,
            active_scenario,
            sample_rate_hz,
            enabled: self.control.bcs_layer_enabled,
            amount,
            effective_amount,
            gain: amount * BCS_ENGINE_LAYER_GAIN,
            effective_gain: effective_amount * BCS_ENGINE_LAYER_GAIN,
            pitch_note: if mode_enabled {
                self.bcs_layer_note
            } else {
                None
            },
            pitch_frequency_hz: if mode_enabled {
                self.bcs_layer_frequency_hz
            } else {
                None
            },
            max_state_abs: self
                .bcs_layer_voice
                .as_ref()
                .map(BcsVoice::max_state_abs)
                .unwrap_or(0.0),
            unsafe_events: self
                .bcs_layer_voice
                .as_ref()
                .map(BcsVoice::unsafe_events)
                .unwrap_or(0),
            unsafe_state: self
                .bcs_layer_voice
                .as_ref()
                .map(BcsVoice::unsafe_state)
                .unwrap_or(false),
        }
    }

    fn handle_note_event(&mut self, event: NoteEvent) {
        match event {
            NoteEvent::NoteOn { note, velocity } if velocity > 0.0 => self.note_on(note, velocity),
            NoteEvent::NoteOn { note, .. } | NoteEvent::NoteOff { note } => self.note_off(note),
        }
    }

    fn handle_controller_event(&mut self, event: ControllerEvent) {
        match event {
            ControllerEvent::PitchBend { semitones } => {
                self.control.pitch_bend_semitones = semitones;
                self.refresh_bcs_layer_pitch();
            }
            ControllerEvent::ModWheel { amount } => {
                self.control.mod_wheel = amount.clamp(0.0, 1.0);
            }
            ControllerEvent::Sustain { down } => {
                self.control.sustain_down = down;
                if !down {
                    for voice in &mut self.voices {
                        if voice.phase == VoicePhase::SustainedReleased {
                            voice.start_release();
                        }
                    }
                }
                self.refresh_bcs_layer_pitch();
            }
            ControllerEvent::ChannelAftertouch { pressure } => {
                let pressure = pressure.clamp(0.0, 1.0);
                let gfm_amount = normalize_gfm_layer_control(pressure);
                self.control.aftertouch = pressure;
                self.control.gfm_layer_amount = gfm_amount;
                if gfm_amount > f32::EPSILON {
                    self.maybe_auto_enable_gfm_layer_for_momentary_control();
                } else {
                    self.begin_auto_disarm_gfm_layer_if_momentary_control_released();
                }
                self.update_gfm_layer_smoothing_targets();
            }
            ControllerEvent::GfmLayerAmount { amount } => {
                let pressure = normalize_gfm_layer_control(amount);
                self.control.gfm_layer_pressure = pressure;
                if pressure > f32::EPSILON {
                    self.maybe_auto_enable_gfm_layer_for_momentary_control();
                } else {
                    self.begin_auto_disarm_gfm_layer_if_momentary_control_released();
                }
                self.update_gfm_layer_smoothing_targets();
            }
            ControllerEvent::BcsLayerEnabled { enabled } => {
                self.control.bcs_layer_enabled = enabled;
                self.refresh_bcs_layer_pitch();
            }
            ControllerEvent::BcsLayerAmount { amount } => {
                self.control.bcs_layer_amount = normalize_bcs_layer_control(amount);
                self.refresh_bcs_layer_pitch();
            }
            ControllerEvent::Macro { id, value } => {
                self.live_macros.set(id, value.clamp(0.0, 1.0));
            }
            ControllerEvent::DirectParam { id, value } => self.apply_direct_param(id, value),
        }
    }

    fn note_on(&mut self, note: u8, velocity: f32) {
        let voice_index = self.allocate_voice_index();
        self.age_counter += 1;
        self.voices[voice_index].trigger(
            note,
            velocity,
            self.age_counter,
            self.last_direct.amp_env,
            self.last_direct.filter_env,
            voice_index,
            self.last_direct.osc1_phase_mode,
            self.last_direct.osc1_start_phase,
            self.last_direct.osc2_phase_mode,
            self.last_direct.osc2_start_phase,
            self.last_direct.additive_random_detune_cents,
        );
        self.refresh_bcs_layer_pitch();
    }

    fn note_off(&mut self, note: u8) {
        if let Some(index) = self
            .voices
            .iter()
            .enumerate()
            .filter(|(_, voice)| voice.note == Some(note) && voice.phase == VoicePhase::Held)
            .min_by_key(|(_, voice)| voice.age)
            .map(|(index, _)| index)
        {
            let voice = &mut self.voices[index];
            if self.control.sustain_down {
                voice.phase = VoicePhase::SustainedReleased;
            } else {
                voice.start_release();
            }
        }
        if !self
            .voices
            .iter()
            .any(|voice| voice.phase == VoicePhase::Held)
        {
            self.control.gfm_layer_amount = 0.0;
            self.begin_auto_disarm_gfm_layer_if_momentary_control_released();
            self.update_gfm_layer_smoothing_targets();
        }
        self.refresh_bcs_layer_pitch();
    }

    fn allocate_voice_index(&self) -> usize {
        if let Some(index) = self
            .voices
            .iter()
            .position(|voice| voice.phase == VoicePhase::Idle)
        {
            return index;
        }

        if let Some((index, _)) = self
            .voices
            .iter()
            .enumerate()
            .filter(|(_, voice)| voice.phase == VoicePhase::Released)
            .min_by(|left, right| compare_voice_reuse(left.1, right.1))
        {
            return index;
        }

        if let Some((index, _)) = self
            .voices
            .iter()
            .enumerate()
            .filter(|(_, voice)| voice.phase == VoicePhase::SustainedReleased)
            .min_by(|left, right| compare_voice_reuse(left.1, right.1))
        {
            return index;
        }

        self.voices
            .iter()
            .enumerate()
            .filter(|(_, voice)| voice.phase == VoicePhase::Held)
            .min_by_key(|(_, voice)| voice.age)
            .map(|(index, _)| index)
            .unwrap_or(0)
    }

    fn effective_macro_state(&self) -> MacroState {
        let performance = self.patch.performance_response;
        let mut macros = self.live_macros;
        let aftertouch = expressive_amount(self.control.aftertouch, 0.82);
        let mod_wheel = expressive_amount(self.control.mod_wheel, 0.92);
        macros.gravitacija += aftertouch * performance.aftertouch_to_gravitacija;
        macros.ruin += aftertouch.powf(1.08) * performance.aftertouch_to_baklja;
        macros.bloom += mod_wheel * performance.mod_wheel_to_bloom;
        macros.swarm += mod_wheel.powf(0.88) * performance.mod_wheel_to_swarm;
        macros.clamped()
    }

    pub(crate) fn render_frame(&mut self) -> (f32, f32) {
        if self.patch_switch_mute_frames > 0 {
            self.patch_switch_mute_frames -= 1;
            return (0.0, 0.0);
        }

        let direct = self.render_smoothers.next_direct(self.last_direct);
        let derived = self.last_frame.derived;
        let identity = self.last_frame.identity;
        let engine_patch = &self.patch.engine;
        let sample_rate_hz = self.config.sample_rate_hz;
        let pitch_bend_semitones = self.control.pitch_bend_semitones;
        let note_velocity_to_level = engine_patch
            .voice
            .velocity_to_level
            .unwrap_or(self.patch.performance_response.velocity_to_level)
            .clamp(0.0, 1.0);
        let note_velocity_to_filter = engine_patch
            .voice
            .velocity_to_filter
            .unwrap_or(self.patch.performance_response.velocity_to_filter)
            .clamp(0.0, 1.0);
        let voice_count = self.voices.len().max(1);
        let osc1_fine = engine_patch.osc1.fine_tune_cents.unwrap_or(0.0) / 100.0;
        let osc2_fine = engine_patch.osc2.fine_tune_cents / 100.0;
        let sub_octave = engine_patch.sub.octave_offset as f32 * 12.0;
        let mixer_body_gain = 0.4 + engine_patch.mixer.body_mix * 0.6;
        let pre_filter_gain = 1.0 + engine_patch.mixer.pre_filter_drive * 2.0;
        let voice_level_gain = 0.40 + direct.voice_level * 0.60;
        let stereo_width = direct.stereo_width;
        let strain_drive = 1.0 + derived.strain * 0.22;
        let strain_bias = identity.baklja_edge * 0.18;

        let mut left = 0.0;
        let mut right = 0.0;

        for (slot, voice) in self.voices.iter_mut().enumerate() {
            if voice.phase == VoicePhase::Idle {
                continue;
            }

            let Some(note) = voice.note else {
                continue;
            };

            let spread_position = if voice_count == 1 {
                0.0
            } else {
                (slot as f32 / (voice_count - 1) as f32) * 2.0 - 1.0
            };
            let spread_detune_semitones =
                spread_position * direct.detune_spread_cents * 0.5 / 100.0;
            let pwm_lfo = (voice.pwm_lfo.phase() * std::f32::consts::TAU).sin();
            voice
                .pwm_lfo
                .advance(direct.source_pwm_rate_hz, sample_rate_hz);
            let drift_lfo = (voice.drift_lfo.phase() * std::f32::consts::TAU).sin();
            voice.drift_lfo.advance(0.083, sample_rate_hz);
            let jitter = voice.jitter.next_bipolar();
            let pitch_instability =
                drift_lfo * direct.analog_drift * 0.018 + jitter * direct.micro_jitter * 0.006;
            let pulse_bias = identity.baklja_edge * 0.18 - identity.horizont_air * 0.05;
            let osc1_pulse_width =
                (direct.osc1_pulse_width + pulse_bias + pwm_lfo * direct.osc1_pwm_depth * 0.40)
                    .clamp(0.05, 0.95);
            let osc2_pulse_width =
                (direct.osc2_pulse_width + pulse_bias + pwm_lfo * direct.osc2_pwm_depth * 0.40)
                    .clamp(0.05, 0.95);
            let raw_noise = voice.noise.next_bipolar();
            let colored_noise =
                color_noise_sample(raw_noise, direct.noise_color, &mut voice.noise_color_state);
            let osc2_preview = oscillator_preview_sample(
                &voice.osc2,
                osc2_pulse_width,
                direct.osc2_saw_bend,
                direct.osc2_triangle_fold,
                direct.osc2_pulse_edge,
            );

            let osc1_note = note as f32
                + pitch_bend_semitones
                + osc1_fine
                + spread_detune_semitones
                + pitch_instability;
            let mut osc1_freq = midi_note_hz(osc1_note);
            if is_osc2_to_osc1(direct.fm_direction) && direct.fm_amount > 0.0 {
                osc1_freq *= (1.0 + osc2_preview * direct.fm_amount * 0.25).clamp(0.25, 4.0);
            }
            let osc1_phase_mod =
                if is_osc2_to_osc1(direct.phase_mod_direction) && direct.phase_mod_amount > 0.0 {
                    osc2_preview * direct.phase_mod_amount * 0.080
                } else {
                    0.0
                };
            let osc1_mix = mixed_wave(
                &voice.osc1,
                direct.osc1_wave_mix,
                osc1_pulse_width,
                colored_noise,
                osc1_phase_mod,
                direct.osc1_saw_bend,
                direct.osc1_triangle_fold,
                direct.osc1_pulse_edge,
            );
            let osc1_wrapped = voice.osc1.advance(osc1_freq, sample_rate_hz);

            let effective_sync_amount =
                direct.sync_amount * (1.0 - direct.sync_softness * 0.75).clamp(0.0, 1.0);
            if osc1_wrapped
                && effective_sync_amount > 0.0
                && !is_osc2_to_osc1(direct.sync_direction)
            {
                voice.osc2.hard_sync(effective_sync_amount);
            }

            let osc2_freq_base = if direct.osc2_pitch_mode.round() as i32 == 1 {
                midi_note_hz(note as f32 + pitch_bend_semitones + spread_detune_semitones)
                    * direct.osc2_ratio
                    * 2.0_f32.powf(osc2_fine / 12.0)
            } else {
                let osc2_note = note as f32
                    + direct.osc2_interval_semitones
                    + osc2_fine
                    + spread_detune_semitones
                    + pitch_instability;
                midi_note_hz(osc2_note)
            };
            let crossmod = (1.0 + osc1_mix * direct.crossmod_amount * 0.25).clamp(0.25, 4.0);
            let fm = if !is_osc2_to_osc1(direct.fm_direction) && direct.fm_amount > 0.0 {
                (1.0 + osc1_mix * direct.fm_amount * 0.25).clamp(0.25, 4.0)
            } else {
                1.0
            };
            let osc2_freq = (osc2_freq_base * crossmod * fm).clamp(4.0, sample_rate_hz * 0.45);
            let osc2_phase_mod =
                if !is_osc2_to_osc1(direct.phase_mod_direction) && direct.phase_mod_amount > 0.0 {
                    osc1_mix * direct.phase_mod_amount * 0.080
                } else {
                    0.0
                };
            let osc2_mix = mixed_wave_osc2(
                &voice.osc2,
                direct.osc2_wave_mix,
                osc2_pulse_width,
                osc2_phase_mod,
                direct.osc2_saw_bend,
                direct.osc2_triangle_fold,
                direct.osc2_pulse_edge,
            ) * direct.osc2_level;
            let osc2_wrapped = voice.osc2.advance(osc2_freq, sample_rate_hz);
            if osc2_wrapped && effective_sync_amount > 0.0 && is_osc2_to_osc1(direct.sync_direction)
            {
                voice.osc1.hard_sync(effective_sync_amount);
            }

            let spectral_note =
                note as f32 + pitch_bend_semitones + spread_detune_semitones + pitch_instability;
            let spectral_freq = (midi_note_hz(spectral_note)
                * direct.spectral_ratio
                * 2.0_f32.powf(direct.spectral_fine_tune_cents / 1200.0))
            .clamp(4.0, sample_rate_hz * 0.45);
            let spectral_mix = spectral_wavetable_sample(
                &voice.spectral,
                direct.spectral_table,
                direct.spectral_position,
                direct.spectral_morph,
            ) * direct.spectral_level;
            voice.spectral.advance(spectral_freq, sample_rate_hz);

            let additive_mix = if direct.additive_level > f32::EPSILON {
                let partial_count = additive_partial_count(direct.additive_partial_count);
                let base_freq = midi_note_hz(spectral_note).clamp(4.0, sample_rate_hz * 0.45);
                let mut sample_sum = 0.0_f32;
                let mut weight_sum = 0.0_f32;
                for index in 0..partial_count {
                    let weight = additive_weight(
                        index,
                        partial_count,
                        direct.additive_odd_even_balance,
                        direct.additive_spectral_tilt,
                    );
                    sample_sum += sine_phase_sample(&voice.additive_partials[index]) * weight;
                    weight_sum += weight;

                    let ratio = additive_ratio(
                        index,
                        direct.additive_harmonic_spread,
                        direct.additive_inharmonicity,
                    );
                    let detune_scale = 2.0_f32.powf(voice.additive_detune_cents[index] / 1200.0);
                    let partial_freq =
                        (base_freq * ratio * detune_scale).clamp(4.0, sample_rate_hz * 0.45);
                    voice.additive_partials[index].advance(partial_freq, sample_rate_hz);
                }
                (sample_sum / weight_sum.max(0.001)) * direct.additive_level
            } else {
                0.0
            };

            let sub_freq = midi_note_hz(note as f32 + pitch_bend_semitones + sub_octave);
            let sub_mix = voice.sub.square_sample() * direct.sub_level;
            voice.sub.advance(sub_freq, sample_rate_hz);

            let body_mix = sub_mix * mixer_body_gain * (0.62 + derived.mass * 0.46);
            let am_gain = 1.0 - direct.am_amount * 0.50
                + ((osc2_mix * 0.5 + 0.5).clamp(0.0, 1.0) * direct.am_amount);
            let osc1_source = osc1_mix * am_gain;
            let summed_source = osc1_source + osc2_mix + spectral_mix + additive_mix;
            let crossed_source = cross_mix_sample(direct.cross_mix_mode, osc1_source, osc2_mix)
                + spectral_mix
                + additive_mix;
            let cross_amount = direct.cross_mix_amount.clamp(0.0, 1.0);
            let ring_amount = direct.ring_mod_amount.clamp(0.0, 1.0);
            let source_mix = (summed_source * (1.0 - cross_amount) + crossed_source * cross_amount)
                * (1.0 - ring_amount)
                + (osc1_source * osc2_mix * 1.4) * ring_amount;
            let pre_filter = soft_clip(
                (source_mix + body_mix) * (pre_filter_gain + direct.filter_drive * 0.8),
                strain_bias,
            );

            let filter_env = voice.filter_env.next_sample();
            let keytrack = (1.0 + ((note as f32 - 60.0) / 48.0) * direct.filter_tracking * 0.42)
                .clamp(0.55, 1.35);
            let velocity_level = shaped_velocity_level(voice.velocity);
            let velocity_filter_curve = shaped_velocity_filter(voice.velocity);
            let velocity_filter = 1.0 + velocity_filter_curve * note_velocity_to_filter * 0.85;
            let cutoff_hz = (direct.cutoff_hz
                * (0.42 + filter_env * direct.filter_env_depth * 0.78)
                * keytrack
                * velocity_filter)
                .clamp(20.0, sample_rate_hz * 0.42);
            let filtered = voice.filter.process(
                pre_filter,
                cutoff_hz,
                direct.resonance,
                direct.filter_drive,
                derived.strain,
                sample_rate_hz,
            );

            let amp = voice.amp_env.next_sample();
            let velocity_gain =
                1.0 - note_velocity_to_level + velocity_level * note_velocity_to_level;
            let body_noise =
                colored_noise * direct.noise_body_level * (0.16 + derived.body_focus * 0.16);
            let mut sample = (filtered + body_noise) * amp * velocity_gain * voice_level_gain;
            sample = soft_clip(sample * strain_drive, direct.final_asymmetry * 0.14);

            if voice.phase != VoicePhase::Held && voice.amp_env.is_idle() {
                *voice = VoiceState::idle(sample_rate_hz, slot);
                continue;
            }

            let pan = spread_position * stereo_width;
            let left_gain = ((1.0 - pan) * 0.5).clamp(0.0, 1.0).sqrt();
            let right_gain = ((1.0 + pan) * 0.5).clamp(0.0, 1.0).sqrt();
            left += sample * left_gain;
            right += sample * right_gain;
        }

        self.final_body_left += (left - self.final_body_left) * (0.022 + derived.mass * 0.026);
        self.final_body_right += (right - self.final_body_right) * (0.022 + derived.mass * 0.026);

        let raw_mid = (left + right) * 0.5;
        let raw_side = (left - right) * 0.5;
        let body_mid = (self.final_body_left + self.final_body_right) * 0.5;
        let focus_amount = (derived.body_focus * 0.26
            + identity.grav_pull * 0.22
            + direct.stereo_crossfeed * 0.18)
            .clamp(0.0, 0.75);
        let saturated_mid = soft_clip(
            (raw_mid + body_mid * (0.30 + direct.low_mid_emphasis * 0.58))
                * (1.0 + direct.body_drive * 1.75 + direct.final_saturation * 1.25),
            direct.final_asymmetry * 0.70,
        );
        let saturated_side = soft_clip(
            raw_side * (1.0 + direct.final_saturation * 0.28),
            -direct.final_asymmetry * 0.18,
        ) * (1.0 - focus_amount);

        left = saturated_mid + saturated_side;
        right = saturated_mid - saturated_side;

        if direct.chorus_enabled {
            (left, right) = self.chorus.process(
                left,
                right,
                direct.chorus_mix,
                direct.chorus_depth,
                direct.chorus_rate_hz,
            );
        }

        if direct.reverb_enabled {
            (left, right) = self.reverb.process(
                left,
                right,
                direct.reverb_mix,
                direct.reverb_size,
                direct.reverb_damping,
            );
        }

        (left, right) = self.apply_gfm_layer(left, right, direct);
        (left, right) = self.apply_bcs_layer(left, right, direct);

        let crossfeed = (0.04 + direct.stereo_crossfeed * 0.16).clamp(0.0, 0.22);
        let crossfed_left = left * (1.0 - crossfeed) + right * crossfeed;
        let crossfed_right = right * (1.0 - crossfeed) + left * crossfeed;
        let output_gain = db_to_gain(direct.output_trim_db);

        (crossfed_left * output_gain, crossfed_right * output_gain)
    }

    fn apply_gfm_layer(&mut self, left: f32, right: f32, direct: DirectParameters) -> (f32, f32) {
        let Some(program_id) = self.gfm_layer_voice.as_ref().map(GfmFieldVoice::program_id) else {
            return (left, right);
        };

        let pressure = self.gfm_layer_pressure.next_value();
        let effective_amount = self.gfm_layer_mix.next_value();
        let gfm_sample = {
            let Some(voice) = self.gfm_layer_voice.as_mut() else {
                return (left, right);
            };
            voice.next_sample_with_live_pressure(pressure)
        };
        if effective_amount <= f32::EPSILON {
            self.finish_pending_auto_disarm_if_silent();
            return (left, right);
        }
        let activity = ((left.abs() + right.abs()) * 0.75).clamp(0.0, 1.0);
        let gate = if activity <= 0.000_01 {
            0.0
        } else {
            (0.20 + activity * 0.80).clamp(0.0, 1.0)
        };
        let layer = gfm_sample * gate * gfm_engine_layer_gain(program_id) * effective_amount;
        let spread = (direct.stereo_width * 0.16 + direct.stereo_crossfeed * 0.04).clamp(0.0, 0.22);
        let left = soft_clip(left + layer * (1.0 - spread), direct.final_asymmetry * 0.20);
        let right = soft_clip(
            right + layer * (1.0 + spread),
            -direct.final_asymmetry * 0.20,
        );

        (left, right)
    }

    fn apply_bcs_layer(&mut self, left: f32, right: f32, direct: DirectParameters) -> (f32, f32) {
        let BcsLayerMode::Enabled { scenario } = self.bcs_layer_mode else {
            return (left, right);
        };

        let effective_amount = self.bcs_layer_mix.next_value();
        let Some(voice) = self.bcs_layer_voice.as_mut() else {
            return (left, right);
        };

        let bcs_sample = voice.next_sample(BcsGesture::new(scenario));
        if voice.unsafe_state() || !bcs_sample.is_finite() {
            return (left, right);
        }
        if effective_amount <= f32::EPSILON {
            return (left, right);
        }

        let activity = ((left.abs() + right.abs()) * 0.75).clamp(0.0, 1.0);
        if activity <= 0.000_01 {
            return (left, right);
        }

        let gate = (0.18 + activity * 0.82).clamp(0.0, 1.0);
        let layer_gain = BCS_ENGINE_LAYER_GAIN * effective_amount;
        let layer = bcs_sample * layer_gain * gate;
        let spread = (direct.stereo_width * 0.10 + direct.stereo_crossfeed * 0.03).clamp(0.0, 0.16);
        let left = soft_clip(left + layer * (1.0 - spread), direct.final_asymmetry * 0.16);
        let right = soft_clip(
            right + layer * (1.0 + spread),
            -direct.final_asymmetry * 0.16,
        );

        (left, right)
    }

    fn apply_direct_param(&mut self, id: ParamId, value: f32) {
        let spec = param_spec(id);
        let clamped = value.clamp(spec.min, spec.max);

        match id {
            ParamId::GravitacijaMacro
            | ParamId::BloomMacro
            | ParamId::HeatMacro
            | ParamId::RuinMacro
            | ParamId::SwarmMacro => {
                let macro_id = spec.macro_id.unwrap_or(MacroId::Gravitacija);
                self.live_macros.set(macro_id, clamped);
                self.patch.macros =
                    macro_defaults_with_override(&self.patch.macros, macro_id, clamped);
            }
            ParamId::Osc1SawLevel => self.patch.engine.osc1.saw_level = clamped,
            ParamId::Osc1PulseLevel => self.patch.engine.osc1.pulse_level = clamped,
            ParamId::Osc1TriangleLevel => self.patch.engine.osc1.triangle_level = clamped,
            ParamId::Osc1NoiseLevel => {
                self.patch.engine.osc1.noise_level = clamped;
                self.patch.engine.noise_filter_level = Some(clamped);
            }
            ParamId::Osc1FineTuneCents => self.patch.engine.osc1.fine_tune_cents = Some(clamped),
            ParamId::Osc1PulseWidth => self.patch.engine.osc1.pulse_width = clamped,
            ParamId::Osc1PwmDepth => self.patch.engine.osc1.pwm_depth = clamped,
            ParamId::Osc1PhaseMode => {
                self.patch.engine.osc1.phase_mode = phase_mode_from_index(clamped)
            }
            ParamId::Osc1StartPhase => self.patch.engine.osc1.start_phase = clamped,
            ParamId::Osc1SawBend => self.patch.engine.osc1.saw_bend = clamped,
            ParamId::Osc1TriangleFold => self.patch.engine.osc1.triangle_fold = clamped,
            ParamId::Osc1PulseEdge => self.patch.engine.osc1.pulse_edge = clamped,
            ParamId::Osc2SawLevel => self.patch.engine.osc2.saw_level = clamped,
            ParamId::Osc2PulseLevel => self.patch.engine.osc2.pulse_level = clamped,
            ParamId::Osc2TriangleLevel => self.patch.engine.osc2.triangle_level = clamped,
            ParamId::Osc2IntervalSemitones => {
                self.patch.engine.osc2.interval_semitones = clamped.round() as i8
            }
            ParamId::Osc2FineTuneCents => self.patch.engine.osc2.fine_tune_cents = clamped,
            ParamId::Osc2SyncAmount => self.patch.engine.osc2.sync_amount = clamped,
            ParamId::Osc2CrossmodAmount => self.patch.engine.osc2.crossmod_amount = clamped,
            ParamId::Osc2PulseWidth => self.patch.engine.osc2.pulse_width = clamped,
            ParamId::Osc2PwmDepth => self.patch.engine.osc2.pwm_depth = clamped,
            ParamId::Osc2PhaseMode => {
                self.patch.engine.osc2.phase_mode = phase_mode_from_index(clamped)
            }
            ParamId::Osc2StartPhase => self.patch.engine.osc2.start_phase = clamped,
            ParamId::Osc2Level => self.patch.engine.osc2.level = clamped,
            ParamId::Osc2PitchMode => {
                self.patch.engine.osc2.pitch_mode = osc2_pitch_mode_from_index(clamped)
            }
            ParamId::Osc2Ratio => self.patch.engine.osc2.ratio = clamped,
            ParamId::Osc2SawBend => self.patch.engine.osc2.saw_bend = clamped,
            ParamId::Osc2TriangleFold => self.patch.engine.osc2.triangle_fold = clamped,
            ParamId::Osc2PulseEdge => self.patch.engine.osc2.pulse_edge = clamped,
            ParamId::SpectralLevel => self.patch.engine.spectral.level = clamped,
            ParamId::SpectralTable => {
                self.patch.engine.spectral.table = spectral_table_from_index(clamped)
            }
            ParamId::SpectralPosition => self.patch.engine.spectral.position = clamped,
            ParamId::SpectralMorph => self.patch.engine.spectral.morph = clamped,
            ParamId::SpectralRatio => self.patch.engine.spectral.ratio = clamped,
            ParamId::SpectralFineTuneCents => self.patch.engine.spectral.fine_tune_cents = clamped,
            ParamId::AdditiveLevel => self.patch.engine.additive.level = clamped,
            ParamId::AdditivePartialCount => {
                self.patch.engine.additive.partial_count = clamped.round().clamp(4.0, 8.0) as u8
            }
            ParamId::AdditiveHarmonicSpread => self.patch.engine.additive.harmonic_spread = clamped,
            ParamId::AdditiveOddEvenBalance => {
                self.patch.engine.additive.odd_even_balance = clamped
            }
            ParamId::AdditiveInharmonicity => self.patch.engine.additive.inharmonicity = clamped,
            ParamId::AdditiveSpectralTilt => self.patch.engine.additive.spectral_tilt = clamped,
            ParamId::AdditiveRandomDetuneCents => {
                self.patch.engine.additive.random_detune_cents = clamped
            }
            ParamId::SourcePwmRateHz => self.patch.engine.source_pwm_rate_hz = clamped,
            ParamId::NoiseColor => self.patch.engine.noise_color = noise_color_from_index(clamped),
            ParamId::NoiseFilterLevel => self.patch.engine.noise_filter_level = Some(clamped),
            ParamId::NoiseBodyLevel => self.patch.engine.noise_body_level = clamped,
            ParamId::AnalogDrift => self.patch.engine.analog_drift = clamped,
            ParamId::MicroJitter => self.patch.engine.micro_jitter = clamped,
            ParamId::FmAmount => self.patch.engine.fm_amount = clamped,
            ParamId::FmDirection => {
                self.patch.engine.fm_direction = mod_direction_from_index(clamped)
            }
            ParamId::PhaseModAmount => self.patch.engine.phase_mod_amount = clamped,
            ParamId::PhaseModDirection => {
                self.patch.engine.phase_mod_direction = mod_direction_from_index(clamped)
            }
            ParamId::RingModAmount => self.patch.engine.ring_mod_amount = clamped,
            ParamId::AmAmount => self.patch.engine.am_amount = clamped,
            ParamId::SyncDirection => {
                self.patch.engine.sync_direction = mod_direction_from_index(clamped)
            }
            ParamId::SyncSoftness => self.patch.engine.sync_softness = clamped,
            ParamId::CrossMixMode => {
                self.patch.engine.cross_mix_mode = cross_mix_mode_from_index(clamped)
            }
            ParamId::CrossMixAmount => self.patch.engine.cross_mix_amount = clamped,
            ParamId::SubLevel => self.patch.engine.sub.level = clamped,
            ParamId::SubOctaveOffset => self.patch.engine.sub.octave_offset = clamped.round() as i8,
            ParamId::MixerPreFilterDrive => self.patch.engine.mixer.pre_filter_drive = clamped,
            ParamId::MixerBodyMix => self.patch.engine.mixer.body_mix = clamped,
            ParamId::FilterCutoffHz => self.patch.engine.filter.cutoff_hz = clamped,
            ParamId::FilterResonance => self.patch.engine.filter.resonance = clamped,
            ParamId::FilterDrive => self.patch.engine.filter.drive = clamped,
            ParamId::FilterKeytrack => self.patch.engine.filter.keytrack = clamped,
            ParamId::AmpEnvAttackMs => self.patch.engine.amp_env.attack_ms = clamped,
            ParamId::AmpEnvDecayMs => self.patch.engine.amp_env.decay_ms = clamped,
            ParamId::AmpEnvSustain => self.patch.engine.amp_env.sustain = clamped,
            ParamId::AmpEnvReleaseMs => self.patch.engine.amp_env.release_ms = clamped,
            ParamId::FilterEnvAttackMs => self.patch.engine.filter_env.adsr.attack_ms = clamped,
            ParamId::FilterEnvDecayMs => self.patch.engine.filter_env.adsr.decay_ms = clamped,
            ParamId::FilterEnvSustain => self.patch.engine.filter_env.adsr.sustain = clamped,
            ParamId::FilterEnvReleaseMs => self.patch.engine.filter_env.adsr.release_ms = clamped,
            ParamId::FilterEnvDepth => self.patch.engine.filter_env.depth = clamped,
            ParamId::VoiceStereoWidth => self.patch.engine.voice.stereo_width = clamped,
            ParamId::VoiceDetuneSpreadCents => {
                self.patch.engine.voice.detune_spread_cents = clamped
            }
            ParamId::VoiceVelocityToLevel => {
                self.patch.engine.voice.velocity_to_level = Some(clamped)
            }
            ParamId::VoiceVelocityToFilter => {
                self.patch.engine.voice.velocity_to_filter = Some(clamped)
            }
            ParamId::FinalStageBodyDrive => self.patch.engine.final_stage.body_drive = clamped,
            ParamId::FinalStageAsymmetry => self.patch.engine.final_stage.asymmetry = clamped,
            ParamId::FinalStageLowMidEmphasis => {
                self.patch.engine.final_stage.low_mid_emphasis = clamped
            }
            ParamId::FinalStageOutputTrimDb => {
                self.patch.engine.final_stage.output_trim_db = clamped
            }
            ParamId::ChorusEnabled => self.patch.engine.fx.chorus.enabled = clamped >= 0.5,
            ParamId::ChorusMix => self.patch.engine.fx.chorus.mix = clamped,
            ParamId::ChorusDepth => self.patch.engine.fx.chorus.depth = clamped,
            ParamId::ChorusRateHz => self.patch.engine.fx.chorus.rate_hz = clamped,
            ParamId::ReverbEnabled => self.patch.engine.fx.reverb.enabled = clamped >= 0.5,
            ParamId::ReverbMix => self.patch.engine.fx.reverb.mix = clamped,
            ParamId::ReverbSize => self.patch.engine.fx.reverb.size = clamped,
            ParamId::ReverbDamping => self.patch.engine.fx.reverb.damping = clamped,
            ParamId::PerformanceVelocityToLevel => {
                self.patch.performance_response.velocity_to_level = clamped
            }
            ParamId::PerformanceVelocityToFilter => {
                self.patch.performance_response.velocity_to_filter = clamped
            }
            ParamId::PerformanceAftertouchToGravitacija => {
                self.patch.performance_response.aftertouch_to_gravitacija = clamped
            }
            ParamId::PerformanceAftertouchToBaklja => {
                self.patch.performance_response.aftertouch_to_baklja = clamped
            }
            ParamId::PerformanceModWheelToBloom => {
                self.patch.performance_response.mod_wheel_to_bloom = clamped
            }
            ParamId::PerformanceModWheelToSwarm => {
                self.patch.performance_response.mod_wheel_to_swarm = clamped
            }
        }
    }
}

fn phase_mode_from_index(value: f32) -> OscPhaseMode {
    match value.round() as i32 {
        1 => OscPhaseMode::Fixed,
        2 => OscPhaseMode::FreeRun,
        _ => OscPhaseMode::Deterministic,
    }
}

fn osc2_pitch_mode_from_index(value: f32) -> Osc2PitchMode {
    if value.round() as i32 >= 1 {
        Osc2PitchMode::Ratio
    } else {
        Osc2PitchMode::Semitone
    }
}

fn noise_color_from_index(value: f32) -> NoiseColor {
    match value.round() as i32 {
        1 => NoiseColor::Pinkish,
        2 => NoiseColor::Dark,
        3 => NoiseColor::Bright,
        _ => NoiseColor::White,
    }
}

fn spectral_table_from_index(value: f32) -> SpectralTable {
    match value.round() as i32 {
        1 => SpectralTable::Vocalish,
        2 => SpectralTable::Metallic,
        3 => SpectralTable::Hollow,
        4 => SpectralTable::Formant,
        _ => SpectralTable::Sineish,
    }
}

fn mod_direction_from_index(value: f32) -> ModDirection {
    if value.round() as i32 >= 1 {
        ModDirection::Osc2ToOsc1
    } else {
        ModDirection::Osc1ToOsc2
    }
}

fn cross_mix_mode_from_index(value: f32) -> CrossMixMode {
    match value.round() as i32 {
        1 => CrossMixMode::Multiply,
        2 => CrossMixMode::Fold,
        3 => CrossMixMode::Max,
        4 => CrossMixMode::Difference,
        _ => CrossMixMode::Sum,
    }
}
