use std::f32::consts::TAU;

use crate::{noise::NoiseRng, phase::PhaseAccumulator};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LfoShape {
    Sine,
    Triangle,
    RampUp,
    RampDown,
    Square,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lfo {
    phase: PhaseAccumulator,
    shape: LfoShape,
}

impl Lfo {
    pub fn new(sample_rate_hz: f32, shape: LfoShape) -> Self {
        Self {
            phase: PhaseAccumulator::new(sample_rate_hz),
            shape,
        }
    }

    pub fn with_phase(sample_rate_hz: f32, shape: LfoShape, phase: f32) -> Self {
        Self {
            phase: PhaseAccumulator::with_phase(sample_rate_hz, phase),
            shape,
        }
    }

    pub fn phase(&self) -> f32 {
        self.phase.phase()
    }

    pub fn shape(&self) -> LfoShape {
        self.shape
    }

    pub fn sample_rate_hz(&self) -> f32 {
        self.phase.sample_rate_hz()
    }

    pub fn set_sample_rate(&mut self, sample_rate_hz: f32) {
        self.phase.set_sample_rate(sample_rate_hz);
    }

    pub fn set_shape(&mut self, shape: LfoShape) {
        self.shape = shape;
    }

    pub fn reset(&mut self) {
        self.phase.reset();
    }

    pub fn reset_to(&mut self, phase: f32) {
        self.phase.reset_to(phase);
    }

    pub fn sample_bipolar(&self) -> f32 {
        lfo_shape_sample(self.shape, self.phase())
    }

    pub fn sample_bipolar_offset(&self, phase_offset: f32) -> f32 {
        lfo_shape_sample(self.shape, self.phase() + phase_offset)
    }

    pub fn sample_unipolar(&self) -> f32 {
        bipolar_to_unipolar(self.sample_bipolar())
    }

    pub fn sample_unipolar_offset(&self, phase_offset: f32) -> f32 {
        bipolar_to_unipolar(self.sample_bipolar_offset(phase_offset))
    }

    pub fn advance(&mut self, rate_hz: f32) -> bool {
        self.phase.advance(rate_hz)
    }

    pub fn next_bipolar(&mut self, rate_hz: f32) -> f32 {
        let sample = self.sample_bipolar();
        self.advance(rate_hz);
        sample
    }

    pub fn next_unipolar(&mut self, rate_hz: f32) -> f32 {
        bipolar_to_unipolar(self.next_bipolar(rate_hz))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SampleHold {
    phase: PhaseAccumulator,
    rng: NoiseRng,
    current: f32,
}

impl SampleHold {
    pub fn new(sample_rate_hz: f32, seed: u32) -> Self {
        let mut rng = NoiseRng::new(seed);
        let current = rng.next_bipolar();
        Self {
            phase: PhaseAccumulator::new(sample_rate_hz),
            rng,
            current,
        }
    }

    pub fn with_value(sample_rate_hz: f32, seed: u32, value: f32) -> Self {
        Self {
            phase: PhaseAccumulator::new(sample_rate_hz),
            rng: NoiseRng::new(seed),
            current: sanitize_bipolar(value),
        }
    }

    pub fn current(&self) -> f32 {
        self.current
    }

    pub fn set_sample_rate(&mut self, sample_rate_hz: f32) {
        self.phase.set_sample_rate(sample_rate_hz);
    }

    pub fn reset(&mut self, value: f32) {
        self.phase.reset();
        self.current = sanitize_bipolar(value);
    }

    pub fn trigger(&mut self) -> f32 {
        self.current = self.rng.next_bipolar();
        self.current
    }

    pub fn next_bipolar(&mut self, rate_hz: f32) -> f32 {
        let value = self.current;
        if self.phase.advance(rate_hz) {
            self.trigger();
        }
        value
    }

    pub fn next_unipolar(&mut self, rate_hz: f32) -> f32 {
        bipolar_to_unipolar(self.next_bipolar(rate_hz))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlewLimiter {
    current: f32,
    sample_rate_hz: f32,
}

impl SlewLimiter {
    pub fn new(sample_rate_hz: f32, initial: f32) -> Self {
        Self {
            current: sanitize_value(initial),
            sample_rate_hz: sanitize_sample_rate(sample_rate_hz),
        }
    }

    pub fn current(&self) -> f32 {
        self.current
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

    pub fn reset(&mut self, value: f32) {
        self.current = sanitize_value(value);
    }

    pub fn process(&mut self, target: f32, rise_per_second: f32, fall_per_second: f32) -> f32 {
        if !target.is_finite() {
            return self.current;
        }

        let delta = target - self.current;
        let rate = if delta >= 0.0 {
            sanitize_rate(rise_per_second)
        } else {
            sanitize_rate(fall_per_second)
        };
        let max_step = rate / self.sample_rate_hz;
        if max_step <= 0.0 {
            return self.current;
        }

        if delta.abs() <= max_step {
            self.current = target;
        } else {
            self.current += delta.signum() * max_step;
        }
        self.current
    }
}

pub fn tempo_rate_hz(bpm: f32) -> f32 {
    if bpm.is_finite() && bpm > 0.0 {
        bpm / 60.0
    } else {
        0.0
    }
}

pub fn tempo_division_rate_hz(bpm: f32, beats_per_cycle: f32) -> f32 {
    if beats_per_cycle.is_finite() && beats_per_cycle > 0.0 {
        tempo_rate_hz(bpm) / beats_per_cycle
    } else {
        0.0
    }
}

pub fn beats_to_seconds(beats: f32, bpm: f32) -> f32 {
    let beat_rate_hz = tempo_rate_hz(bpm);
    if beats.is_finite() && beats > 0.0 && beat_rate_hz > 0.0 {
        beats / beat_rate_hz
    } else {
        0.0
    }
}

pub fn hz_to_samples(rate_hz: f32, sample_rate_hz: f32) -> usize {
    if !rate_hz.is_finite() || rate_hz <= 0.0 {
        return 0;
    }

    let samples = sanitize_sample_rate(sample_rate_hz) / rate_hz;
    if samples.is_finite() {
        samples.round().clamp(1.0, usize::MAX as f32) as usize
    } else {
        usize::MAX
    }
}

fn lfo_shape_sample(shape: LfoShape, phase: f32) -> f32 {
    let phase = wrapped_phase(phase);
    match shape {
        LfoShape::Sine => (phase * TAU).sin(),
        LfoShape::Triangle => 1.0 - 4.0 * (phase - 0.5).abs(),
        LfoShape::RampUp => phase * 2.0 - 1.0,
        LfoShape::RampDown => 1.0 - phase * 2.0,
        LfoShape::Square => {
            if phase < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
    }
}

fn bipolar_to_unipolar(value: f32) -> f32 {
    (value * 0.5 + 0.5).clamp(0.0, 1.0)
}

fn wrapped_phase(phase: f32) -> f32 {
    if phase.is_finite() {
        phase.rem_euclid(1.0)
    } else {
        0.0
    }
}

fn sanitize_bipolar(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

fn sanitize_value(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn sanitize_rate(rate_per_second: f32) -> f32 {
    if rate_per_second.is_finite() {
        rate_per_second.max(0.0)
    } else {
        0.0
    }
}

fn sanitize_sample_rate(sample_rate_hz: f32) -> f32 {
    if sample_rate_hz.is_finite() {
        sample_rate_hz.max(1.0)
    } else {
        1.0
    }
}
