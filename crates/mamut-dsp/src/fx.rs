use std::f32::consts::TAU;

use crate::safety::sanitize_sample;

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
