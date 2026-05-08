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
