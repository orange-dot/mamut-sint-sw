use super::*;

const MOZAIK_TEST_BLOCK_FRAMES: usize = 256;
const MOZAIK_TEST_BLOCKS: usize = 200;

fn render_with_note(engine: &mut Engine, blocks: usize) -> Vec<(u32, u32)> {
    let mut rendered = Vec::with_capacity(blocks * MOZAIK_TEST_BLOCK_FRAMES);
    let mut left = [0.0_f32; MOZAIK_TEST_BLOCK_FRAMES];
    let mut right = [0.0_f32; MOZAIK_TEST_BLOCK_FRAMES];
    for block in 0..blocks {
        let note_events = if block == 0 {
            vec![Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: 48,
                    velocity: 0.85,
                },
            }]
        } else {
            Vec::new()
        };
        engine.process_block(ProcessBlock {
            frame_count: MOZAIK_TEST_BLOCK_FRAMES,
            note_events: &note_events,
            controller_events: &[],
            macro_state: None,
            output: Some(StereoBlockMut::new(&mut left, &mut right)),
        });
        for index in 0..MOZAIK_TEST_BLOCK_FRAMES {
            rendered.push((left[index].to_bits(), right[index].to_bits()));
        }
    }
    rendered
}

#[test]
fn mozaik_disabled_render_is_bit_identical_to_untouched_engine() {
    let mut untouched = fixture_engine();
    let baseline = render_with_note(&mut untouched, MOZAIK_TEST_BLOCKS);

    let mut toggled = fixture_engine();
    toggled.set_mozaik_mode(MozaikMode::Enabled {
        seed: DEFAULT_MOZAIK_SEED,
    });
    toggled.set_mozaik_mode(MozaikMode::Disabled);
    let disabled = render_with_note(&mut toggled, MOZAIK_TEST_BLOCKS);

    assert_eq!(baseline, disabled);
}

#[test]
fn mozaik_enabled_render_differs_and_carries_audible_energy() {
    let mut disabled_engine = fixture_engine();
    let disabled = render_with_note(&mut disabled_engine, MOZAIK_TEST_BLOCKS);

    let mut enabled_engine = fixture_engine();
    enabled_engine.set_mozaik_mode(MozaikMode::Enabled {
        seed: DEFAULT_MOZAIK_SEED,
    });
    let enabled = render_with_note(&mut enabled_engine, MOZAIK_TEST_BLOCKS);

    assert_ne!(disabled, enabled);

    // Bare `mozaik on` with the on-enable defaults must be audible on a held
    // note: measure the difference-signal RMS across the held region.
    let mut diff_power = 0.0_f64;
    let mut count = 0_usize;
    for (before, after) in disabled.iter().zip(enabled.iter()) {
        let left_delta = f64::from(f32::from_bits(after.0)) - f64::from(f32::from_bits(before.0));
        let right_delta = f64::from(f32::from_bits(after.1)) - f64::from(f32::from_bits(before.1));
        diff_power += left_delta * left_delta + right_delta * right_delta;
        count += 2;
    }
    let diff_rms = (diff_power / count.max(1) as f64).sqrt();
    assert!(
        diff_rms > 0.005,
        "mozaik default blend too quiet: {diff_rms}"
    );

    for (left_bits, right_bits) in &enabled {
        assert!(f32::from_bits(*left_bits).is_finite());
        assert!(f32::from_bits(*right_bits).is_finite());
    }
}

#[test]
fn mozaik_enabled_render_is_bit_deterministic() {
    let render = || {
        let mut engine = fixture_engine();
        engine.set_mozaik_mode(MozaikMode::Enabled { seed: 0x5EED });
        engine.set_mozaik_param(MozaikParam::Drift, 0.6);
        engine.set_mozaik_param(MozaikParam::Contrast, 0.8);
        render_with_note(&mut engine, 80)
    };
    assert_eq!(render(), render());
}

#[test]
fn mozaik_controls_clamp_and_sanitize_hostile_values() {
    let mut engine = fixture_engine();
    engine.set_mozaik_mode(MozaikMode::Enabled {
        seed: DEFAULT_MOZAIK_SEED,
    });

    let snapshot = engine.set_mozaik_param(MozaikParam::Mix, f32::NAN);
    assert_eq!(snapshot.mix, MOZAIK_DEFAULT_MIX);
    let snapshot = engine.set_mozaik_param(MozaikParam::Mix, 5.0);
    assert_eq!(snapshot.mix, 1.0);
    let snapshot = engine.set_mozaik_param(MozaikParam::Slope, f32::NEG_INFINITY);
    assert_eq!(snapshot.slope, MOZAIK_DEFAULT_SLOPE_CONTROL);
    let snapshot = engine.set_mozaik_param(MozaikParam::Contrast, -3.0);
    assert_eq!(snapshot.contrast, 0.0);
    assert_eq!(snapshot.contrast_gamma, 1.0);
    let snapshot = engine.set_mozaik_param(MozaikParam::Phason, f32::INFINITY);
    assert_eq!(snapshot.phason, MOZAIK_DEFAULT_PHASON);
    let snapshot = engine.set_mozaik_param(MozaikParam::Drift, 2.0);
    assert_eq!(snapshot.drift, 1.0);

    let rendered = render_with_note(&mut engine, 40);
    for (left_bits, right_bits) in rendered {
        assert!(f32::from_bits(left_bits).is_finite());
        assert!(f32::from_bits(right_bits).is_finite());
    }
}

#[test]
fn mozaik_slope_mapping_snaps_detents_and_prefers_golden() {
    // The default slope control sits exactly on the golden detent and maps
    // back to the exact Q32 constant.
    let (sigma, snapped) = mozaik_sigma_from_control(MOZAIK_DEFAULT_SLOPE_CONTROL);
    assert!(snapped);
    assert_eq!(sigma, MOZAIK_SIGMA_DETENTS[0].0);
    assert_eq!(
        mozaik_slope_q32_from_sigma(sigma),
        mamut_dsp::MOZAIK_SLOPE_GOLDEN_Q32
    );

    // Near-golden snaps to golden.
    let control = (0.616_f32 - MOZAIK_SLOPE_SIGMA_MIN) / MOZAIK_SLOPE_SIGMA_SPAN;
    let (sigma, snapped) = mozaik_sigma_from_control(control);
    assert!(snapped);
    assert_eq!(sigma, MOZAIK_SIGMA_DETENTS[0].0);

    // In the golden/5-8 overlap the nearest detent wins: 0.6215 is closer to
    // golden than to 5/8 even though both snap windows contain it.
    let control = (0.621_5_f32 - MOZAIK_SLOPE_SIGMA_MIN) / MOZAIK_SLOPE_SIGMA_SPAN;
    let (sigma, snapped) = mozaik_sigma_from_control(control);
    assert!(snapped);
    assert_eq!(sigma, MOZAIK_SIGMA_DETENTS[0].0);

    // Just past the midpoint 5/8 wins.
    let control = (0.623_5_f32 - MOZAIK_SLOPE_SIGMA_MIN) / MOZAIK_SLOPE_SIGMA_SPAN;
    let (sigma, snapped) = mozaik_sigma_from_control(control);
    assert!(snapped);
    assert_eq!(sigma, MOZAIK_SIGMA_DETENTS[3].0);
    assert_eq!(
        mozaik_slope_q32_from_sigma(sigma),
        mamut_dsp::MOZAIK_SLOPE_DETENT_FIVE_EIGHTHS_Q32
    );

    // Between detents no snap happens and the morph passes through freely.
    let control = (0.58_f32 - MOZAIK_SLOPE_SIGMA_MIN) / MOZAIK_SLOPE_SIGMA_SPAN;
    let (sigma, snapped) = mozaik_sigma_from_control(control);
    assert!(!snapped);
    assert!((sigma - 0.58).abs() < 1.0e-6);
}

#[test]
fn mozaik_reset_controllers_restores_on_enable_defaults() {
    let mut engine = fixture_engine();
    engine.set_mozaik_mode(MozaikMode::Enabled {
        seed: DEFAULT_MOZAIK_SEED,
    });
    engine.set_mozaik_param(MozaikParam::Mix, 0.9);
    engine.set_mozaik_param(MozaikParam::Slope, 0.1);
    engine.set_mozaik_param(MozaikParam::Contrast, 1.0);
    engine.set_mozaik_param(MozaikParam::Phason, 0.7);
    engine.set_mozaik_param(MozaikParam::Drift, 1.0);

    engine.reset_controllers();
    let snapshot = engine.snapshot().mozaik;
    assert_eq!(snapshot.mix, MOZAIK_DEFAULT_MIX);
    assert_eq!(snapshot.slope, MOZAIK_DEFAULT_SLOPE_CONTROL);
    assert_eq!(snapshot.contrast, MOZAIK_DEFAULT_CONTRAST_CONTROL);
    assert_eq!(snapshot.phason, MOZAIK_DEFAULT_PHASON);
    assert_eq!(snapshot.drift, MOZAIK_DEFAULT_DRIFT);
    assert_eq!(snapshot.drift_phason, 0.0);
    assert!(matches!(snapshot.mode, MozaikMode::Enabled { .. }));
}

#[test]
fn mozaik_drift_advances_the_phason_offset() {
    let mut engine = fixture_engine();
    engine.set_mozaik_mode(MozaikMode::Enabled {
        seed: DEFAULT_MOZAIK_SEED,
    });
    engine.set_mozaik_param(MozaikParam::Drift, 1.0);
    render_with_note(&mut engine, 40);
    let snapshot = engine.snapshot().mozaik;
    assert!(snapshot.drift_phason > 0.0);
    assert!(snapshot.drift_phason < 1.0);
}

#[test]
fn mozaik_voice_phason_bases_are_distinct_per_slot() {
    let mut bases = Vec::new();
    for slot in 0..6 {
        bases.push(mozaik_voice_phason_q32(DEFAULT_MOZAIK_SEED, slot));
    }
    for (left_index, left) in bases.iter().enumerate() {
        for right in bases.iter().skip(left_index + 1) {
            assert_ne!(left, right);
        }
    }
    // Different seeds move every base.
    for slot in 0..6 {
        assert_ne!(
            mozaik_voice_phason_q32(DEFAULT_MOZAIK_SEED, slot),
            mozaik_voice_phason_q32(DEFAULT_MOZAIK_SEED ^ 1, slot),
        );
    }
}

#[test]
fn mozaik_snapshot_reports_disabled_defaults_on_fresh_engine() {
    let engine = fixture_engine();
    let snapshot = engine.snapshot().mozaik;
    assert_eq!(snapshot.mode, MozaikMode::Disabled);
    assert_eq!(snapshot.mix, 0.0);
    assert_eq!(snapshot.effective_mix, 0.0);
    assert_eq!(snapshot.drift_phason, 0.0);
}
