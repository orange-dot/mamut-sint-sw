#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BandlimitedTriangle {
    value: f32,
    initialized: bool,
}

impl BandlimitedTriangle {
    pub fn new() -> Self {
        Self {
            value: 0.0,
            initialized: false,
        }
    }

    pub fn reset_to_phase(&mut self, phase: f32) {
        self.value = raw_triangle(phase);
        self.initialized = true;
    }

    pub fn next_sample(&mut self, phase: f32, phase_step: f32) -> f32 {
        if !self.initialized {
            self.reset_to_phase(phase);
        }
        let sample = self.value.clamp(-1.0, 1.0);
        let square = poly_blep_pulse_sample(phase, 0.5, phase_step);
        self.value =
            (self.value + square * sanitize_phase_step(phase_step) * 4.0).clamp(-1.25, 1.25);
        sample
    }
}

impl Default for BandlimitedTriangle {
    fn default() -> Self {
        Self::new()
    }
}

pub fn poly_blep(phase: f32, phase_step: f32) -> f32 {
    let phase_step = sanitize_phase_step(phase_step);
    if phase_step <= f32::EPSILON {
        return 0.0;
    }

    let phase = normalize_phase(phase);
    if phase < phase_step {
        let t = phase / phase_step;
        t + t - t * t - 1.0
    } else if phase > 1.0 - phase_step {
        let t = (phase - 1.0) / phase_step;
        t * t + t + t + 1.0
    } else {
        0.0
    }
}

pub fn poly_blep_saw_sample(phase: f32, phase_step: f32) -> f32 {
    let phase = normalize_phase(phase);
    (phase * 2.0 - 1.0 - poly_blep(phase, phase_step)).clamp(-1.25, 1.25)
}

pub fn poly_blep_pulse_sample(phase: f32, width: f32, phase_step: f32) -> f32 {
    let phase = normalize_phase(phase);
    let width = width.clamp(0.05, 0.95);
    let mut sample = if phase < width { 1.0 } else { -1.0 };
    sample += poly_blep(phase, phase_step);
    sample -= poly_blep(normalize_phase(phase - width), phase_step);
    sample.clamp(-1.25, 1.25)
}

fn raw_triangle(phase: f32) -> f32 {
    1.0 - 4.0 * (normalize_phase(phase) - 0.5).abs()
}

fn sanitize_phase_step(phase_step: f32) -> f32 {
    if phase_step.is_finite() {
        phase_step.clamp(0.0, 0.5)
    } else {
        0.0
    }
}

fn normalize_phase(phase: f32) -> f32 {
    if phase.is_finite() {
        phase.rem_euclid(1.0)
    } else {
        0.0
    }
}
