use super::*;

impl Engine {
    pub(super) fn apply_direct_param(&mut self, id: ParamId, value: f32) {
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
            ParamId::Osc1Bandlimit => self.patch.engine.osc1.bandlimit = clamped,
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
            ParamId::Osc2Bandlimit => self.patch.engine.osc2.bandlimit = clamped,
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
            ParamId::FilterModel => {
                self.patch.engine.filter.model = filter_model_from_index(clamped)
            }
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
