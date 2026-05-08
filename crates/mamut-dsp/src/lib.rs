use std::f32::consts::{PI, TAU};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdsrTiming {
    pub attack_ms: f32,
    pub decay_ms: f32,
    pub sustain: f32,
    pub release_ms: f32,
}

impl AdsrTiming {
    pub fn clamp(self) -> Self {
        Self {
            attack_ms: self.attack_ms.max(1.0),
            decay_ms: self.decay_ms.max(1.0),
            sustain: self.sustain.clamp(0.0, 1.0),
            release_ms: self.release_ms.max(1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdsrEnvelope {
    timing: AdsrTiming,
    sample_rate_hz: f32,
    level: f32,
    stage: EnvelopeStage,
}

impl AdsrEnvelope {
    pub fn new(sample_rate_hz: f32, timing: AdsrTiming) -> Self {
        Self {
            timing: timing.clamp(),
            sample_rate_hz: sample_rate_hz.max(1.0),
            level: 0.0,
            stage: EnvelopeStage::Idle,
        }
    }

    pub fn set_timing(&mut self, timing: AdsrTiming) {
        self.timing = timing.clamp();
    }

    pub fn note_on(&mut self) {
        self.stage = EnvelopeStage::Attack;
    }

    pub fn note_off(&mut self) {
        if self.stage != EnvelopeStage::Idle {
            self.stage = EnvelopeStage::Release;
        }
    }

    pub fn is_idle(&self) -> bool {
        self.stage == EnvelopeStage::Idle
    }

    pub fn current_level(&self) -> f32 {
        self.level
    }

    pub fn stage(&self) -> EnvelopeStage {
        self.stage
    }

    pub fn next_sample(&mut self) -> f32 {
        match self.stage {
            EnvelopeStage::Idle => {
                self.level = 0.0;
            }
            EnvelopeStage::Attack => {
                let attack_samples = (self.timing.attack_ms * 0.001 * self.sample_rate_hz).max(1.0);
                self.level += 1.0 / attack_samples;
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = EnvelopeStage::Decay;
                }
            }
            EnvelopeStage::Decay => {
                let decay_samples = (self.timing.decay_ms * 0.001 * self.sample_rate_hz).max(1.0);
                self.level += (self.timing.sustain - self.level) / decay_samples;
                if (self.level - self.timing.sustain).abs() < 0.001 {
                    self.level = self.timing.sustain;
                    self.stage = EnvelopeStage::Sustain;
                }
            }
            EnvelopeStage::Sustain => {
                self.level = self.timing.sustain;
            }
            EnvelopeStage::Release => {
                let release_samples =
                    (self.timing.release_ms * 0.001 * self.sample_rate_hz).max(1.0);
                self.level += (0.0 - self.level) / release_samples;
                if self.level.abs() < 0.0005 {
                    self.level = 0.0;
                    self.stage = EnvelopeStage::Idle;
                }
            }
        }

        self.level.clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearSmoother {
    current: f32,
    target: f32,
    step: f32,
    samples_remaining: usize,
}

impl LinearSmoother {
    pub fn new(initial: f32) -> Self {
        Self {
            current: initial,
            target: initial,
            step: 0.0,
            samples_remaining: 0,
        }
    }

    pub fn current(&self) -> f32 {
        self.current
    }

    pub fn set_target(&mut self, target: f32, sample_count: usize) {
        self.target = target;
        self.samples_remaining = sample_count;
        self.step = if sample_count == 0 {
            self.current = target;
            0.0
        } else {
            (target - self.current) / sample_count as f32
        };
    }

    pub fn next_value(&mut self) -> f32 {
        if self.samples_remaining > 0 {
            self.current += self.step;
            self.samples_remaining -= 1;
            if self.samples_remaining == 0 {
                self.current = self.target;
            }
        }

        self.current
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Oscillator {
    phase: f32,
}

impl Oscillator {
    pub fn new() -> Self {
        Self { phase: 0.0 }
    }

    pub fn set_phase(&mut self, phase: f32) {
        self.phase = phase.fract().abs();
    }

    pub fn phase(&self) -> f32 {
        self.phase
    }

    pub fn hard_sync(&mut self, amount: f32) {
        self.phase *= (1.0 - amount.clamp(0.0, 1.0)).max(0.0);
    }

    pub fn saw_sample(&self) -> f32 {
        self.phase * 2.0 - 1.0
    }

    pub fn pulse_sample(&self, width: f32) -> f32 {
        if self.phase < width.clamp(0.05, 0.95) {
            1.0
        } else {
            -1.0
        }
    }

    pub fn triangle_sample(&self) -> f32 {
        1.0 - 4.0 * (self.phase - 0.5).abs()
    }

    pub fn square_sample(&self) -> f32 {
        self.pulse_sample(0.5)
    }

    pub fn advance(&mut self, frequency_hz: f32, sample_rate_hz: f32) -> bool {
        let increment = (frequency_hz / sample_rate_hz.max(1.0)).clamp(0.0, 0.5);
        self.phase += increment;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
            true
        } else {
            false
        }
    }
}

impl Default for Oscillator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoiseRng {
    state: u32,
}

impl NoiseRng {
    pub fn new(seed: u32) -> Self {
        Self { state: seed.max(1) }
    }

    pub fn next_bipolar(&mut self) -> f32 {
        self.state = self
            .state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        let normalized = (self.state as f32 / u32::MAX as f32).clamp(0.0, 1.0);
        normalized * 2.0 - 1.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StateVariableFilter {
    low: f32,
    band: f32,
}

impl StateVariableFilter {
    pub fn new() -> Self {
        Self {
            low: 0.0,
            band: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.low = 0.0;
        self.band = 0.0;
    }

    pub fn process(
        &mut self,
        input: f32,
        cutoff_hz: f32,
        resonance: f32,
        drive: f32,
        strain: f32,
        sample_rate_hz: f32,
    ) -> f32 {
        let cutoff_hz = cutoff_hz.clamp(20.0, sample_rate_hz * 0.42);
        let resonance = resonance.clamp(0.0, 1.0);
        let drive = drive.clamp(0.0, 1.0);
        let strain = strain.clamp(0.0, 1.0);
        let frequency = ((PI * cutoff_hz / sample_rate_hz.max(1.0)).sin() * 1.92).clamp(0.0, 1.8);
        let damping = (1.95 - resonance * 1.34 - strain * 0.24).clamp(0.12, 1.95);
        let input_gain = 1.0 + drive * 1.9 + strain * 0.35;
        let mut stage_input = soft_clip(input * input_gain, strain * 0.20);
        let blend = (0.42 + drive * 0.24 + strain * 0.12).clamp(0.0, 1.0);

        for _ in 0..2 {
            let high = stage_input - self.low - damping * self.band;
            self.band += frequency * high * 0.5;
            self.band = soft_clip(self.band * (1.0 + drive * 0.10), strain * 0.05);
            self.low += frequency * self.band * 0.5;
            self.low = mix(
                self.low,
                soft_clip(self.low * (1.0 + drive * 0.18), strain * 0.08),
                blend,
            );
            stage_input = self.low;
        }

        let output = mix(self.low, self.low + self.band * 0.10, strain * 0.32);
        soft_clip(output, strain * 0.14 + resonance * 0.04)
    }
}

impl Default for StateVariableFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct SimpleChorus {
    left_buffer: Vec<f32>,
    right_buffer: Vec<f32>,
    write_index: usize,
    lfo_phase: f32,
    sample_rate_hz: f32,
}

impl SimpleChorus {
    pub fn new(sample_rate_hz: f32) -> Self {
        let buffer_len = ((sample_rate_hz * 0.08).round() as usize).max(4);
        Self {
            left_buffer: vec![0.0; buffer_len],
            right_buffer: vec![0.0; buffer_len],
            write_index: 0,
            lfo_phase: 0.0,
            sample_rate_hz,
        }
    }

    pub fn process(
        &mut self,
        left: f32,
        right: f32,
        mix: f32,
        depth: f32,
        rate_hz: f32,
    ) -> (f32, f32) {
        let mix = mix.clamp(0.0, 1.0);
        let depth = depth.clamp(0.0, 1.0);
        let base_delay = self.sample_rate_hz * 0.014;
        let modulation = self.sample_rate_hz * 0.006 * depth;
        let lfo_a = (self.lfo_phase * TAU).sin();
        let lfo_b = ((self.lfo_phase + 0.31).fract() * TAU).sin();
        let lfo_c = ((self.lfo_phase + 0.63).fract() * TAU).sin();
        let left_delay = base_delay + modulation * lfo_a;
        let right_delay = base_delay + modulation * lfo_b;
        let cross_delay = base_delay * 0.74 + modulation * 0.45 * lfo_c;
        let delayed_left = self.read_delay(&self.left_buffer, left_delay);
        let delayed_right = self.read_delay(&self.right_buffer, right_delay);
        let cross_left = self.read_delay(&self.right_buffer, cross_delay * 0.94);
        let cross_right = self.read_delay(&self.left_buffer, cross_delay * 1.06);
        let center = (left + right) * 0.5;
        let feedback = 0.08 + depth * 0.12;

        self.left_buffer[self.write_index] = sanitize_sample(left + delayed_left * feedback);
        self.right_buffer[self.write_index] = sanitize_sample(right + delayed_right * feedback);
        self.write_index = (self.write_index + 1) % self.left_buffer.len();
        self.lfo_phase = (self.lfo_phase + rate_hz.max(0.01) / self.sample_rate_hz).fract();

        let wet_left = delayed_left * 0.66 + cross_left * 0.22 + center * 0.12;
        let wet_right = delayed_right * 0.66 + cross_right * 0.22 + center * 0.12;
        let dry_gain = 1.0 - mix * 0.72;
        let wet_gain = mix * 0.78;
        (
            sanitize_sample(left * dry_gain + wet_left * wet_gain),
            sanitize_sample(right * dry_gain + wet_right * wet_gain),
        )
    }

    fn read_delay(&self, buffer: &[f32], delay_samples: f32) -> f32 {
        let len = buffer.len();
        let delay_samples = delay_samples.clamp(1.0, (len - 1) as f32);
        let read_pos = (self.write_index + len) as f32 - delay_samples;
        let read_floor = read_pos.floor();
        let frac = read_pos - read_floor;
        let index_a = (read_floor as usize) % len;
        let index_b = (index_a + 1) % len;
        buffer[index_a] * (1.0 - frac) + buffer[index_b] * frac
    }
}

#[derive(Debug, Clone)]
pub struct SimpleReverb {
    left_buffer: Vec<f32>,
    right_buffer: Vec<f32>,
    write_index: usize,
    damped_left: f32,
    damped_right: f32,
}

impl SimpleReverb {
    pub fn new(sample_rate_hz: f32) -> Self {
        let buffer_len = ((sample_rate_hz * 0.34).round() as usize).max(8);
        Self {
            left_buffer: vec![0.0; buffer_len],
            right_buffer: vec![0.0; buffer_len],
            write_index: 0,
            damped_left: 0.0,
            damped_right: 0.0,
        }
    }

    pub fn process(
        &mut self,
        left: f32,
        right: f32,
        mix: f32,
        size: f32,
        damping: f32,
    ) -> (f32, f32) {
        let mix = mix.clamp(0.0, 1.0);
        let size = size.clamp(0.0, 1.0);
        let damping = damping.clamp(0.0, 0.99);
        let delayed_left = self.left_buffer[self.write_index];
        let delayed_right = self.right_buffer[self.write_index];
        let len = self.left_buffer.len();
        let tap_a_offset = ((len as f32) * (0.16 + size * 0.28)).round() as usize;
        let tap_b_offset = ((len as f32) * (0.31 + size * 0.22)).round() as usize;
        let tap_left = self.read_tap(&self.left_buffer, tap_a_offset.max(1));
        let tap_right = self.read_tap(&self.right_buffer, tap_a_offset.max(1));
        let cross_left = self.read_tap(&self.right_buffer, tap_b_offset.max(1));
        let cross_right = self.read_tap(&self.left_buffer, tap_b_offset.max(1));
        let feedback = (0.34 + size * 0.40).clamp(0.0, 0.88);
        let diffusion = 0.16 + size * 0.22;
        let damping_blend = 1.0 - damping * 0.88;

        self.damped_left +=
            ((delayed_left + cross_left * diffusion) - self.damped_left) * damping_blend;
        self.damped_right +=
            ((delayed_right + cross_right * diffusion) - self.damped_right) * damping_blend;

        self.left_buffer[self.write_index] =
            sanitize_sample(left * 0.42 + self.damped_right * feedback);
        self.right_buffer[self.write_index] =
            sanitize_sample(right * 0.42 + self.damped_left * feedback);
        self.write_index = (self.write_index + 1) % self.left_buffer.len();

        let wet_left = delayed_left * 0.48 + tap_left * 0.18 + cross_left * 0.26;
        let wet_right = delayed_right * 0.48 + tap_right * 0.18 + cross_right * 0.26;
        let dry_gain = 1.0 - mix * 0.58;
        let wet_gain = mix * (0.62 + size * 0.10);
        (
            sanitize_sample(left * dry_gain + wet_left * wet_gain),
            sanitize_sample(right * dry_gain + wet_right * wet_gain),
        )
    }

    fn read_tap(&self, buffer: &[f32], offset: usize) -> f32 {
        let len = buffer.len();
        buffer[(self.write_index + len - (offset % len.max(1))) % len]
    }
}

#[derive(Debug)]
pub struct StereoBlockMut<'a> {
    pub left: &'a mut [f32],
    pub right: &'a mut [f32],
}

impl<'a> StereoBlockMut<'a> {
    pub fn new(left: &'a mut [f32], right: &'a mut [f32]) -> Self {
        Self { left, right }
    }

    pub fn frames(&self) -> usize {
        self.left.len().min(self.right.len())
    }

    pub fn clear(&mut self) {
        let frames = self.frames();
        self.left[..frames].fill(0.0);
        self.right[..frames].fill(0.0);
    }

    pub fn peak_abs(&self) -> f32 {
        self.left
            .iter()
            .zip(self.right.iter())
            .map(|(left, right)| left.abs().max(right.abs()))
            .fold(0.0, f32::max)
    }
}

pub fn midi_note_hz(note: f32) -> f32 {
    440.0 * 2.0_f32.powf((note - 69.0) / 12.0)
}

pub fn db_to_gain(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

pub fn mix(a: f32, b: f32, amount: f32) -> f32 {
    a + (b - a) * amount.clamp(0.0, 1.0)
}

pub const DENORMAL_FLUSH_ABS: f32 = 1.0e-20;
pub const MASTER_SAFETY_KNEE: f32 = 0.92;
pub const MASTER_SAFETY_CEILING: f32 = 0.96;

pub fn flush_tiny_sample(sample: f32) -> f32 {
    if sample.is_finite() && sample.abs() >= DENORMAL_FLUSH_ABS {
        sample
    } else {
        0.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DcBlocker {
    coefficient: f32,
    previous_input: f32,
    previous_output: f32,
}

impl DcBlocker {
    pub fn new(sample_rate_hz: f32, cutoff_hz: f32) -> Self {
        let sample_rate_hz = sample_rate_hz.max(1.0);
        let cutoff_hz = cutoff_hz.max(0.001);
        let coefficient = (-TAU * cutoff_hz / sample_rate_hz)
            .exp()
            .clamp(0.0, 0.999_999);
        Self {
            coefficient,
            previous_input: 0.0,
            previous_output: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.previous_input = 0.0;
        self.previous_output = 0.0;
    }

    pub fn process(&mut self, sample: f32) -> f32 {
        let input = flush_tiny_sample(sample);
        let output = input - self.previous_input + self.coefficient * self.previous_output;
        self.previous_input = input;
        self.previous_output = flush_tiny_sample(output);
        self.previous_output
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StereoDcBlocker {
    left: DcBlocker,
    right: DcBlocker,
}

impl StereoDcBlocker {
    pub fn new(sample_rate_hz: f32, cutoff_hz: f32) -> Self {
        Self {
            left: DcBlocker::new(sample_rate_hz, cutoff_hz),
            right: DcBlocker::new(sample_rate_hz, cutoff_hz),
        }
    }

    pub fn reset(&mut self) {
        self.left.reset();
        self.right.reset();
    }

    pub fn process(&mut self, left: f32, right: f32) -> (f32, f32) {
        (self.left.process(left), self.right.process(right))
    }
}

pub fn soft_clip(sample: f32, asymmetry: f32) -> f32 {
    let offset = asymmetry.clamp(-1.0, 1.0) * 0.35;
    ((sample + offset) * 1.25).tanh()
}

pub fn master_safety_limit(sample: f32) -> f32 {
    let sample = flush_tiny_sample(sample);
    let abs_sample = sample.abs();
    if abs_sample <= MASTER_SAFETY_KNEE {
        return sample;
    }

    let over = abs_sample - MASTER_SAFETY_KNEE;
    let ceiling_span = MASTER_SAFETY_CEILING - MASTER_SAFETY_KNEE;
    let limited_abs = MASTER_SAFETY_KNEE + ceiling_span * over / (over + ceiling_span);
    let limited = limited_abs.min(MASTER_SAFETY_CEILING).copysign(sample);
    flush_tiny_sample(limited)
}

pub fn sanitize_sample(sample: f32) -> f32 {
    flush_tiny_sample(sample).clamp(-1.0, 1.0)
}

pub fn sanitize_block(block: &mut StereoBlockMut<'_>) {
    let frames = block.frames();
    for index in 0..frames {
        block.left[index] = sanitize_sample(block.left[index]);
        block.right[index] = sanitize_sample(block.right[index]);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn linear_smoother_reaches_target() {
        let mut smoother = LinearSmoother::new(0.0);
        smoother.set_target(1.0, 4);

        for _ in 0..4 {
            smoother.next_value();
        }

        assert_eq!(smoother.current(), 1.0);
    }

    #[test]
    fn sanitize_non_finite_samples() {
        assert_eq!(sanitize_sample(f32::NAN), 0.0);
        assert_eq!(sanitize_sample(f32::INFINITY), 0.0);
    }

    #[test]
    fn sanitize_flushes_tiny_samples() {
        assert_eq!(sanitize_sample(DENORMAL_FLUSH_ABS * 0.5), 0.0);
        assert_eq!(sanitize_sample(-DENORMAL_FLUSH_ABS * 0.5), 0.0);
        assert_eq!(
            sanitize_sample(DENORMAL_FLUSH_ABS * 2.0),
            DENORMAL_FLUSH_ABS * 2.0
        );
    }

    #[test]
    fn envelope_reaches_release_idle() {
        let mut env = AdsrEnvelope::new(
            48_000.0,
            AdsrTiming {
                attack_ms: 1.0,
                decay_ms: 1.0,
                sustain: 0.5,
                release_ms: 1.0,
            },
        );
        env.note_on();
        for _ in 0..512 {
            env.next_sample();
        }
        env.note_off();
        for _ in 0..1024 {
            env.next_sample();
        }
        assert!(env.is_idle());
    }

    #[test]
    fn filter_output_stays_finite() {
        let mut filter = StateVariableFilter::new();
        for _ in 0..1024 {
            let sample = filter.process(0.5, 1_200.0, 0.4, 0.3, 0.2, 48_000.0);
            assert!(sample.is_finite());
        }
    }

    #[test]
    fn filter_survives_extreme_drive_and_strain() {
        let mut filter = StateVariableFilter::new();
        for _ in 0..4096 {
            let sample = filter.process(0.85, 8_400.0, 0.95, 0.95, 0.90, 48_000.0);
            assert!(sample.is_finite());
        }
    }

    #[test]
    fn dc_blocker_removes_constant_bias() {
        let mut blocker = DcBlocker::new(48_000.0, 5.0);
        let mut output = 0.0;

        for _ in 0..48_000 {
            output = blocker.process(0.2);
        }

        assert!(output.abs() < 0.001, "output={output}");
    }

    #[test]
    fn dc_blocker_preserves_audio_band_sine_level() {
        let mut blocker = DcBlocker::new(48_000.0, 5.0);
        let mut input_sum = 0.0;
        let mut output_sum = 0.0;
        let mut samples = 0_usize;

        for frame in 0..96_000 {
            let sample = (TAU * 100.0 * frame as f32 / 48_000.0).sin() * 0.4;
            let output = blocker.process(sample);
            assert!(output.is_finite());
            if frame >= 4_800 {
                input_sum += sample * sample;
                output_sum += output * output;
                samples += 1;
            }
        }

        let input_rms = (input_sum / samples as f32).sqrt();
        let output_rms = (output_sum / samples as f32).sqrt();
        let ratio = output_rms / input_rms;
        assert!((0.98..=1.02).contains(&ratio), "ratio={ratio}");
    }

    #[test]
    fn dc_blocker_reset_clears_history() {
        let mut blocker = StereoDcBlocker::new(48_000.0, 5.0);
        for _ in 0..4_800 {
            blocker.process(0.2, -0.2);
        }

        blocker.reset();

        assert_eq!(blocker.process(0.0, 0.0), (0.0, 0.0));
    }

    #[test]
    fn dc_blocker_flushes_tiny_silence_tail() {
        let mut blocker = DcBlocker::new(48_000.0, 5.0);
        let output = blocker.process(DENORMAL_FLUSH_ABS * 0.5);

        assert_eq!(output, 0.0);
        assert_eq!(blocker.process(0.0), 0.0);
    }

    #[test]
    fn master_safety_limit_is_neutral_below_knee() {
        for sample in [-0.92, -0.5, 0.0, 0.5, 0.92] {
            assert_eq!(master_safety_limit(sample), sample);
        }
    }

    #[test]
    fn master_safety_limit_is_finite_and_bounded() {
        for sample in [
            f32::NEG_INFINITY,
            -1000.0,
            -1.0,
            -0.95,
            0.95,
            1.0,
            1000.0,
            f32::INFINITY,
            f32::NAN,
        ] {
            let limited = master_safety_limit(sample);
            assert!(limited.is_finite());
            assert!(
                limited.abs() <= MASTER_SAFETY_CEILING,
                "sample={sample} limited={limited}"
            );
        }
    }

    #[test]
    fn chorus_output_stays_finite_over_long_run() {
        let mut chorus = SimpleChorus::new(48_000.0);
        for step in 0..12_000 {
            let phase = step as f32 * 0.01;
            let (left, right) =
                chorus.process(phase.sin() * 0.7, phase.cos() * 0.6, 0.42, 0.64, 0.28);
            assert!(left.is_finite());
            assert!(right.is_finite());
        }
    }

    #[test]
    fn reverb_output_stays_finite_over_long_run() {
        let mut reverb = SimpleReverb::new(48_000.0);
        for step in 0..18_000 {
            let impulse = if step % 512 == 0 { 0.9 } else { 0.0 };
            let (left, right) = reverb.process(impulse, impulse * 0.7, 0.36, 0.58, 0.44);
            assert!(left.is_finite());
            assert!(right.is_finite());
        }
    }
}
