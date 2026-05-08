use std::sync::LazyLock;

use crate::{oscillator::Oscillator, waveform::wrapped_phase};

pub const SPECTRAL_WAVETABLE_SIZE: usize = 256;
pub const SPECTRAL_WAVETABLE_COUNT: usize = 5;

static SPECTRAL_WAVETABLES: LazyLock<[[f32; SPECTRAL_WAVETABLE_SIZE]; SPECTRAL_WAVETABLE_COUNT]> =
    LazyLock::new(build_spectral_wavetables);

pub fn spectral_wavetable_sample(
    oscillator: &Oscillator,
    table_index: f32,
    position: f32,
    morph: f32,
) -> f32 {
    let table_index = table_index
        .round()
        .clamp(0.0, (SPECTRAL_WAVETABLE_COUNT - 1) as f32);
    let anchor = sample_spectral_table(table_index as usize, oscillator.phase());
    let scan_position = position.clamp(0.0, 1.0) * (SPECTRAL_WAVETABLE_COUNT - 1) as f32;
    let scan = sample_spectral_bank(scan_position, oscillator.phase());
    (anchor * (1.0 - morph.clamp(0.0, 1.0)) + scan * morph.clamp(0.0, 1.0)).clamp(-1.0, 1.0)
}

fn sample_spectral_bank(table_position: f32, phase: f32) -> f32 {
    let lower = table_position.floor() as usize;
    let upper = (lower + 1).min(SPECTRAL_WAVETABLE_COUNT - 1);
    let blend = table_position - lower as f32;
    let a = sample_spectral_table(lower, phase);
    let b = sample_spectral_table(upper, phase);
    a + (b - a) * blend
}

fn sample_spectral_table(table: usize, phase: f32) -> f32 {
    let phase = wrapped_phase(phase);
    let position = phase * SPECTRAL_WAVETABLE_SIZE as f32;
    let lower = position.floor() as usize % SPECTRAL_WAVETABLE_SIZE;
    let upper = (lower + 1) % SPECTRAL_WAVETABLE_SIZE;
    let blend = position - lower as f32;
    let table = &SPECTRAL_WAVETABLES[table.min(SPECTRAL_WAVETABLE_COUNT - 1)];
    table[lower] + (table[upper] - table[lower]) * blend
}

fn build_spectral_wavetables() -> [[f32; SPECTRAL_WAVETABLE_SIZE]; SPECTRAL_WAVETABLE_COUNT] {
    let mut tables = [[0.0; SPECTRAL_WAVETABLE_SIZE]; SPECTRAL_WAVETABLE_COUNT];
    for (table, table_samples) in tables.iter_mut().enumerate() {
        let mut peak: f32 = 0.0;
        for (index, sample_slot) in table_samples.iter_mut().enumerate() {
            let phase = index as f32 / SPECTRAL_WAVETABLE_SIZE as f32;
            let sample = analytic_spectral_sample(table, phase);
            *sample_slot = sample;
            peak = peak.max(sample.abs());
        }
        let normalizer = peak.max(0.001);
        for sample in table_samples {
            *sample = (*sample / normalizer).clamp(-1.0, 1.0);
        }
    }
    tables
}

fn analytic_spectral_sample(table: usize, phase: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    match table {
        1 => {
            (tau * phase).sin() * 0.72
                + (tau * phase * 2.0).sin() * 0.22
                + (tau * phase * 3.0).sin() * 0.30
                + (tau * phase * 5.0).sin() * 0.18
        }
        2 => {
            (tau * phase).sin() * 0.38
                + (tau * phase * 2.71).sin() * 0.30
                + (tau * phase * 4.93).sin() * 0.24
                + (tau * phase * 8.01).sin() * 0.16
        }
        3 => {
            (tau * phase).sin() * 0.84 - (tau * phase * 2.0).sin() * 0.42
                + (tau * phase * 4.0).sin() * 0.20
                - (tau * phase * 8.0).sin() * 0.10
        }
        4 => {
            (tau * phase).sin() * 0.48
                + (tau * phase * 3.0).sin() * 0.42
                + (tau * phase * 7.0).sin() * 0.22
                + (tau * phase * 11.0).sin() * 0.12
        }
        _ => (tau * phase).sin() * 0.92 + (tau * phase * 2.0).sin() * 0.08,
    }
}
