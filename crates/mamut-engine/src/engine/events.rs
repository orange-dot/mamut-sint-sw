use super::*;

impl Engine {
    pub(super) fn handle_note_event(&mut self, event: NoteEvent) {
        match event {
            NoteEvent::NoteOn { note, velocity } if velocity > 0.0 => self.note_on(note, velocity),
            NoteEvent::NoteOn { note, .. } | NoteEvent::NoteOff { note } => self.note_off(note),
        }
    }

    pub(super) fn handle_controller_event(&mut self, event: ControllerEvent) {
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
                let gfm_pressure = normalize_gfm_layer_control(pressure);
                self.control.aftertouch = pressure;
                self.control.gfm_layer_pressure = gfm_pressure;
                if gfm_pressure > f32::EPSILON {
                    self.maybe_auto_enable_gfm_layer_for_momentary_control();
                } else {
                    self.begin_auto_disarm_gfm_layer_if_momentary_control_released();
                }
                self.update_gfm_layer_smoothing_targets();
            }
            ControllerEvent::GfmLayerAmount { amount } => {
                let amount = normalize_gfm_layer_control(amount);
                self.control.gfm_layer_amount = amount;
                if amount > f32::EPSILON {
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

    pub(super) fn note_on(&mut self, note: u8, velocity: f32) {
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
        self.configure_mozaik_voice_on_trigger(voice_index);
        self.strike_gfm_layer_note_on(note, velocity);
        self.refresh_bcs_layer_pitch();
    }

    pub(super) fn note_off(&mut self, note: u8) {
        if let Some(index) = self
            .voices
            .iter()
            .enumerate()
            .filter(|(_, voice)| voice.note == Some(note) && voice.phase == VoicePhase::Held)
            .min_by_key(|(_, voice)| voice.age)
            .map(|(index, _)| index)
        {
            let release_velocity = {
                let voice = &mut self.voices[index];
                if self.control.sustain_down {
                    voice.phase = VoicePhase::SustainedReleased;
                } else {
                    voice.start_release();
                }
                voice.velocity
            };
            self.strike_gfm_layer_note_off(note, release_velocity);
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

    pub(super) fn strike_gfm_layer_note_on(&mut self, note: u8, velocity: f32) {
        if !self.gfm_note_strikes_enabled {
            return;
        }
        if let Some(voice) = self.gfm_layer_voice.as_mut() {
            voice.strike_note_on(note, velocity);
        }
    }

    pub(super) fn strike_gfm_layer_note_off(&mut self, note: u8, velocity: f32) {
        if !self.gfm_note_strikes_enabled {
            return;
        }
        if let Some(voice) = self.gfm_layer_voice.as_mut() {
            voice.strike_note_off(note, velocity);
        }
    }

    pub(super) fn allocate_voice_index(&self) -> usize {
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
}
