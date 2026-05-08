use std::f32::consts::PI;

use crate::safety::{mix, soft_clip};

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
