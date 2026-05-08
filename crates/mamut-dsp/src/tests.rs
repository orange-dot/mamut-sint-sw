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
fn lerp_and_mix_have_distinct_clamp_behavior() {
    assert_eq!(lerp(10.0, 20.0, -0.5), 5.0);
    assert_eq!(lerp(10.0, 20.0, 1.5), 25.0);
    assert_eq!(mix(10.0, 20.0, -0.5), 10.0);
    assert_eq!(mix(10.0, 20.0, 1.5), 20.0);
    assert_eq!(mix(10.0, 20.0, 0.25), 12.5);
}

#[test]
fn unipolar_and_bipolar_remap_round_trip_and_clamp() {
    for value in [0.0, 0.5, 1.0] {
        let bipolar = unipolar_to_bipolar(value);
        let round_trip = bipolar_to_unipolar(bipolar);
        assert!((round_trip - value).abs() <= f32::EPSILON);
    }

    assert_eq!(unipolar_to_bipolar(-0.5), -1.0);
    assert_eq!(unipolar_to_bipolar(1.5), 1.0);
    assert_eq!(bipolar_to_unipolar(-2.0), 0.0);
    assert_eq!(bipolar_to_unipolar(2.0), 1.0);
}

#[test]
fn gain_and_pitch_conversions_are_stable() {
    assert_eq!(db_to_gain(0.0), 1.0);
    assert!((db_to_gain(-6.0) - 0.501_187_2).abs() < 0.000_001);
    assert_eq!(gain_to_db(1.0), 0.0);
    assert!(gain_to_db(0.0).is_finite());
    assert!(gain_to_db(-1.0).is_finite());

    assert_eq!(semitones_to_ratio(12.0), 2.0);
    assert_eq!(cents_to_ratio(1200.0), 2.0);
    assert_eq!(midi_note_to_hz(69.0), 440.0);
    assert_eq!(midi_note_hz(69.0), midi_note_to_hz(69.0));
}

#[test]
fn equal_power_pan_matches_engine_formula() {
    let (hard_left_l, hard_left_r) = equal_power_pan(-1.0);
    assert_eq!(hard_left_l, 1.0);
    assert_eq!(hard_left_r, 0.0);

    let (center_l, center_r) = equal_power_pan(0.0);
    let expected_center = 0.5_f32.sqrt();
    assert_eq!(center_l, expected_center);
    assert_eq!(center_r, expected_center);

    let (hard_right_l, hard_right_r) = equal_power_pan(1.0);
    assert_eq!(hard_right_l, 0.0);
    assert_eq!(hard_right_r, 1.0);

    let (clamped_l, clamped_r) = equal_power_pan(2.0);
    assert_eq!((clamped_l, clamped_r), (hard_right_l, hard_right_r));
}

#[test]
fn soft_clip_and_smoothstep_are_bounded() {
    for sample in [-8.0, -1.0, 0.0, 1.0, 8.0] {
        let clipped = soft_clip(sample, 0.25);
        assert!(clipped.is_finite());
        assert!(clipped.abs() <= 1.0);
    }

    assert_eq!(smoothstep(-1.0), 0.0);
    assert_eq!(smoothstep(0.0), 0.0);
    assert_eq!(smoothstep(0.5), 0.5);
    assert_eq!(smoothstep(1.0), 1.0);
    assert_eq!(smoothstep(2.0), 1.0);
}

#[test]
fn phase_accumulator_advances_with_sample_rate() {
    let mut phase = PhaseAccumulator::new(48_000.0);

    assert_eq!(phase.sample_rate_hz(), 48_000.0);
    assert!(!phase.advance(480.0));
    assert!((phase.phase() - 0.01).abs() < 0.000_001);
}

#[test]
fn phase_accumulator_reports_wrap_and_wraps_phase() {
    let mut phase = PhaseAccumulator::with_phase(48_000.0, 0.75);

    assert!(phase.advance(24_000.0));
    assert!((phase.phase() - 0.25).abs() < 0.000_001);

    assert!(!phase.advance_by(0.25));
    assert!((phase.phase() - 0.50).abs() < 0.000_001);
}

#[test]
fn phase_accumulator_clamps_frequency_step_like_oscillator() {
    let phase = PhaseAccumulator::new(48_000.0);

    assert_eq!(phase.step_for_frequency(96_000.0), 0.5);
    assert_eq!(phase.step_for_frequency(-96_000.0), 0.0);
    assert_eq!(phase.step_for_frequency(f32::NAN), 0.0);

    let phase = PhaseAccumulator::new(0.0);
    assert_eq!(phase.sample_rate_hz(), 1.0);
    assert_eq!(phase.step_for_frequency(1.0), 0.5);
}

#[test]
fn phase_accumulator_reset_and_reset_to_normalize_phase() {
    let mut phase = PhaseAccumulator::with_phase(48_000.0, -1.25);

    assert_eq!(phase.phase(), 0.25);

    phase.reset_to(1.75);
    assert_eq!(phase.phase(), 0.75);

    phase.set_sample_rate(f32::INFINITY);
    assert_eq!(phase.sample_rate_hz(), 1.0);

    phase.reset();
    assert_eq!(phase.phase(), 0.0);
}

#[test]
fn phase_accumulator_hard_sync_matches_existing_oscillator_behavior() {
    let mut phase = PhaseAccumulator::with_phase(48_000.0, 0.80);

    phase.hard_sync(0.25);
    assert!((phase.phase() - 0.60).abs() < 0.000_001);

    phase.hard_sync(-1.0);
    assert!((phase.phase() - 0.60).abs() < 0.000_001);

    phase.hard_sync(2.0);
    assert_eq!(phase.phase(), 0.0);
}

#[test]
fn oscillator_keeps_existing_advance_and_sample_methods() {
    let mut oscillator = Oscillator::new();

    oscillator.set_phase(-1.25);
    assert_eq!(oscillator.phase(), 0.25);
    assert_eq!(oscillator.saw_sample(), -0.5);
    assert_eq!(oscillator.pulse_sample(0.5), 1.0);
    assert_eq!(oscillator.square_sample(), 1.0);
    assert_eq!(oscillator.triangle_sample(), 0.0);

    oscillator.set_phase(0.75);
    assert!(oscillator.advance(12_000.0, 48_000.0));
    assert_eq!(oscillator.phase(), 0.0);

    oscillator.set_phase(0.25);
    oscillator.hard_sync(0.5);
    assert_eq!(oscillator.phase(), 0.125);
}

#[test]
fn lfo_shapes_are_stable_at_known_phases() {
    let sine = Lfo::with_phase(48_000.0, LfoShape::Sine, 0.25);
    assert!((sine.sample_bipolar() - 1.0).abs() < 0.000_001);

    let triangle_low = Lfo::with_phase(48_000.0, LfoShape::Triangle, 0.0);
    let triangle_high = Lfo::with_phase(48_000.0, LfoShape::Triangle, 0.5);
    assert_eq!(triangle_low.sample_bipolar(), -1.0);
    assert_eq!(triangle_high.sample_bipolar(), 1.0);

    let ramp_up = Lfo::with_phase(48_000.0, LfoShape::RampUp, 0.25);
    let ramp_down = Lfo::with_phase(48_000.0, LfoShape::RampDown, 0.25);
    assert_eq!(ramp_up.sample_bipolar(), -0.5);
    assert_eq!(ramp_down.sample_bipolar(), 0.5);
    assert_eq!(ramp_up.sample_unipolar(), 0.25);
    assert_eq!(ramp_down.sample_unipolar(), 0.75);

    let square_high = Lfo::with_phase(48_000.0, LfoShape::Square, 0.49);
    let square_low = Lfo::with_phase(48_000.0, LfoShape::Square, 0.5);
    assert_eq!(square_high.sample_bipolar(), 1.0);
    assert_eq!(square_low.sample_bipolar(), -1.0);
}

#[test]
fn lfo_next_samples_before_advancing_and_wraps() {
    let mut lfo = Lfo::with_phase(4.0, LfoShape::Sine, 0.0);

    assert_eq!(lfo.next_bipolar(1.0), 0.0);
    assert!((lfo.phase() - 0.25).abs() < 0.000_001);
    assert!((lfo.next_bipolar(1.0) - 1.0).abs() < 0.000_001);
    assert!((lfo.phase() - 0.50).abs() < 0.000_001);

    let mut lfo = Lfo::with_phase(4.0, LfoShape::RampUp, 0.75);
    assert!(lfo.advance(1.0));
    assert_eq!(lfo.phase(), 0.0);

    lfo.set_shape(LfoShape::RampDown);
    assert_eq!(lfo.shape(), LfoShape::RampDown);
    assert_eq!(lfo.sample_bipolar_offset(0.25), 0.5);
}

#[test]
fn sample_hold_is_deterministic_and_updates_after_wrap_or_trigger() {
    let mut left = SampleHold::new(4.0, 17);
    let mut right = SampleHold::new(4.0, 17);
    assert_eq!(left.current(), right.current());
    assert_eq!(left.next_bipolar(0.0), right.next_bipolar(0.0));

    let mut hold = SampleHold::with_value(4.0, 11, 0.25);
    assert_eq!(hold.current(), 0.25);
    assert_eq!(hold.next_bipolar(2.0), 0.25);
    assert_eq!(hold.current(), 0.25);
    assert_eq!(hold.next_bipolar(2.0), 0.25);
    assert_ne!(hold.current(), 0.25);

    hold.reset(f32::INFINITY);
    assert_eq!(hold.current(), 0.0);

    let mut rng = NoiseRng::new(11);
    rng.next_bipolar();
    let expected = rng.next_bipolar();
    assert_eq!(hold.trigger(), expected);
    assert_eq!(
        hold.next_unipolar(0.0),
        (expected * 0.5 + 0.5).clamp(0.0, 1.0)
    );
}

#[test]
fn slew_limiter_limits_rise_and_fall_per_second() {
    let mut slew = SlewLimiter::new(10.0, 0.0);

    assert_eq!(slew.process(1.0, 5.0, 5.0), 0.5);
    assert_eq!(slew.process(1.0, 5.0, 5.0), 1.0);
    assert_eq!(slew.process(-1.0, 5.0, 10.0), 0.0);

    assert_eq!(slew.process(f32::NAN, 10.0, 10.0), 0.0);
    assert_eq!(slew.process(1.0, -1.0, 10.0), 0.0);

    slew.set_sample_rate(f32::INFINITY);
    assert_eq!(slew.sample_rate_hz(), 1.0);
    slew.reset(f32::NAN);
    assert_eq!(slew.current(), 0.0);
}

#[test]
fn tempo_and_rate_helpers_are_guarded() {
    assert_eq!(tempo_rate_hz(120.0), 2.0);
    assert_eq!(tempo_division_rate_hz(120.0, 4.0), 0.5);
    assert_eq!(beats_to_seconds(4.0, 120.0), 2.0);
    assert_eq!(hz_to_samples(2.0, 48_000.0), 24_000);

    assert_eq!(tempo_rate_hz(0.0), 0.0);
    assert_eq!(tempo_division_rate_hz(120.0, 0.0), 0.0);
    assert_eq!(beats_to_seconds(1.0, f32::NAN), 0.0);
    assert_eq!(hz_to_samples(-1.0, 48_000.0), 0);
}

#[test]
fn mono_block_ops_clear_gain_mix_and_report_stats() {
    let mut samples = [1.0, -2.0, 3.0, -4.0];
    let source = [0.5, 1.0, -1.5, 2.0, 99.0];
    let mut block = MonoBlockMut::new(&mut samples);

    assert_eq!(block.frames(), 4);
    assert_eq!(block.peak_abs(), 4.0);
    assert!((block.rms() - (30.0_f32 / 4.0).sqrt()).abs() < 0.000_001);
    assert_eq!(block.dc_offset(), -0.5);
    assert!(block.is_finite());

    block.apply_gain(0.5);
    assert_eq!(&block.samples[..], &[0.5, -1.0, 1.5, -2.0]);

    block.mix_from(&source, 2.0);
    assert_eq!(&block.samples[..], &[1.5, 1.0, -1.5, 2.0]);

    block.clear();
    assert_eq!(&block.samples[..], &[0.0; 4]);
    assert_eq!(block.peak_abs(), 0.0);
    assert_eq!(block.rms(), 0.0);
    assert_eq!(block.dc_offset(), 0.0);
}

#[test]
fn mono_block_empty_and_non_finite_stats_are_stable() {
    let mut empty = [];
    let block = MonoBlockMut::new(&mut empty);
    assert_eq!(block.frames(), 0);
    assert_eq!(block.peak_abs(), 0.0);
    assert_eq!(block.rms(), 0.0);
    assert_eq!(block.dc_offset(), 0.0);
    assert!(block.is_finite());

    let mut samples = [0.0, f32::NAN];
    let block = MonoBlockMut::new(&mut samples);
    assert!(!block.is_finite());
}

#[test]
fn stereo_block_ops_use_shared_frame_count() {
    let mut left = [1.0, -2.0, 3.0];
    let mut right = [-4.0, 5.0];
    let source_left = [0.5, 1.0, 99.0];
    let source_right = [-0.25, -1.0, 99.0];
    let mut block = StereoBlockMut::new(&mut left, &mut right);

    assert_eq!(block.frames(), 2);
    assert_eq!(block.peak_abs(), 5.0);
    assert!((block.rms() - (46.0_f32 / 4.0).sqrt()).abs() < 0.000_001);
    assert_eq!(block.dc_offset(), (-0.5, 0.5));
    assert!(block.is_finite());

    block.apply_gain(0.5);
    assert_eq!(&block.left[..2], &[0.5, -1.0]);
    assert_eq!(&block.right[..2], &[-2.0, 2.5]);
    assert_eq!(block.left[2], 3.0);

    block.mix_from(&source_left, &source_right, 2.0);
    assert_eq!(&block.left[..2], &[1.5, 1.0]);
    assert_eq!(&block.right[..2], &[-2.5, 0.5]);
    assert_eq!(block.left[2], 3.0);

    block.clear();
    assert_eq!(&block.left[..2], &[0.0, 0.0]);
    assert_eq!(&block.right[..2], &[0.0, 0.0]);
    assert_eq!(block.left[2], 3.0);
}

#[test]
fn stereo_block_empty_and_non_finite_stats_are_stable() {
    let mut left = [];
    let mut right = [1.0, 2.0];
    let block = StereoBlockMut::new(&mut left, &mut right);
    assert_eq!(block.frames(), 0);
    assert_eq!(block.peak_abs(), 0.0);
    assert_eq!(block.rms(), 0.0);
    assert_eq!(block.dc_offset(), (0.0, 0.0));
    assert!(block.is_finite());

    let mut left = [0.0, f32::INFINITY];
    let mut right = [0.0, 0.0];
    let block = StereoBlockMut::new(&mut left, &mut right);
    assert!(!block.is_finite());
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
