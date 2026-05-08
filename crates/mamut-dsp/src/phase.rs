#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhaseAccumulator {
    phase: f32,
    sample_rate_hz: f32,
}

impl PhaseAccumulator {
    pub fn new(sample_rate_hz: f32) -> Self {
        Self {
            phase: 0.0,
            sample_rate_hz: sanitize_sample_rate(sample_rate_hz),
        }
    }

    pub fn with_phase(sample_rate_hz: f32, phase: f32) -> Self {
        Self {
            phase: normalize_phase(phase),
            sample_rate_hz: sanitize_sample_rate(sample_rate_hz),
        }
    }

    pub fn phase(&self) -> f32 {
        self.phase
    }

    pub fn sample_rate_hz(&self) -> f32 {
        self.sample_rate_hz
    }

    pub fn set_sample_rate(&mut self, sample_rate_hz: f32) {
        let sample_rate_hz = sanitize_sample_rate(sample_rate_hz);
        if self.sample_rate_hz != sample_rate_hz {
            self.sample_rate_hz = sample_rate_hz;
        }
    }

    pub fn set_phase(&mut self, phase: f32) {
        self.phase = normalize_phase(phase);
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
    }

    pub fn reset_to(&mut self, phase: f32) {
        self.set_phase(phase);
    }

    pub fn step_for_frequency(&self, frequency_hz: f32) -> f32 {
        sanitize_step(frequency_hz / self.sample_rate_hz)
    }

    pub fn advance(&mut self, frequency_hz: f32) -> bool {
        self.advance_by(self.step_for_frequency(frequency_hz))
    }

    pub fn advance_by(&mut self, step: f32) -> bool {
        self.phase += sanitize_step(step);
        if self.phase >= 1.0 {
            self.phase -= 1.0;
            true
        } else {
            false
        }
    }

    pub fn hard_sync(&mut self, amount: f32) {
        self.phase *= (1.0 - amount.clamp(0.0, 1.0)).max(0.0);
    }
}

impl Default for PhaseAccumulator {
    fn default() -> Self {
        Self::new(1.0)
    }
}

fn sanitize_sample_rate(sample_rate_hz: f32) -> f32 {
    if sample_rate_hz.is_finite() {
        sample_rate_hz.max(1.0)
    } else {
        1.0
    }
}

fn sanitize_step(step: f32) -> f32 {
    if step.is_finite() {
        step.clamp(0.0, 0.5)
    } else {
        0.0
    }
}

fn normalize_phase(phase: f32) -> f32 {
    if phase.is_finite() {
        phase.fract().abs()
    } else {
        0.0
    }
}
