use super::*;

impl Engine {
    pub(super) fn reset_runtime_state(&mut self) {
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

    pub(super) fn refresh_resolved_state(&mut self) {
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

    pub(super) fn rebuild_gfm_layer(&mut self) {
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

    pub(super) fn rebuild_bcs_layer(&mut self) {
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

    pub(super) fn refresh_bcs_layer_pitch(&mut self) {
        let desired = self.current_bcs_layer_pitch();
        match desired {
            Some((note, frequency_hz)) => {
                let changed = self.bcs_layer_note != Some(note)
                    || self
                        .bcs_layer_frequency_hz
                        .is_none_or(|current| (current - frequency_hz).abs() > 0.001);
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

    pub(super) fn current_bcs_layer_pitch(&self) -> Option<(u8, f32)> {
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

    pub(super) fn gfm_layer_snapshot(&self) -> GfmLayerSnapshot {
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

    pub(super) fn bcs_layer_snapshot(&self) -> BcsLayerSnapshot {
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
}
