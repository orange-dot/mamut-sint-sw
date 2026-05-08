use std::f32::consts::TAU;

use crate::block::StereoBlockMut;

pub use crate::math::{mix, soft_clip};

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
