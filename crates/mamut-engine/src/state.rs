use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoicePhase {
    Idle,
    Held,
    Released,
    SustainedReleased,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoiceSnapshot {
    pub slot: usize,
    pub note: Option<u8>,
    pub velocity: f32,
    pub phase: VoicePhase,
    pub age: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectParameters {
    pub osc1_wave_mix: [f32; 4],
    pub osc1_fine_tune_cents: f32,
    pub osc1_pulse_width: f32,
    pub osc1_pwm_depth: f32,
    pub osc1_phase_mode: f32,
    pub osc1_start_phase: f32,
    pub osc1_saw_bend: f32,
    pub osc1_triangle_fold: f32,
    pub osc1_pulse_edge: f32,
    pub osc1_bandlimit: f32,
    pub osc2_wave_mix: [f32; 3],
    pub osc2_fine_tune_cents: f32,
    pub osc2_pulse_width: f32,
    pub osc2_pwm_depth: f32,
    pub osc2_phase_mode: f32,
    pub osc2_start_phase: f32,
    pub osc2_level: f32,
    pub osc2_pitch_mode: f32,
    pub osc2_ratio: f32,
    pub osc2_saw_bend: f32,
    pub osc2_triangle_fold: f32,
    pub osc2_pulse_edge: f32,
    pub osc2_bandlimit: f32,
    pub spectral_level: f32,
    pub spectral_table: f32,
    pub spectral_position: f32,
    pub spectral_morph: f32,
    pub spectral_ratio: f32,
    pub spectral_fine_tune_cents: f32,
    pub additive_level: f32,
    pub additive_partial_count: f32,
    pub additive_harmonic_spread: f32,
    pub additive_odd_even_balance: f32,
    pub additive_inharmonicity: f32,
    pub additive_spectral_tilt: f32,
    pub additive_random_detune_cents: f32,
    pub source_pwm_rate_hz: f32,
    pub noise_color: f32,
    pub noise_filter_level: f32,
    pub noise_body_level: f32,
    pub analog_drift: f32,
    pub micro_jitter: f32,
    pub fm_amount: f32,
    pub fm_direction: f32,
    pub phase_mod_amount: f32,
    pub phase_mod_direction: f32,
    pub ring_mod_amount: f32,
    pub am_amount: f32,
    pub sync_direction: f32,
    pub sync_softness: f32,
    pub cross_mix_mode: f32,
    pub cross_mix_amount: f32,
    pub sub_level: f32,
    pub sub_octave_offset: f32,
    pub mixer_pre_filter_drive: f32,
    pub mixer_body_mix: f32,
    pub osc2_interval_semitones: f32,
    pub sync_amount: f32,
    pub crossmod_amount: f32,
    pub detune_spread_cents: f32,
    pub cutoff_hz: f32,
    pub resonance: f32,
    pub filter_drive: f32,
    pub filter_env_depth: f32,
    pub filter_tracking: f32,
    pub amp_env: AdsrTiming,
    pub filter_env: AdsrTiming,
    pub voice_level: f32,
    pub voice_velocity_to_level: f32,
    pub voice_velocity_to_filter: f32,
    pub body_drive: f32,
    pub output_trim_db: f32,
    pub stereo_width: f32,
    pub stereo_crossfeed: f32,
    pub final_saturation: f32,
    pub final_asymmetry: f32,
    pub low_mid_emphasis: f32,
    pub chorus_enabled: bool,
    pub chorus_mix: f32,
    pub chorus_depth: f32,
    pub chorus_rate_hz: f32,
    pub reverb_enabled: bool,
    pub reverb_mix: f32,
    pub reverb_size: f32,
    pub reverb_damping: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PerformanceResponseSnapshot {
    pub velocity_to_level: f32,
    pub velocity_to_filter: f32,
    pub aftertouch_to_gravitacija: f32,
    pub aftertouch_to_baklja: f32,
    pub mod_wheel_to_bloom: f32,
    pub mod_wheel_to_swarm: f32,
    pub bend_range_semitones: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EngineSnapshot {
    pub patch_name: String,
    pub patch_description: Option<String>,
    pub patch_tags: Vec<String>,
    pub patch_favorite: bool,
    pub sample_rate_hz: f32,
    pub last_block_frames: usize,
    pub events_processed: usize,
    pub active_voice_count: usize,
    pub sustain_down: bool,
    pub live_macros: MacroState,
    pub effective_macros: MacroState,
    pub identity: IdentityState,
    pub derived: DerivedState,
    pub direct: DirectParameters,
    pub performance_response: PerformanceResponseSnapshot,
    pub gfm_layer: GfmLayerSnapshot,
    pub bcs_layer: BcsLayerSnapshot,
    pub voices: Vec<VoiceSnapshot>,
    pub held_notes: Vec<u8>,
    pub output_safety: OutputSafetySnapshot,
    pub peak_output: f32,
    pub clip_detected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct OutputSafetySnapshot {
    pub pre_safety_peak: f32,
    pub post_safety_peak: f32,
    pub safety_limiter_hits: u64,
    pub max_safety_reduction: f32,
    pub tiny_flush_events: u64,
}

impl OutputSafetySnapshot {
    pub(crate) fn observe_channel(&mut self, pre_safety: f32, post_limiter: f32) {
        if pre_safety.is_finite()
            && pre_safety.abs() > MASTER_SAFETY_KNEE
            && (pre_safety - post_limiter).abs() > f32::EPSILON
        {
            self.safety_limiter_hits += 1;
            self.max_safety_reduction = self
                .max_safety_reduction
                .max((pre_safety.abs() - post_limiter.abs()).max(0.0));
        }

        if pre_safety.is_finite()
            && pre_safety != 0.0
            && pre_safety.abs() < DENORMAL_FLUSH_ABS
            && post_limiter == 0.0
        {
            self.tiny_flush_events += 1;
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ControlState {
    pub(crate) pitch_bend_semitones: f32,
    pub(crate) mod_wheel: f32,
    pub(crate) sustain_down: bool,
    pub(crate) aftertouch: f32,
    pub(crate) gfm_layer_pressure: f32,
    pub(crate) gfm_layer_amount: f32,
    pub(crate) bcs_layer_enabled: bool,
    pub(crate) bcs_layer_amount: f32,
}

impl Default for ControlState {
    fn default() -> Self {
        Self {
            pitch_bend_semitones: 0.0,
            mod_wheel: 0.0,
            sustain_down: false,
            aftertouch: 0.0,
            gfm_layer_pressure: 0.0,
            gfm_layer_amount: 0.0,
            bcs_layer_enabled: false,
            bcs_layer_amount: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RenderSmoothers {
    pub(crate) osc1_pulse_width: LinearSmoother,
    pub(crate) osc1_pwm_depth: LinearSmoother,
    pub(crate) osc1_saw_bend: LinearSmoother,
    pub(crate) osc1_triangle_fold: LinearSmoother,
    pub(crate) osc1_pulse_edge: LinearSmoother,
    pub(crate) osc1_bandlimit: LinearSmoother,
    pub(crate) osc2_pulse_width: LinearSmoother,
    pub(crate) osc2_pwm_depth: LinearSmoother,
    pub(crate) osc2_level: LinearSmoother,
    pub(crate) osc2_ratio: LinearSmoother,
    pub(crate) osc2_saw_bend: LinearSmoother,
    pub(crate) osc2_triangle_fold: LinearSmoother,
    pub(crate) osc2_pulse_edge: LinearSmoother,
    pub(crate) osc2_bandlimit: LinearSmoother,
    pub(crate) spectral_level: LinearSmoother,
    pub(crate) spectral_position: LinearSmoother,
    pub(crate) spectral_morph: LinearSmoother,
    pub(crate) spectral_ratio: LinearSmoother,
    pub(crate) spectral_fine_tune_cents: LinearSmoother,
    pub(crate) additive_level: LinearSmoother,
    pub(crate) additive_harmonic_spread: LinearSmoother,
    pub(crate) additive_odd_even_balance: LinearSmoother,
    pub(crate) additive_inharmonicity: LinearSmoother,
    pub(crate) additive_spectral_tilt: LinearSmoother,
    pub(crate) additive_random_detune_cents: LinearSmoother,
    pub(crate) noise_filter_level: LinearSmoother,
    pub(crate) noise_body_level: LinearSmoother,
    pub(crate) analog_drift: LinearSmoother,
    pub(crate) micro_jitter: LinearSmoother,
    pub(crate) fm_amount: LinearSmoother,
    pub(crate) phase_mod_amount: LinearSmoother,
    pub(crate) ring_mod_amount: LinearSmoother,
    pub(crate) am_amount: LinearSmoother,
    pub(crate) sync_softness: LinearSmoother,
    pub(crate) cross_mix_amount: LinearSmoother,
    pub(crate) sync_amount: LinearSmoother,
    pub(crate) crossmod_amount: LinearSmoother,
    pub(crate) cutoff_hz: LinearSmoother,
    pub(crate) resonance: LinearSmoother,
    pub(crate) filter_drive: LinearSmoother,
    pub(crate) voice_level: LinearSmoother,
    pub(crate) body_drive: LinearSmoother,
    pub(crate) stereo_width: LinearSmoother,
    pub(crate) stereo_crossfeed: LinearSmoother,
    pub(crate) final_saturation: LinearSmoother,
    pub(crate) final_asymmetry: LinearSmoother,
    pub(crate) low_mid_emphasis: LinearSmoother,
}

impl RenderSmoothers {
    pub(crate) fn new(direct: DirectParameters) -> Self {
        Self {
            osc1_pulse_width: LinearSmoother::new(direct.osc1_pulse_width),
            osc1_pwm_depth: LinearSmoother::new(direct.osc1_pwm_depth),
            osc1_saw_bend: LinearSmoother::new(direct.osc1_saw_bend),
            osc1_triangle_fold: LinearSmoother::new(direct.osc1_triangle_fold),
            osc1_pulse_edge: LinearSmoother::new(direct.osc1_pulse_edge),
            osc1_bandlimit: LinearSmoother::new(direct.osc1_bandlimit),
            osc2_pulse_width: LinearSmoother::new(direct.osc2_pulse_width),
            osc2_pwm_depth: LinearSmoother::new(direct.osc2_pwm_depth),
            osc2_level: LinearSmoother::new(direct.osc2_level),
            osc2_ratio: LinearSmoother::new(direct.osc2_ratio),
            osc2_saw_bend: LinearSmoother::new(direct.osc2_saw_bend),
            osc2_triangle_fold: LinearSmoother::new(direct.osc2_triangle_fold),
            osc2_pulse_edge: LinearSmoother::new(direct.osc2_pulse_edge),
            osc2_bandlimit: LinearSmoother::new(direct.osc2_bandlimit),
            spectral_level: LinearSmoother::new(direct.spectral_level),
            spectral_position: LinearSmoother::new(direct.spectral_position),
            spectral_morph: LinearSmoother::new(direct.spectral_morph),
            spectral_ratio: LinearSmoother::new(direct.spectral_ratio),
            spectral_fine_tune_cents: LinearSmoother::new(direct.spectral_fine_tune_cents),
            additive_level: LinearSmoother::new(direct.additive_level),
            additive_harmonic_spread: LinearSmoother::new(direct.additive_harmonic_spread),
            additive_odd_even_balance: LinearSmoother::new(direct.additive_odd_even_balance),
            additive_inharmonicity: LinearSmoother::new(direct.additive_inharmonicity),
            additive_spectral_tilt: LinearSmoother::new(direct.additive_spectral_tilt),
            additive_random_detune_cents: LinearSmoother::new(direct.additive_random_detune_cents),
            noise_filter_level: LinearSmoother::new(direct.noise_filter_level),
            noise_body_level: LinearSmoother::new(direct.noise_body_level),
            analog_drift: LinearSmoother::new(direct.analog_drift),
            micro_jitter: LinearSmoother::new(direct.micro_jitter),
            fm_amount: LinearSmoother::new(direct.fm_amount),
            phase_mod_amount: LinearSmoother::new(direct.phase_mod_amount),
            ring_mod_amount: LinearSmoother::new(direct.ring_mod_amount),
            am_amount: LinearSmoother::new(direct.am_amount),
            sync_softness: LinearSmoother::new(direct.sync_softness),
            cross_mix_amount: LinearSmoother::new(direct.cross_mix_amount),
            sync_amount: LinearSmoother::new(direct.sync_amount),
            crossmod_amount: LinearSmoother::new(direct.crossmod_amount),
            cutoff_hz: LinearSmoother::new(direct.cutoff_hz),
            resonance: LinearSmoother::new(direct.resonance),
            filter_drive: LinearSmoother::new(direct.filter_drive),
            voice_level: LinearSmoother::new(direct.voice_level),
            body_drive: LinearSmoother::new(direct.body_drive),
            stereo_width: LinearSmoother::new(direct.stereo_width),
            stereo_crossfeed: LinearSmoother::new(direct.stereo_crossfeed),
            final_saturation: LinearSmoother::new(direct.final_saturation),
            final_asymmetry: LinearSmoother::new(direct.final_asymmetry),
            low_mid_emphasis: LinearSmoother::new(direct.low_mid_emphasis),
        }
    }

    pub(crate) fn set_targets(&mut self, direct: DirectParameters, sample_count: usize) {
        self.osc1_pulse_width
            .set_target(direct.osc1_pulse_width, sample_count);
        self.osc1_pwm_depth
            .set_target(direct.osc1_pwm_depth, sample_count);
        self.osc1_saw_bend
            .set_target(direct.osc1_saw_bend, sample_count);
        self.osc1_triangle_fold
            .set_target(direct.osc1_triangle_fold, sample_count);
        self.osc1_pulse_edge
            .set_target(direct.osc1_pulse_edge, sample_count);
        self.osc1_bandlimit
            .set_target(direct.osc1_bandlimit, sample_count);
        self.osc2_pulse_width
            .set_target(direct.osc2_pulse_width, sample_count);
        self.osc2_pwm_depth
            .set_target(direct.osc2_pwm_depth, sample_count);
        self.osc2_level.set_target(direct.osc2_level, sample_count);
        self.osc2_ratio.set_target(direct.osc2_ratio, sample_count);
        self.osc2_saw_bend
            .set_target(direct.osc2_saw_bend, sample_count);
        self.osc2_triangle_fold
            .set_target(direct.osc2_triangle_fold, sample_count);
        self.osc2_pulse_edge
            .set_target(direct.osc2_pulse_edge, sample_count);
        self.osc2_bandlimit
            .set_target(direct.osc2_bandlimit, sample_count);
        self.spectral_level
            .set_target(direct.spectral_level, sample_count);
        self.spectral_position
            .set_target(direct.spectral_position, sample_count);
        self.spectral_morph
            .set_target(direct.spectral_morph, sample_count);
        self.spectral_ratio
            .set_target(direct.spectral_ratio, sample_count);
        self.spectral_fine_tune_cents
            .set_target(direct.spectral_fine_tune_cents, sample_count);
        self.additive_level
            .set_target(direct.additive_level, sample_count);
        self.additive_harmonic_spread
            .set_target(direct.additive_harmonic_spread, sample_count);
        self.additive_odd_even_balance
            .set_target(direct.additive_odd_even_balance, sample_count);
        self.additive_inharmonicity
            .set_target(direct.additive_inharmonicity, sample_count);
        self.additive_spectral_tilt
            .set_target(direct.additive_spectral_tilt, sample_count);
        self.additive_random_detune_cents
            .set_target(direct.additive_random_detune_cents, sample_count);
        self.noise_filter_level
            .set_target(direct.noise_filter_level, sample_count);
        self.noise_body_level
            .set_target(direct.noise_body_level, sample_count);
        self.analog_drift
            .set_target(direct.analog_drift, sample_count);
        self.micro_jitter
            .set_target(direct.micro_jitter, sample_count);
        self.fm_amount.set_target(direct.fm_amount, sample_count);
        self.phase_mod_amount
            .set_target(direct.phase_mod_amount, sample_count);
        self.ring_mod_amount
            .set_target(direct.ring_mod_amount, sample_count);
        self.am_amount.set_target(direct.am_amount, sample_count);
        self.sync_softness
            .set_target(direct.sync_softness, sample_count);
        self.cross_mix_amount
            .set_target(direct.cross_mix_amount, sample_count);
        self.sync_amount
            .set_target(direct.sync_amount, sample_count);
        self.crossmod_amount
            .set_target(direct.crossmod_amount, sample_count);
        self.cutoff_hz.set_target(direct.cutoff_hz, sample_count);
        self.resonance.set_target(direct.resonance, sample_count);
        self.filter_drive
            .set_target(direct.filter_drive, sample_count);
        self.voice_level
            .set_target(direct.voice_level, sample_count);
        self.body_drive.set_target(direct.body_drive, sample_count);
        self.stereo_width
            .set_target(direct.stereo_width, sample_count);
        self.stereo_crossfeed
            .set_target(direct.stereo_crossfeed, sample_count);
        self.final_saturation
            .set_target(direct.final_saturation, sample_count);
        self.final_asymmetry
            .set_target(direct.final_asymmetry, sample_count);
        self.low_mid_emphasis
            .set_target(direct.low_mid_emphasis, sample_count);
    }

    pub(crate) fn next_direct(&mut self, mut direct: DirectParameters) -> DirectParameters {
        direct.osc1_pulse_width = self.osc1_pulse_width.next_value();
        direct.osc1_pwm_depth = self.osc1_pwm_depth.next_value();
        direct.osc1_saw_bend = self.osc1_saw_bend.next_value();
        direct.osc1_triangle_fold = self.osc1_triangle_fold.next_value();
        direct.osc1_pulse_edge = self.osc1_pulse_edge.next_value();
        direct.osc1_bandlimit = self.osc1_bandlimit.next_value();
        direct.osc2_pulse_width = self.osc2_pulse_width.next_value();
        direct.osc2_pwm_depth = self.osc2_pwm_depth.next_value();
        direct.osc2_level = self.osc2_level.next_value();
        direct.osc2_ratio = self.osc2_ratio.next_value();
        direct.osc2_saw_bend = self.osc2_saw_bend.next_value();
        direct.osc2_triangle_fold = self.osc2_triangle_fold.next_value();
        direct.osc2_pulse_edge = self.osc2_pulse_edge.next_value();
        direct.osc2_bandlimit = self.osc2_bandlimit.next_value();
        direct.spectral_level = self.spectral_level.next_value();
        direct.spectral_position = self.spectral_position.next_value();
        direct.spectral_morph = self.spectral_morph.next_value();
        direct.spectral_ratio = self.spectral_ratio.next_value();
        direct.spectral_fine_tune_cents = self.spectral_fine_tune_cents.next_value();
        direct.additive_level = self.additive_level.next_value();
        direct.additive_harmonic_spread = self.additive_harmonic_spread.next_value();
        direct.additive_odd_even_balance = self.additive_odd_even_balance.next_value();
        direct.additive_inharmonicity = self.additive_inharmonicity.next_value();
        direct.additive_spectral_tilt = self.additive_spectral_tilt.next_value();
        direct.additive_random_detune_cents = self.additive_random_detune_cents.next_value();
        direct.noise_filter_level = self.noise_filter_level.next_value();
        direct.noise_body_level = self.noise_body_level.next_value();
        direct.analog_drift = self.analog_drift.next_value();
        direct.micro_jitter = self.micro_jitter.next_value();
        direct.fm_amount = self.fm_amount.next_value();
        direct.phase_mod_amount = self.phase_mod_amount.next_value();
        direct.ring_mod_amount = self.ring_mod_amount.next_value();
        direct.am_amount = self.am_amount.next_value();
        direct.sync_softness = self.sync_softness.next_value();
        direct.cross_mix_amount = self.cross_mix_amount.next_value();
        direct.sync_amount = self.sync_amount.next_value();
        direct.crossmod_amount = self.crossmod_amount.next_value();
        direct.cutoff_hz = self.cutoff_hz.next_value();
        direct.resonance = self.resonance.next_value();
        direct.filter_drive = self.filter_drive.next_value();
        direct.voice_level = self.voice_level.next_value();
        direct.body_drive = self.body_drive.next_value();
        direct.stereo_width = self.stereo_width.next_value();
        direct.stereo_crossfeed = self.stereo_crossfeed.next_value();
        direct.final_saturation = self.final_saturation.next_value();
        direct.final_asymmetry = self.final_asymmetry.next_value();
        direct.low_mid_emphasis = self.low_mid_emphasis.next_value();
        direct
    }
}

#[derive(Debug, Clone)]
pub(crate) struct VoiceState {
    pub(crate) note: Option<u8>,
    pub(crate) velocity: f32,
    pub(crate) phase: VoicePhase,
    pub(crate) age: u64,
    pub(crate) osc1: Oscillator,
    pub(crate) osc2: Oscillator,
    pub(crate) osc1_triangle: BandlimitedTriangle,
    pub(crate) osc2_triangle: BandlimitedTriangle,
    pub(crate) spectral: Oscillator,
    pub(crate) additive_partials: [Oscillator; 8],
    pub(crate) additive_detune_cents: [f32; 8],
    pub(crate) sub: Oscillator,
    pub(crate) pwm_lfo: Lfo,
    pub(crate) drift_lfo: Lfo,
    pub(crate) noise: NoiseRng,
    pub(crate) jitter: NoiseRng,
    pub(crate) noise_color_state: f32,
    pub(crate) filter: StateVariableFilter,
    pub(crate) amp_env: AdsrEnvelope,
    pub(crate) filter_env: AdsrEnvelope,
}

impl VoiceState {
    pub(crate) fn idle(sample_rate_hz: f32, slot: usize) -> Self {
        let seed = (0x1234_5678_u32).wrapping_add((slot as u32).wrapping_mul(0x9E37_79B9));
        Self {
            note: None,
            velocity: 0.0,
            phase: VoicePhase::Idle,
            age: 0,
            osc1: Oscillator::new(),
            osc2: Oscillator::new(),
            osc1_triangle: BandlimitedTriangle::default(),
            osc2_triangle: BandlimitedTriangle::default(),
            spectral: Oscillator::new(),
            additive_partials: [Oscillator::new(); 8],
            additive_detune_cents: [0.0; 8],
            sub: Oscillator::new(),
            pwm_lfo: Lfo::new(sample_rate_hz, LfoShape::Sine),
            drift_lfo: Lfo::new(sample_rate_hz, LfoShape::Sine),
            noise: NoiseRng::new(seed),
            jitter: NoiseRng::new(seed ^ 0xA5A5_5A5A),
            noise_color_state: 0.0,
            filter: StateVariableFilter::new(),
            amp_env: AdsrEnvelope::new(
                sample_rate_hz,
                AdsrTiming {
                    attack_ms: 10.0,
                    decay_ms: 50.0,
                    sustain: 1.0,
                    release_ms: 100.0,
                },
            ),
            filter_env: AdsrEnvelope::new(
                sample_rate_hz,
                AdsrTiming {
                    attack_ms: 10.0,
                    decay_ms: 50.0,
                    sustain: 1.0,
                    release_ms: 100.0,
                },
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn trigger(
        &mut self,
        note: u8,
        velocity: f32,
        age: u64,
        amp_env: AdsrTiming,
        filter_env: AdsrTiming,
        slot: usize,
        osc1_phase_mode: f32,
        osc1_start_phase: f32,
        osc2_phase_mode: f32,
        osc2_start_phase: f32,
        additive_random_detune_cents: f32,
    ) {
        self.note = Some(note);
        self.velocity = velocity.clamp(0.0, 1.0);
        self.phase = VoicePhase::Held;
        self.age = age;
        self.filter.reset();
        self.amp_env.set_timing(amp_env);
        self.filter_env.set_timing(filter_env);
        self.amp_env.note_on();
        self.filter_env.note_on();
        apply_phase_mode(
            &mut self.osc1,
            osc1_phase_mode,
            osc1_start_phase,
            (((slot as f32) * 0.137) + note as f32 * 0.017).fract(),
        );
        self.osc1_triangle.reset_to_phase(self.osc1.phase());
        apply_phase_mode(
            &mut self.osc2,
            osc2_phase_mode,
            osc2_start_phase,
            (((slot as f32) * 0.223) + note as f32 * 0.031).fract(),
        );
        self.osc2_triangle.reset_to_phase(self.osc2.phase());
        self.spectral
            .set_phase((((slot as f32) * 0.163) + note as f32 * 0.023).fract());
        for (index, partial) in self.additive_partials.iter_mut().enumerate() {
            partial.set_phase(deterministic_partial_unit(
                slot,
                note,
                age,
                index,
                0xA7D1_71C3,
            ));
            self.additive_detune_cents[index] =
                (deterministic_partial_unit(slot, note, age, index, 0x51E1_D123) * 2.0 - 1.0)
                    * additive_random_detune_cents.clamp(0.0, 35.0);
        }
        self.sub
            .set_phase((((slot as f32) * 0.089) + note as f32 * 0.013).fract());
        self.pwm_lfo
            .reset_to((((slot as f32) * 0.191) + note as f32 * 0.011).fract());
        self.drift_lfo
            .reset_to((((slot as f32) * 0.071) + note as f32 * 0.019).fract());
        self.noise_color_state = 0.0;
    }

    pub(crate) fn start_release(&mut self) {
        if self.phase != VoicePhase::Idle {
            self.phase = VoicePhase::Released;
            self.amp_env.note_off();
            self.filter_env.note_off();
        }
    }
}

fn deterministic_partial_unit(
    slot: usize,
    note: u8,
    age: u64,
    partial_index: usize,
    salt: u32,
) -> f32 {
    let mut state = salt
        ^ ((slot as u32).wrapping_mul(0x9E37_79B9))
        ^ ((note as u32).wrapping_mul(0x85EB_CA6B))
        ^ ((partial_index as u32).wrapping_mul(0xC2B2_AE35))
        ^ (age as u32)
        ^ ((age >> 32) as u32);
    state ^= state >> 16;
    state = state.wrapping_mul(0x7FEB_352D);
    state ^= state >> 15;
    state = state.wrapping_mul(0x846C_A68B);
    state ^= state >> 16;
    (state as f32) / (u32::MAX as f32)
}

fn apply_phase_mode(
    oscillator: &mut Oscillator,
    mode: f32,
    start_phase: f32,
    deterministic_phase: f32,
) {
    match mode.round() as i32 {
        1 => oscillator.set_phase(start_phase),
        2 => {}
        _ => oscillator.set_phase(deterministic_phase),
    }
}
