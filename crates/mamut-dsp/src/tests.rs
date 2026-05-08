#![allow(clippy::expect_used)]

use std::f32::consts::TAU;

use super::*;

#[test]
fn linear_smoother_reaches_target() {
    let mut smoother = LinearSmoother::new(0.0);
    smoother.set_target(1.0, 4);

    for _ in 0..4 {
        smoother.next_value();
    }

    assert_eq!(smoother.current(), 1.0);
}

#[test]
fn sanitize_non_finite_samples() {
    assert_eq!(sanitize_sample(f32::NAN), 0.0);
    assert_eq!(sanitize_sample(f32::INFINITY), 0.0);
}

#[test]
fn sanitize_flushes_tiny_samples() {
    assert_eq!(sanitize_sample(DENORMAL_FLUSH_ABS * 0.5), 0.0);
    assert_eq!(sanitize_sample(-DENORMAL_FLUSH_ABS * 0.5), 0.0);
    assert_eq!(
        sanitize_sample(DENORMAL_FLUSH_ABS * 2.0),
        DENORMAL_FLUSH_ABS * 2.0
    );
}

#[test]
fn envelope_reaches_release_idle() {
    let mut env = AdsrEnvelope::new(
        48_000.0,
        AdsrTiming {
            attack_ms: 1.0,
            decay_ms: 1.0,
            sustain: 0.5,
            release_ms: 1.0,
        },
    );
    env.note_on();
    for _ in 0..512 {
        env.next_sample();
    }
    env.note_off();
    for _ in 0..1024 {
        env.next_sample();
    }
    assert!(env.is_idle());
}

#[test]
fn filter_output_stays_finite() {
    let mut filter = StateVariableFilter::new();
    for _ in 0..1024 {
        let sample = filter.process(0.5, 1_200.0, 0.4, 0.3, 0.2, 48_000.0);
        assert!(sample.is_finite());
    }
}

#[test]
fn filter_survives_extreme_drive_and_strain() {
    let mut filter = StateVariableFilter::new();
    for _ in 0..4096 {
        let sample = filter.process(0.85, 8_400.0, 0.95, 0.95, 0.90, 48_000.0);
        assert!(sample.is_finite());
    }
}

#[test]
fn mixed_wave_helpers_stay_finite_and_bounded() {
    let mut oscillator = Oscillator::new();
    oscillator.set_phase(0.37);

    let osc1 = mixed_wave(
        &oscillator,
        [0.7, 0.4, 0.3, 0.2],
        0.42,
        -0.35,
        0.08,
        0.55,
        0.36,
        0.25,
    );
    let osc2 = mixed_wave_osc2(&oscillator, [0.8, 0.2, 0.5], 0.58, -0.04, -0.35, 0.42, 0.18);
    let preview = oscillator_preview_sample(&oscillator, 0.50, 0.25, 0.20, 0.15);
    let sine = sine_phase_sample(&oscillator);

    for sample in [osc1, osc2, preview, sine] {
        assert!(sample.is_finite());
        assert!(sample.abs() <= 1.0);
    }
}

#[test]
fn spectral_wavetable_samples_are_bounded_and_deterministic() {
    let mut oscillator = Oscillator::new();
    oscillator.set_phase(0.618);

    let first = spectral_wavetable_sample(&oscillator, 2.0, 0.73, 0.41);
    let second = spectral_wavetable_sample(&oscillator, 2.0, 0.73, 0.41);

    assert_eq!(SPECTRAL_WAVETABLE_SIZE, 256);
    assert_eq!(SPECTRAL_WAVETABLE_COUNT, 5);
    assert_eq!(first, second);
    assert!(first.is_finite());
    assert!(first.abs() <= 1.0);
}

#[test]
fn additive_helpers_clamp_and_shape_partials() {
    assert_eq!(additive_partial_count(2.0), 4);
    assert_eq!(additive_partial_count(9.0), 8);

    let fundamental = additive_ratio(0, 1.0, 1.0);
    let upper = additive_ratio(5, 0.42, 0.37);
    let odd_weight = additive_weight(0, 8, 0.75, 0.25);
    let even_weight = additive_weight(1, 8, 0.75, 0.25);

    assert_eq!(fundamental, 1.0);
    assert!(upper > fundamental);
    assert!(odd_weight > even_weight);
    assert!(odd_weight.is_finite());
    assert!(even_weight.is_finite());
}

#[test]
fn noise_rng_and_color_modes_are_deterministic_and_bounded() {
    let mut left = NoiseRng::new(7);
    let mut right = NoiseRng::new(7);
    let mut color_state = 0.0;

    for mode in 0..=3 {
        let raw = left.next_bipolar();
        assert_eq!(raw, right.next_bipolar());
        let colored = color_noise_sample(raw, mode as f32, &mut color_state);
        assert!(colored.is_finite());
        assert!(colored.abs() <= 1.0);
    }
}

#[test]
fn cross_mix_modes_remain_finite() {
    for mode in 0..=4 {
        let sample = cross_mix_sample(mode as f32, 0.35, -0.62);
        assert!(sample.is_finite());
        assert!(sample.abs() <= 1.0);
    }
}

#[test]
fn dc_blocker_removes_constant_bias() {
    let mut blocker = DcBlocker::new(48_000.0, 5.0);
    let mut output = 0.0;

    for _ in 0..48_000 {
        output = blocker.process(0.2);
    }

    assert!(output.abs() < 0.001, "output={output}");
}

#[test]
fn dc_blocker_preserves_audio_band_sine_level() {
    let mut blocker = DcBlocker::new(48_000.0, 5.0);
    let mut input_sum = 0.0;
    let mut output_sum = 0.0;
    let mut samples = 0_usize;

    for frame in 0..96_000 {
        let sample = (TAU * 100.0 * frame as f32 / 48_000.0).sin() * 0.4;
        let output = blocker.process(sample);
        assert!(output.is_finite());
        if frame >= 4_800 {
            input_sum += sample * sample;
            output_sum += output * output;
            samples += 1;
        }
    }

    let input_rms = (input_sum / samples as f32).sqrt();
    let output_rms = (output_sum / samples as f32).sqrt();
    let ratio = output_rms / input_rms;
    assert!((0.98..=1.02).contains(&ratio), "ratio={ratio}");
}

#[test]
fn dc_blocker_reset_clears_history() {
    let mut blocker = StereoDcBlocker::new(48_000.0, 5.0);
    for _ in 0..4_800 {
        blocker.process(0.2, -0.2);
    }

    blocker.reset();

    assert_eq!(blocker.process(0.0, 0.0), (0.0, 0.0));
}

#[test]
fn dc_blocker_flushes_tiny_silence_tail() {
    let mut blocker = DcBlocker::new(48_000.0, 5.0);
    let output = blocker.process(DENORMAL_FLUSH_ABS * 0.5);

    assert_eq!(output, 0.0);
    assert_eq!(blocker.process(0.0), 0.0);
}

#[test]
fn master_safety_limit_is_neutral_below_knee() {
    for sample in [-0.92, -0.5, 0.0, 0.5, 0.92] {
        assert_eq!(master_safety_limit(sample), sample);
    }
}

#[test]
fn master_safety_limit_is_finite_and_bounded() {
    for sample in [
        f32::NEG_INFINITY,
        -1000.0,
        -1.0,
        -0.95,
        0.95,
        1.0,
        1000.0,
        f32::INFINITY,
        f32::NAN,
    ] {
        let limited = master_safety_limit(sample);
        assert!(limited.is_finite());
        assert!(
            limited.abs() <= MASTER_SAFETY_CEILING,
            "sample={sample} limited={limited}"
        );
    }
}

#[test]
fn chorus_output_stays_finite_over_long_run() {
    let mut chorus = SimpleChorus::new(48_000.0);
    for step in 0..12_000 {
        let phase = step as f32 * 0.01;
        let (left, right) = chorus.process(phase.sin() * 0.7, phase.cos() * 0.6, 0.42, 0.64, 0.28);
        assert!(left.is_finite());
        assert!(right.is_finite());
    }
}

#[test]
fn reverb_output_stays_finite_over_long_run() {
    let mut reverb = SimpleReverb::new(48_000.0);
    for step in 0..18_000 {
        let impulse = if step % 512 == 0 { 0.9 } else { 0.0 };
        let (left, right) = reverb.process(impulse, impulse * 0.7, 0.36, 0.58, 0.44);
        assert!(left.is_finite());
        assert!(right.is_finite());
    }
}
