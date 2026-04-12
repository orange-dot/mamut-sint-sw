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
        sample_rate_hz: f32,
    ) -> f32 {
        let cutoff_hz = cutoff_hz.clamp(20.0, sample_rate_hz * 0.45);
        let frequency = (PI * cutoff_hz / sample_rate_hz.max(1.0)).sin() * 2.0;
        let damping = (2.0 - resonance.clamp(0.0, 1.0) * 1.8).clamp(0.1, 2.0);

        let high = input - self.low - damping * self.band;
        self.band += frequency * high;
        self.low += frequency * self.band;
        self.low
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
        let base_delay = self.sample_rate_hz * 0.018;
        let modulation = self.sample_rate_hz * 0.008 * depth.clamp(0.0, 1.0);
        let lfo = (self.lfo_phase * TAU).sin();
        let left_delay = base_delay + modulation * lfo;
        let right_delay = base_delay - modulation * lfo;
        let delayed_left = self.read_delay(&self.left_buffer, left_delay);
        let delayed_right = self.read_delay(&self.right_buffer, right_delay);

        self.left_buffer[self.write_index] = left;
        self.right_buffer[self.write_index] = right;
        self.write_index = (self.write_index + 1) % self.left_buffer.len();
        self.lfo_phase = (self.lfo_phase + rate_hz.max(0.01) / self.sample_rate_hz).fract();

        let mix = mix.clamp(0.0, 1.0);
        (
            left * (1.0 - mix) + delayed_left * mix,
            right * (1.0 - mix) + delayed_right * mix,
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
        let delayed_left = self.left_buffer[self.write_index];
        let delayed_right = self.right_buffer[self.write_index];
        let damping = damping.clamp(0.0, 0.99);
        let feedback = (0.45 + size.clamp(0.0, 1.0) * 0.45).clamp(0.0, 0.92);

        self.damped_left += (delayed_left - self.damped_left) * (1.0 - damping);
        self.damped_right += (delayed_right - self.damped_right) * (1.0 - damping);

        self.left_buffer[self.write_index] = left + self.damped_right * feedback;
        self.right_buffer[self.write_index] = right + self.damped_left * feedback;
        self.write_index = (self.write_index + 1) % self.left_buffer.len();

        let mix = mix.clamp(0.0, 1.0);
        (
            left * (1.0 - mix) + self.damped_left * mix,
            right * (1.0 - mix) + self.damped_right * mix,
        )
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

pub fn soft_clip(sample: f32, asymmetry: f32) -> f32 {
    let offset = asymmetry.clamp(-1.0, 1.0) * 0.35;
    ((sample + offset) * 1.25).tanh()
}

pub fn sanitize_sample(sample: f32) -> f32 {
    if sample.is_finite() {
        sample.clamp(-1.0, 1.0)
    } else {
        0.0
    }
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
            let sample = filter.process(0.5, 1_200.0, 0.4, 48_000.0);
            assert!(sample.is_finite());
        }
    }
}
