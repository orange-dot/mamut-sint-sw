use std::f32::consts::PI;

use crate::safety::{mix, soft_clip};
use crate::waveshaper::{asymmetric_diode, cubic_soft_clip_compensated, tanh_drive_compensated};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TptSvfOutput {
    pub low: f32,
    pub band: f32,
    pub high: f32,
    pub notch: f32,
    pub peak: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TptStateVariableFilter {
    ic1eq: f32,
    ic2eq: f32,
}

impl TptStateVariableFilter {
    pub fn new() -> Self {
        Self {
            ic1eq: 0.0,
            ic2eq: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.ic1eq = 0.0;
        self.ic2eq = 0.0;
    }

    pub fn process(
        &mut self,
        input: f32,
        cutoff_hz: f32,
        resonance: f32,
        sample_rate_hz: f32,
    ) -> TptSvfOutput {
        let sample_rate_hz = finite_or(sample_rate_hz, 48_000.0).max(1.0);
        let cutoff_hz = finite_or(cutoff_hz, 20.0).clamp(20.0, max_cutoff(sample_rate_hz, 0.45));
        let resonance = finite_or(resonance, 0.0).clamp(0.0, 1.0);
        let g = (PI * cutoff_hz / sample_rate_hz)
            .tan()
            .clamp(0.000_01, 32.0);
        let k = (2.0 - resonance * 1.86).clamp(0.08, 2.0);
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;
        let input = finite_or(input, 0.0);

        let v3 = input - self.ic2eq;
        let v1 = a1 * self.ic1eq + a2 * v3;
        let v2 = self.ic2eq + a2 * self.ic1eq + a3 * v3;
        self.ic1eq = sanitize_state(2.0 * v1 - self.ic1eq);
        self.ic2eq = sanitize_state(2.0 * v2 - self.ic2eq);

        let low = sanitize_state(v2);
        let band = sanitize_state(v1);
        let high = sanitize_state(input - k * v1 - v2);
        TptSvfOutput {
            low,
            band,
            high,
            notch: sanitize_state(low + high),
            peak: sanitize_state(low - high),
        }
    }
}

impl Default for TptStateVariableFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResonantMatterFilter {
    core: TptStateVariableFilter,
    body_state: f32,
    pressure_state: f32,
}

impl ResonantMatterFilter {
    pub fn new() -> Self {
        Self {
            core: TptStateVariableFilter::new(),
            body_state: 0.0,
            pressure_state: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.core.reset();
        self.body_state = 0.0;
        self.pressure_state = 0.0;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn process(
        &mut self,
        input: f32,
        cutoff_hz: f32,
        resonance: f32,
        drive: f32,
        strain: f32,
        matter: f32,
        sample_rate_hz: f32,
    ) -> f32 {
        let drive = finite_or(drive, 0.0).clamp(0.0, 1.0);
        let strain = finite_or(strain, 0.0).clamp(0.0, 1.0);
        let matter = finite_or(matter, 0.0).clamp(0.0, 1.0);
        let resonance = finite_or(resonance, 0.0).clamp(0.0, 1.0);
        let input_gain = 1.0 + drive * 2.2 + strain * 0.45 + matter * 0.35;
        let shaped_input = asymmetric_diode(
            input * input_gain,
            drive * 0.70 + matter * 0.25,
            strain * 0.9,
        );
        let output = self
            .core
            .process(shaped_input, cutoff_hz, resonance, sample_rate_hz);

        let body_rate = 0.012 + matter * 0.030 + strain * 0.010;
        let pressure_rate = 0.035 + resonance * 0.025 + matter * 0.020;
        self.body_state += (output.low - self.body_state) * body_rate;
        self.pressure_state += (output.band - self.pressure_state) * pressure_rate;
        self.body_state = sanitize_state(self.body_state);
        self.pressure_state = sanitize_state(self.pressure_state);

        let mass = 0.88 + matter * 0.24;
        let pressure = matter * (0.20 + resonance * 0.28 + strain * 0.16);
        let air = matter * (0.04 + drive * 0.08);
        let body_feedback = self.body_state * (0.18 + matter * 0.18);
        let pressure_feedback = self.pressure_state * (0.05 + strain * 0.14);
        let mixed = output.low * mass
            + output.band * pressure
            + output.notch * air
            + body_feedback
            + pressure_feedback;
        let clipped = cubic_soft_clip_compensated(mixed * (1.0 + drive * 0.25));
        tanh_drive_compensated(clipped, drive * 0.35 + strain * 0.20 + matter * 0.12)
    }
}

impl Default for ResonantMatterFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StateVariableFilter {
    low: f32,
    band: f32,
    tpt: TptStateVariableFilter,
    matter: ResonantMatterFilter,
}

impl StateVariableFilter {
    pub fn new() -> Self {
        Self {
            low: 0.0,
            band: 0.0,
            tpt: TptStateVariableFilter::new(),
            matter: ResonantMatterFilter::new(),
        }
    }

    pub fn reset(&mut self) {
        self.low = 0.0;
        self.band = 0.0;
        self.tpt.reset();
        self.matter.reset();
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
        let sample_rate_hz = finite_or(sample_rate_hz, 48_000.0).max(1.0);
        let cutoff_hz = finite_or(cutoff_hz, 20.0).clamp(20.0, max_cutoff(sample_rate_hz, 0.42));
        let resonance = finite_or(resonance, 0.0).clamp(0.0, 1.0);
        let drive = finite_or(drive, 0.0).clamp(0.0, 1.0);
        let strain = finite_or(strain, 0.0).clamp(0.0, 1.0);
        let input = finite_or(input, 0.0);
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

    #[allow(clippy::too_many_arguments)]
    pub fn process_model(
        &mut self,
        input: f32,
        cutoff_hz: f32,
        resonance: f32,
        drive: f32,
        strain: f32,
        matter: f32,
        filter_model: f32,
        sample_rate_hz: f32,
    ) -> f32 {
        match filter_model.round() as i32 {
            1 => {
                self.tpt
                    .process(input, cutoff_hz, resonance, sample_rate_hz)
                    .low
            }
            2 => self.matter.process(
                input,
                cutoff_hz,
                resonance,
                drive,
                strain,
                matter,
                sample_rate_hz,
            ),
            _ => self.process(input, cutoff_hz, resonance, drive, strain, sample_rate_hz),
        }
    }
}

impl Default for StateVariableFilter {
    fn default() -> Self {
        Self::new()
    }
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

fn sanitize_state(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn max_cutoff(sample_rate_hz: f32, nyquist_fraction: f32) -> f32 {
    (sample_rate_hz * nyquist_fraction).max(20.0)
}
