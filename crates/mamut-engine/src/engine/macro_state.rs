use super::*;

impl Engine {
    pub(super) fn effective_macro_state(&self) -> MacroState {
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
            let pwm_lfo = voice.pwm_lfo.next_bipolar(direct.source_pwm_rate_hz);
            let drift_lfo = voice.drift_lfo.next_bipolar(0.083);
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
            let (left_gain, right_gain) = equal_power_pan(pan);
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
}
