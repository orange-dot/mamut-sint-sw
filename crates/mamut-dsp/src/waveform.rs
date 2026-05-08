use crate::{
    BandlimitedTriangle,
    bandlimited::{poly_blep_pulse_sample, poly_blep_saw_sample},
    lerp,
    oscillator::Oscillator,
};

#[allow(clippy::too_many_arguments)]
pub fn mixed_wave(
    oscillator: &Oscillator,
    mix_levels: [f32; 4],
    pulse_width: f32,
    noise_sample: f32,
    phase_mod: f32,
    saw_bend: f32,
    triangle_fold: f32,
    pulse_edge: f32,
) -> f32 {
    let phase = wrapped_phase(oscillator.phase() + phase_mod);
    let saw = saw_sample_bent(phase, saw_bend) * mix_levels[0];
    let pulse = pulse_sample_shaped(phase, pulse_width, pulse_edge) * mix_levels[1];
    let triangle = triangle_sample_folded(phase, triangle_fold) * mix_levels[2];
    let noise = noise_sample * mix_levels[3] * 0.85;
    normalize_weighted_mix(saw + pulse + triangle + noise, &mix_levels)
}

pub fn mixed_wave_osc2(
    oscillator: &Oscillator,
    mix_levels: [f32; 3],
    pulse_width: f32,
    phase_mod: f32,
    saw_bend: f32,
    triangle_fold: f32,
    pulse_edge: f32,
) -> f32 {
    let phase = wrapped_phase(oscillator.phase() + phase_mod);
    let saw = saw_sample_bent(phase, saw_bend) * mix_levels[0];
    let pulse = pulse_sample_shaped(phase, pulse_width, pulse_edge) * mix_levels[1];
    let triangle = triangle_sample_folded(phase, triangle_fold) * mix_levels[2];
    normalize_weighted_mix(saw + pulse + triangle, &mix_levels)
}

#[allow(clippy::too_many_arguments)]
pub fn mixed_wave_bandlimited(
    oscillator: &Oscillator,
    triangle_state: &mut BandlimitedTriangle,
    mix_levels: [f32; 4],
    pulse_width: f32,
    noise_sample: f32,
    phase_mod: f32,
    phase_step: f32,
    bandlimit: f32,
    saw_bend: f32,
    triangle_fold: f32,
    pulse_edge: f32,
) -> f32 {
    let phase = wrapped_phase(oscillator.phase() + phase_mod);
    let bandlimit = bandlimit.clamp(0.0, 1.0);
    if bandlimit <= f32::EPSILON {
        return mixed_wave(
            oscillator,
            mix_levels,
            pulse_width,
            noise_sample,
            phase_mod,
            saw_bend,
            triangle_fold,
            pulse_edge,
        );
    }

    let saw = bandlimited_saw_sample(phase, phase_step, bandlimit, saw_bend) * mix_levels[0];
    let pulse = bandlimited_pulse_sample(phase, pulse_width, phase_step, bandlimit, pulse_edge)
        * mix_levels[1];
    let triangle =
        bandlimited_triangle_sample(triangle_state, phase, phase_step, bandlimit, triangle_fold)
            * mix_levels[2];
    let noise = noise_sample * mix_levels[3] * 0.85;
    normalize_weighted_mix(saw + pulse + triangle + noise, &mix_levels)
}

#[allow(clippy::too_many_arguments)]
pub fn mixed_wave_osc2_bandlimited(
    oscillator: &Oscillator,
    triangle_state: &mut BandlimitedTriangle,
    mix_levels: [f32; 3],
    pulse_width: f32,
    phase_mod: f32,
    phase_step: f32,
    bandlimit: f32,
    saw_bend: f32,
    triangle_fold: f32,
    pulse_edge: f32,
) -> f32 {
    let phase = wrapped_phase(oscillator.phase() + phase_mod);
    let bandlimit = bandlimit.clamp(0.0, 1.0);
    if bandlimit <= f32::EPSILON {
        return mixed_wave_osc2(
            oscillator,
            mix_levels,
            pulse_width,
            phase_mod,
            saw_bend,
            triangle_fold,
            pulse_edge,
        );
    }

    let saw = bandlimited_saw_sample(phase, phase_step, bandlimit, saw_bend) * mix_levels[0];
    let pulse = bandlimited_pulse_sample(phase, pulse_width, phase_step, bandlimit, pulse_edge)
        * mix_levels[1];
    let triangle =
        bandlimited_triangle_sample(triangle_state, phase, phase_step, bandlimit, triangle_fold)
            * mix_levels[2];
    normalize_weighted_mix(saw + pulse + triangle, &mix_levels)
}

pub fn oscillator_preview_sample(
    oscillator: &Oscillator,
    pulse_width: f32,
    saw_bend: f32,
    triangle_fold: f32,
    pulse_edge: f32,
) -> f32 {
    let phase = oscillator.phase();
    (saw_sample_bent(phase, saw_bend)
        + pulse_sample_shaped(phase, pulse_width, pulse_edge)
        + triangle_sample_folded(phase, triangle_fold))
        * (1.0 / 3.0)
}

pub fn sine_phase_sample(oscillator: &Oscillator) -> f32 {
    (oscillator.phase() * std::f32::consts::TAU).sin()
}

pub(crate) fn normalize_weighted_mix<const N: usize>(
    sample_sum: f32,
    mix_levels: &[f32; N],
) -> f32 {
    let normalizer = mix_levels.iter().copied().sum::<f32>().max(1.0);
    sample_sum / normalizer
}

pub(crate) fn wrapped_phase(phase: f32) -> f32 {
    phase.rem_euclid(1.0)
}

fn saw_sample_bent(phase: f32, bend: f32) -> f32 {
    let bend = bend.clamp(-1.0, 1.0);
    if bend.abs() <= f32::EPSILON {
        return phase * 2.0 - 1.0;
    }

    let shaped = if bend > 0.0 {
        phase.powf(1.0 + bend * 2.5)
    } else {
        1.0 - (1.0 - phase).powf(1.0 + (-bend) * 2.5)
    };
    (shaped * 2.0 - 1.0).clamp(-1.0, 1.0)
}

fn bandlimited_saw_sample(phase: f32, phase_step: f32, bandlimit: f32, bend: f32) -> f32 {
    let raw = saw_sample_bent(phase, bend);
    let clean = poly_blep_saw_sample(phase, phase_step);
    lerp(raw, clean, bandlimit)
}

fn pulse_sample_shaped(phase: f32, width: f32, edge: f32) -> f32 {
    let width = width.clamp(0.05, 0.95);
    let edge = edge.clamp(0.0, 1.0);
    if edge <= f32::EPSILON {
        return if phase < width { 1.0 } else { -1.0 };
    }

    let edge_width = 0.003 + edge * 0.080;
    let falling = smooth_transition((phase - width) / edge_width);
    let rising = smooth_transition((phase - 1.0) / edge_width);
    let base = if phase < width { 1.0 } else { -1.0 };
    if (phase - width).abs() < edge_width {
        falling
    } else if phase > 1.0 - edge_width {
        -rising
    } else {
        base
    }
}

fn bandlimited_pulse_sample(
    phase: f32,
    width: f32,
    phase_step: f32,
    bandlimit: f32,
    edge: f32,
) -> f32 {
    let raw = pulse_sample_shaped(phase, width, edge);
    let clean = poly_blep_pulse_sample(phase, width, phase_step);
    lerp(raw, clean, bandlimit)
}

fn smooth_transition(x: f32) -> f32 {
    let t = ((x + 1.0) * 0.5).clamp(0.0, 1.0);
    let smooth = t * t * (3.0 - 2.0 * t);
    1.0 - smooth * 2.0
}

fn triangle_sample_folded(phase: f32, fold: f32) -> f32 {
    let triangle = 1.0 - 4.0 * (phase - 0.5).abs();
    let fold = fold.clamp(0.0, 1.0);
    if fold <= f32::EPSILON {
        return triangle;
    }

    let folded_phase = (phase * (1.0 + fold * 2.0)).fract();
    let folded = 1.0 - 4.0 * (folded_phase - 0.5).abs();
    (triangle * (1.0 - fold) + folded * fold).clamp(-1.0, 1.0)
}

fn bandlimited_triangle_sample(
    triangle_state: &mut BandlimitedTriangle,
    phase: f32,
    phase_step: f32,
    bandlimit: f32,
    fold: f32,
) -> f32 {
    let raw = triangle_sample_folded(phase, fold);
    let clean_base = triangle_state.next_sample(phase, phase_step);
    let fold = fold.clamp(0.0, 1.0);
    let clean_with_character = clean_base * (1.0 - fold) + raw * fold;
    lerp(raw, clean_with_character.clamp(-1.0, 1.0), bandlimit)
}
