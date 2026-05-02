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
    pub osc2_wave_mix: [f32; 3],
    pub sub_level: f32,
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
    pub(crate) sub: Oscillator,
    pub(crate) noise: NoiseRng,
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
            sub: Oscillator::new(),
            noise: NoiseRng::new(seed),
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

    pub(crate) fn trigger(
        &mut self,
        note: u8,
        velocity: f32,
        age: u64,
        amp_env: AdsrTiming,
        filter_env: AdsrTiming,
        slot: usize,
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
        self.osc1
            .set_phase((((slot as f32) * 0.137) + note as f32 * 0.017).fract());
        self.osc2
            .set_phase((((slot as f32) * 0.223) + note as f32 * 0.031).fract());
        self.sub
            .set_phase((((slot as f32) * 0.089) + note as f32 * 0.013).fract());
    }

    pub(crate) fn start_release(&mut self) {
        if self.phase != VoicePhase::Idle {
            self.phase = VoicePhase::Released;
            self.amp_env.note_off();
            self.filter_env.note_off();
        }
    }
}
