#![allow(clippy::expect_used, clippy::module_inception, clippy::panic)]

use super::*;

mod tests {
    use super::*;

    const TEST_RATE: f32 = 1_000.0;

    #[test]
    fn same_seed_and_params_render_byte_identical_pcm() {
        let params = GfmParams::slice_a(TEST_RATE);
        let first = render_pcm_signature(0xA11D_CAFE, params, 6_000);
        let second = render_pcm_signature(0xA11D_CAFE, params, 6_000);

        assert_eq!(first, second);
    }

    #[test]
    fn grav_and_heat_sweep_stays_finite() {
        for grav_coupling in [0.0, 0.25, 0.75, 1.25, 2.0] {
            for heat_dispersion in [0.0, 0.35, 0.70, 1.0] {
                let mut params = GfmParams::slice_a(TEST_RATE);
                params.grav_coupling = grav_coupling;
                params.heat_dispersion = heat_dispersion;
                let mut lattice = GfmLattice16::new(0x5157_1CE5, params);
                let stats = render_stats(&mut lattice, 4_000);
                assert!(stats.finite);
                assert!(stats.peak_abs <= 1.0);
            }
        }
    }

    #[test]
    fn low_and_high_grav_coupling_are_numerically_distinct() {
        let mut low = GfmParams::slice_a(TEST_RATE);
        low.grav_coupling = 0.05;
        low.heat_dispersion = 0.25;
        let mut high = low;
        high.grav_coupling = 1.85;

        let low_signature = render_pcm_signature(0x6469_6666, low, 8_000);
        let high_signature = render_pcm_signature(0x6469_6666, high, 8_000);

        assert_ne!(low_signature, high_signature);
        assert!(low_signature.abs_diff(high_signature) > 10_000);
    }

    #[test]
    fn deterministic_macro_sweep_is_finite() {
        for (index, params) in deterministic_macro_sweep(TEST_RATE).into_iter().enumerate() {
            let mut lattice = GfmLattice16::new(0x5EED_0000 + index as u64, params);
            let stats = render_stats(&mut lattice, 3_000);
            assert!(stats.finite);
            assert!(stats.peak_abs <= 1.0);
            assert_eq!(
                stats.diagnostics.health.total(),
                GFM_V1_WIDTH * GFM_V1_HEIGHT
            );
        }
    }

    #[test]
    fn ten_second_stress_render_is_finite_and_bounded() {
        let params = GfmParams::stress(TEST_RATE);
        let mut lattice = GfmLattice16::new(0x57A5_57E5, params);
        let stats = render_stats(&mut lattice, (TEST_RATE as usize) * 10);

        assert!(stats.finite);
        assert!(stats.rms < 0.55);
        assert!(stats.peak_abs <= 1.0);
        assert!(stats.diagnostics.suspect_damping_events > 0);
    }

    #[test]
    fn stress_render_signature_repeats_for_ten_runs() {
        let params = GfmParams::stress(TEST_RATE);
        let expected = render_pcm_signature(0xFACE_FEED, params, (TEST_RATE as usize) * 10);

        for _ in 0..10 {
            assert_eq!(
                expected,
                render_pcm_signature(0xFACE_FEED, params, (TEST_RATE as usize) * 10)
            );
        }
    }

    #[test]
    fn suspect_damping_limits_unstable_cells() {
        let mut damped = GfmParams::stress(TEST_RATE);
        damped.suspect_energy_ceiling = 0.20;
        damped.suspect_strain_ceiling = 0.20;
        damped.suspect_damping = 0.54;
        damped.suspect_coupling_scale = 0.08;

        let mut open = damped;
        open.suspect_damping = 0.0;
        open.suspect_coupling_scale = 1.0;
        open.suspect_output_scale = 1.0;

        let mut damped_lattice = GfmLattice16::new(0xDA4D_E000, damped);
        let mut open_lattice = GfmLattice16::new(0xDA4D_E000, open);
        let damped_stats = render_stats(&mut damped_lattice, 10_000);
        let open_stats = render_stats(&mut open_lattice, 10_000);

        assert!(damped_stats.diagnostics.suspect_damping_events > 0);
        assert!(damped_stats.rms < open_stats.rms);
        assert!(damped_stats.peak_abs <= open_stats.peak_abs);
    }

    #[test]
    fn three_postures_have_distinct_audio_without_probe_motion() {
        let seed = 0xBEE0_0001;
        let horizont = GfmParams::horizont(TEST_RATE);
        let pec = GfmParams::pec(TEST_RATE);
        let baklja = GfmParams::baklja(TEST_RATE);

        let horizont_lattice = GfmLattice16::new(seed, horizont);
        let pec_lattice = GfmLattice16::new(seed, pec);
        let baklja_lattice = GfmLattice16::new(seed, baklja);
        assert_eq!(
            horizont_lattice.probe_position(),
            pec_lattice.probe_position()
        );
        assert_eq!(
            pec_lattice.probe_position(),
            baklja_lattice.probe_position()
        );

        let horizont_signature = render_pcm_signature(seed, horizont, 12_000);
        let pec_signature = render_pcm_signature(seed, pec, 12_000);
        let baklja_signature = render_pcm_signature(seed, baklja, 12_000);
        let (pec_baklja_diff_rms, pec_baklja_corr) = render_pair_metrics(seed, pec, baklja, 12_000);
        let mut pec_stats_lattice = GfmLattice16::new(seed, pec);
        let mut baklja_stats_lattice = GfmLattice16::new(seed, baklja);
        let pec_stats = render_stats(&mut pec_stats_lattice, 12_000);
        let baklja_stats = render_stats(&mut baklja_stats_lattice, 12_000);

        assert_ne!(horizont_signature, pec_signature);
        assert_ne!(horizont_signature, baklja_signature);
        assert_ne!(pec_signature, baklja_signature);
        assert!(pec_baklja_diff_rms > 0.010);
        assert!(pec_baklja_corr < 0.985);
        assert_eq!(pec_stats.diagnostics.max_rupture_count, 0);
        assert!(baklja_stats.diagnostics.max_rupture_count > 0);
    }

    #[test]
    fn none_excitation_matches_plain_next_sample() {
        let params = GfmParams::pec(TEST_RATE);
        let mut plain = GfmLattice16::new(0x000E_0000, params);
        let mut explicit_none = GfmLattice16::new(0x000E_0000, params);

        for _ in 0..6_000 {
            assert_eq!(
                plain.next_sample().to_bits(),
                explicit_none
                    .next_sample_with_excitation(GfmExcitation::none())
                    .to_bits()
            );
        }
    }

    #[test]
    fn same_seed_and_gesture_render_byte_identical_pcm() {
        let params = GfmParams::baklja(TEST_RATE);
        let first = render_gesture_signature(0x6A46_4D20, params, TEST_RATE as usize * 6);
        let second = render_gesture_signature(0x6A46_4D20, params, TEST_RATE as usize * 6);

        assert_eq!(first, second);
    }

    #[test]
    fn gesture_postures_are_finite_bounded_and_distinct() {
        let seed = 0x6A46_4D20;
        let frames = TEST_RATE as usize * 6;
        let horizont = GfmParams::horizont(TEST_RATE);
        let pec = GfmParams::pec(TEST_RATE);
        let baklja = GfmParams::baklja(TEST_RATE);

        let horizont_lattice = GfmLattice16::new(seed, horizont);
        let pec_lattice = GfmLattice16::new(seed, pec);
        let baklja_lattice = GfmLattice16::new(seed, baklja);
        assert_eq!(
            horizont_lattice.probe_position(),
            pec_lattice.probe_position()
        );
        assert_eq!(
            pec_lattice.probe_position(),
            baklja_lattice.probe_position()
        );

        let horizont_stats = render_gesture_stats(seed, horizont, frames);
        let pec_stats = render_gesture_stats(seed, pec, frames);
        let baklja_stats = render_gesture_stats(seed, baklja, frames);
        let (horizont_pec_diff_rms, horizont_pec_corr) =
            render_gesture_pair_metrics(seed, horizont, pec, frames);
        let (pec_baklja_diff_rms, pec_baklja_corr) =
            render_gesture_pair_metrics(seed, pec, baklja, frames);

        assert!(horizont_stats.finite);
        assert!(pec_stats.finite);
        assert!(baklja_stats.finite);
        assert!(horizont_stats.peak_abs <= 1.0);
        assert!(pec_stats.peak_abs <= 1.0);
        assert!(baklja_stats.peak_abs <= 1.0);
        assert_eq!(horizont_stats.diagnostics.max_rupture_count, 0);
        assert_eq!(pec_stats.diagnostics.max_rupture_count, 0);
        assert!(horizont_pec_diff_rms > 0.004);
        assert!(horizont_pec_corr < 0.995);
        assert!(baklja_stats.diagnostics.max_rupture_count > 0);
        assert!(
            baklja_stats.diagnostics.max_rupture_count <= (GFM_V1_WIDTH * GFM_V1_HEIGHT) / 3,
            "gesture Baklja rupture count exceeded local limit: {}",
            baklja_stats.diagnostics.max_rupture_count
        );
        assert!(pec_baklja_diff_rms > 0.010);
        assert!(pec_baklja_corr < 0.985);
        assert!(
            baklja_stats.diagnostics.health.healthy > 0,
            "gesture Baklja should recover some healthy cells after the attack-hold-release gesture"
        );
    }

    #[test]
    fn same_seed_and_variant_render_byte_identical_pcm() {
        let params = GfmParams::baklja(TEST_RATE);
        let first = render_variant_signature(0x6A46_4D30, params, TEST_RATE as usize * 12);
        let second = render_variant_signature(0x6A46_4D30, params, TEST_RATE as usize * 12);

        assert_eq!(first, second);
    }

    #[test]
    fn gesture_variants_are_finite_bounded_and_distinct() {
        let seed = 0x6A46_4D30;
        let frames = TEST_RATE as usize * 12;
        let horizont = GfmParams::horizont(TEST_RATE);
        let pec = GfmParams::pec(TEST_RATE);
        let baklja = GfmParams::baklja(TEST_RATE);

        let horizont_stats = render_variant_stats(seed, horizont, frames);
        let pec_stats = render_variant_stats(seed, pec, frames);
        let baklja_stats = render_variant_stats(seed, baklja, frames);
        let (horizont_pec_diff_rms, horizont_pec_corr) =
            render_variant_pair_metrics(seed, horizont, pec, frames);
        let (pec_baklja_diff_rms, pec_baklja_corr) =
            render_variant_pair_metrics(seed, pec, baklja, frames);

        assert!(horizont_stats.finite);
        assert!(pec_stats.finite);
        assert!(baklja_stats.finite);
        assert!(horizont_stats.peak_abs <= 1.0);
        assert!(pec_stats.peak_abs <= 1.0);
        assert!(baklja_stats.peak_abs <= 1.0);
        assert_eq!(horizont_stats.diagnostics.max_rupture_count, 0);
        assert_eq!(pec_stats.diagnostics.max_rupture_count, 0);
        assert!(baklja_stats.diagnostics.max_rupture_count > 0);
        assert!(
            baklja_stats.diagnostics.max_rupture_count <= (GFM_V1_WIDTH * GFM_V1_HEIGHT * 2) / 5,
            "variant Baklja rupture count exceeded combined-gesture limit: {}",
            baklja_stats.diagnostics.max_rupture_count
        );
        assert!(horizont_pec_diff_rms > 0.004);
        assert!(horizont_pec_corr < 0.995);
        assert!(pec_baklja_diff_rms > 0.010);
        assert!(pec_baklja_corr < 0.985);
    }

    #[test]
    fn same_seed_and_performance_render_byte_identical_pcm() {
        let program = GfmPerformanceProgram::new(GfmProgramId::BakljaPerformance, TEST_RATE);
        let gesture = GfmPerformanceGesture::v0_4();
        let frames = gesture.frames(TEST_RATE as u32);
        let first =
            render_performance_signature(GFM_PERFORMANCE_BASELINE_SEED, program, gesture, frames);
        let second =
            render_performance_signature(GFM_PERFORMANCE_BASELINE_SEED, program, gesture, frames);

        assert_eq!(first, second);
    }

    #[test]
    fn performance_take_is_finite_bounded_and_recovery_safe() {
        let seed = GFM_PERFORMANCE_BASELINE_SEED;
        let gesture = GfmPerformanceGesture::v0_4();
        let frames = gesture.frames(TEST_RATE as u32);
        let horizont = GfmPerformanceProgram::new(GfmProgramId::HorizontPerformance, TEST_RATE);
        let pec = GfmPerformanceProgram::new(GfmProgramId::PecPerformance, TEST_RATE);
        let baklja = GfmPerformanceProgram::new(GfmProgramId::BakljaPerformance, TEST_RATE);

        let horizont_lattice = GfmLattice16::new(seed, horizont.params());
        let pec_lattice = GfmLattice16::new(seed, pec.params());
        let baklja_lattice = GfmLattice16::new(seed, baklja.params());
        assert_eq!(
            horizont_lattice.probe_position(),
            pec_lattice.probe_position()
        );
        assert_eq!(
            pec_lattice.probe_position(),
            baklja_lattice.probe_position()
        );

        let horizont_stats = render_performance_stats(seed, horizont, gesture, frames);
        let pec_stats = render_performance_stats(seed, pec, gesture, frames);
        let baklja_stats = render_performance_stats(seed, baklja, gesture, frames);
        let (horizont_pec_diff_rms, horizont_pec_corr) =
            render_performance_pair_metrics(seed, horizont, pec, gesture, frames);
        let (pec_baklja_diff_rms, pec_baklja_corr) =
            render_performance_pair_metrics(seed, pec, baklja, gesture, frames);
        let baklja_final_recovered =
            baklja_stats.diagnostics.health.healthy + baklja_stats.diagnostics.health.recovering;

        assert!(horizont_stats.finite);
        assert!(pec_stats.finite);
        assert!(baklja_stats.finite);
        assert!(horizont_stats.peak_abs <= 1.0);
        assert!(pec_stats.peak_abs <= 1.0);
        assert!(baklja_stats.peak_abs <= 1.0);
        assert_eq!(horizont_stats.diagnostics.max_rupture_count, 0);
        assert_eq!(pec_stats.diagnostics.max_rupture_count, 0);
        assert!(baklja_stats.diagnostics.max_rupture_count > 0);
        assert!(
            baklja_stats.diagnostics.max_rupture_count <= (GFM_V1_WIDTH * GFM_V1_HEIGHT * 2) / 5,
            "performance Baklja rupture count exceeded local limit: {}",
            baklja_stats.diagnostics.max_rupture_count
        );
        assert!(
            baklja_final_recovered >= (GFM_V1_WIDTH * GFM_V1_HEIGHT * 3) / 4,
            "performance Baklja should end mostly healthy/recovering: {:?}",
            baklja_stats.diagnostics.health
        );
        assert!(horizont_pec_diff_rms > 0.004);
        assert!(horizont_pec_corr < 0.995);
        assert!(pec_baklja_diff_rms > 0.010);
        assert!(pec_baklja_corr < 0.985);
    }

    #[test]
    fn performance_program_params_match_v0_4_evidence() {
        let programs = GfmPerformanceProgram::all(TEST_RATE);
        assert_eq!(
            programs.map(GfmPerformanceProgram::id),
            GfmProgramId::ALL_PERFORMANCE
        );
        assert_eq!(GfmPerformanceGesture::v0_4().duration_seconds(), 14);
        assert_eq!(GfmPerformanceGesture::v0_4().frames(1_000), 14_000);

        let horizont = programs[0].params();
        assert_eq!(programs[0].wav_file_name(), "performance_horizont.wav");
        assert_eq!(horizont.posture, GfmPosture::Horizont);
        assert_eq!(horizont.output_gain, 0.92);
        assert_eq!(horizont.grav_coupling, 0.34);
        assert_eq!(horizont.heat_dispersion, 0.18);

        let pec = programs[1].params();
        assert_eq!(programs[1].wav_file_name(), "performance_pec.wav");
        assert_eq!(pec.posture, GfmPosture::Pec);
        assert_eq!(pec.output_gain, 0.86);
        assert_eq!(pec.grav_coupling, 0.68);
        assert_eq!(pec.heat_dispersion, 0.98);

        let baklja = programs[2].params();
        assert_eq!(programs[2].wav_file_name(), "performance_baklja.wav");
        assert_eq!(baklja.posture, GfmPosture::Baklja);
        assert_eq!(baklja.output_gain, 0.38);
        assert_eq!(baklja.grav_coupling, 1.42);
        assert_eq!(baklja.heat_dispersion, 0.36);
        assert_eq!(baklja.ruin, 0.86);
        assert_eq!(baklja.rupture_threshold, 0.42);
        assert_eq!(baklja.rupture_response, 0.72);
        assert_eq!(baklja.suspect_energy_ceiling, 2.75);
        assert_eq!(baklja.suspect_strain_ceiling, 2.75);
        assert_eq!(baklja.suspect_damping, 0.38);
        assert_eq!(baklja.recovery_after_samples, 180);
    }

    #[test]
    fn performance_controls_are_neutral_at_default_and_bounded_at_extremes() {
        for program_id in GfmProgramId::ALL_PERFORMANCE {
            let baseline = GfmPerformanceProgram::new(program_id, TEST_RATE).params();
            let neutral = GfmPerformanceProgram::params_for_controls(
                program_id,
                TEST_RATE,
                GfmPerformanceControls::DEFAULT,
            );
            assert_eq!(baseline, neutral);

            let low = GfmPerformanceProgram::params_for_controls(
                program_id,
                TEST_RATE,
                GfmPerformanceControls {
                    depth: 0.0,
                    heat: 0.0,
                    spread: 0.0,
                    rupture: 0.0,
                    recovery: 1.0,
                    motion: 0.0,
                    body: 0.0,
                    brightness: 0.0,
                },
            );
            let high = GfmPerformanceProgram::params_for_controls(
                program_id,
                TEST_RATE,
                GfmPerformanceControls {
                    depth: 1.0,
                    heat: 1.0,
                    spread: 1.0,
                    rupture: 1.0,
                    recovery: 0.0,
                    motion: 1.0,
                    body: 1.0,
                    brightness: 1.0,
                },
            );
            let mut low_lattice = GfmLattice16::new(0xC047_7001, low);
            let mut high_lattice = GfmLattice16::new(0xC047_7001, high);
            let low_stats = render_stats(&mut low_lattice, 4_000);
            let high_stats = render_stats(&mut high_lattice, 4_000);

            assert!(low_stats.finite);
            assert!(high_stats.finite);
            assert!(low_stats.peak_abs <= 1.0);
            assert!(high_stats.peak_abs <= 1.0);
            assert_ne!(
                render_pcm_signature(0xC047_7001, low, 4_000),
                render_pcm_signature(0xC047_7001, high, 4_000)
            );
        }
    }

    #[test]
    fn baklja_rupture_quorum_stays_local() {
        let mut params = GfmParams::baklja(TEST_RATE);
        params.ruin = 0.95;
        params.rupture_threshold = 0.64;
        params.rupture_quorum = 4;
        let mut lattice = GfmLattice16::new(0xBAA1_1A00, params);
        let stats = render_stats(&mut lattice, 24_000);
        let max_allowed = (GFM_V1_WIDTH * GFM_V1_HEIGHT) / 3;

        assert!(stats.finite);
        assert!(stats.diagnostics.max_rupture_count <= max_allowed);
    }

    #[test]
    fn quarantine_and_recovery_do_not_break_rendering() {
        let mut params = GfmParams::stress(TEST_RATE);
        params.suspect_energy_ceiling = 0.20;
        params.suspect_strain_ceiling = 0.20;
        params.quarantine_after_samples = 8;
        params.recovery_after_samples = 4_000;
        let mut lattice = GfmLattice16::new(0x0C0D_EC0D, params);
        let stats = render_stats(&mut lattice, 10_000);
        let health = stats.diagnostics.health;

        assert!(stats.finite);
        assert!(stats.diagnostics.suspect_damping_events > 0);
        assert!(health.quarantined + health.recovering + health.suspect > 0);
        assert!(stats.peak_abs <= 1.0);
    }

    fn render_pcm_signature(seed: u64, params: GfmParams, frames: usize) -> u64 {
        let mut lattice = GfmLattice16::new(seed, params);
        let mut signature = 0xcbf2_9ce4_8422_2325_u64;
        for _ in 0..frames {
            let sample = sample_to_pcm16(lattice.next_sample());
            signature ^= sample as u16 as u64;
            signature = signature.wrapping_mul(0x100_0000_01b3);
        }
        signature
    }

    fn gesture_excitation(frame: usize) -> GfmExcitation {
        let seconds = frame as f32 / TEST_RATE;
        let pressure = if seconds < 0.02 {
            seconds / 0.02
        } else if seconds < 1.20 {
            1.0
        } else if seconds < 2.10 {
            1.0 - (seconds - 1.20) / 0.90
        } else {
            0.0
        };

        GfmExcitation {
            pressure,
            heat: pressure * 0.55,
            rupture_bias: pressure * 0.70,
        }
    }

    fn render_gesture_signature(seed: u64, params: GfmParams, frames: usize) -> u64 {
        let mut lattice = GfmLattice16::new(seed, params);
        let mut signature = 0xcbf2_9ce4_8422_2325_u64;
        for frame in 0..frames {
            let sample =
                sample_to_pcm16(lattice.next_sample_with_excitation(gesture_excitation(frame)));
            signature ^= sample as u16 as u64;
            signature = signature.wrapping_mul(0x100_0000_01b3);
        }
        signature
    }

    fn render_gesture_stats(seed: u64, params: GfmParams, frames: usize) -> GfmRenderStats {
        let mut lattice = GfmLattice16::new(seed, params);
        let mut sum_squares = 0.0;
        let mut peak_abs = 0.0;
        let mut finite = true;

        for frame in 0..frames {
            let sample = lattice.next_sample_with_excitation(gesture_excitation(frame));
            finite &= sample.is_finite();
            peak_abs = f32::max(peak_abs, sample.abs());
            sum_squares += sample * sample;
        }

        let rms = if frames > 0 {
            (sum_squares / frames as f32).sqrt()
        } else {
            0.0
        };

        GfmRenderStats {
            frames,
            finite,
            rms,
            peak_abs,
            diagnostics: lattice.diagnostics(),
        }
    }

    fn render_gesture_pair_metrics(
        seed: u64,
        left_params: GfmParams,
        right_params: GfmParams,
        frames: usize,
    ) -> (f32, f32) {
        let mut left = GfmLattice16::new(seed, left_params);
        let mut right = GfmLattice16::new(seed, right_params);
        let mut left_power = 0.0;
        let mut right_power = 0.0;
        let mut cross_power = 0.0;
        let mut diff_power = 0.0;

        for frame in 0..frames {
            let excitation = gesture_excitation(frame);
            let left_sample = left.next_sample_with_excitation(excitation);
            let right_sample = right.next_sample_with_excitation(excitation);
            left_power += left_sample * left_sample;
            right_power += right_sample * right_sample;
            cross_power += left_sample * right_sample;
            let diff = left_sample - right_sample;
            diff_power += diff * diff;
        }

        let diff_rms = (diff_power / frames as f32).sqrt();
        let denominator = (left_power * right_power).sqrt().max(0.000_001);
        (diff_rms, cross_power / denominator)
    }

    fn variant_excitation(frame: usize) -> GfmExcitation {
        let seconds = frame as f32 / TEST_RATE;
        let pressure = short_strike(seconds) * 0.80
            + slow_press(seconds) * 0.45
            + repeated_strike(seconds) * 0.70;
        let pressure = pressure.clamp(0.0, 0.85);

        GfmExcitation {
            pressure,
            heat: pressure * 0.58,
            rupture_bias: pressure * 0.62,
        }
    }

    fn short_strike(seconds: f32) -> f32 {
        if seconds < 0.015 {
            seconds / 0.015
        } else if seconds < 0.42 {
            1.0 - (seconds - 0.015) / 0.405
        } else {
            0.0
        }
    }

    fn slow_press(seconds: f32) -> f32 {
        if seconds < 3.0 {
            0.0
        } else if seconds < 4.0 {
            (seconds - 3.0) / 1.0
        } else if seconds < 6.2 {
            1.0
        } else if seconds < 7.5 {
            1.0 - (seconds - 6.2) / 1.3
        } else {
            0.0
        }
    }

    fn repeated_strike(seconds: f32) -> f32 {
        let strike_starts = [8.2_f32, 8.85, 9.50];
        let mut pressure = 0.0;

        for start in strike_starts {
            let local = seconds - start;
            if (0.0..0.36).contains(&local) {
                let strike = if local < 0.018 {
                    local / 0.018
                } else {
                    1.0 - (local - 0.018) / 0.342
                };
                pressure += strike.clamp(0.0, 1.0);
            }
        }

        pressure.clamp(0.0, 1.0)
    }

    fn render_variant_signature(seed: u64, params: GfmParams, frames: usize) -> u64 {
        let mut lattice = GfmLattice16::new(seed, params);
        let mut signature = 0xcbf2_9ce4_8422_2325_u64;
        for frame in 0..frames {
            let sample =
                sample_to_pcm16(lattice.next_sample_with_excitation(variant_excitation(frame)));
            signature ^= sample as u16 as u64;
            signature = signature.wrapping_mul(0x100_0000_01b3);
        }
        signature
    }

    fn render_variant_stats(seed: u64, params: GfmParams, frames: usize) -> GfmRenderStats {
        let mut lattice = GfmLattice16::new(seed, params);
        let mut sum_squares = 0.0;
        let mut peak_abs = 0.0;
        let mut finite = true;

        for frame in 0..frames {
            let sample = lattice.next_sample_with_excitation(variant_excitation(frame));
            finite &= sample.is_finite();
            peak_abs = f32::max(peak_abs, sample.abs());
            sum_squares += sample * sample;
        }

        let rms = if frames > 0 {
            (sum_squares / frames as f32).sqrt()
        } else {
            0.0
        };

        GfmRenderStats {
            frames,
            finite,
            rms,
            peak_abs,
            diagnostics: lattice.diagnostics(),
        }
    }

    fn render_variant_pair_metrics(
        seed: u64,
        left_params: GfmParams,
        right_params: GfmParams,
        frames: usize,
    ) -> (f32, f32) {
        let mut left = GfmLattice16::new(seed, left_params);
        let mut right = GfmLattice16::new(seed, right_params);
        let mut left_power = 0.0;
        let mut right_power = 0.0;
        let mut cross_power = 0.0;
        let mut diff_power = 0.0;

        for frame in 0..frames {
            let excitation = variant_excitation(frame);
            let left_sample = left.next_sample_with_excitation(excitation);
            let right_sample = right.next_sample_with_excitation(excitation);
            left_power += left_sample * left_sample;
            right_power += right_sample * right_sample;
            cross_power += left_sample * right_sample;
            let diff = left_sample - right_sample;
            diff_power += diff * diff;
        }

        let diff_rms = (diff_power / frames as f32).sqrt();
        let denominator = (left_power * right_power).sqrt().max(0.000_001);
        (diff_rms, cross_power / denominator)
    }

    fn render_performance_signature(
        seed: u64,
        program: GfmPerformanceProgram,
        gesture: GfmPerformanceGesture,
        frames: usize,
    ) -> u64 {
        let mut lattice = GfmLattice16::new(seed, program.params());
        let mut signature = 0xcbf2_9ce4_8422_2325_u64;
        for frame in 0..frames {
            let sample =
                sample_to_pcm16(lattice.next_sample_with_excitation(
                    gesture.excitation_at_frame(frame, TEST_RATE as u32),
                ));
            signature ^= sample as u16 as u64;
            signature = signature.wrapping_mul(0x100_0000_01b3);
        }
        signature
    }

    fn render_performance_stats(
        seed: u64,
        program: GfmPerformanceProgram,
        gesture: GfmPerformanceGesture,
        frames: usize,
    ) -> GfmRenderStats {
        let mut lattice = GfmLattice16::new(seed, program.params());
        let mut sum_squares = 0.0;
        let mut peak_abs = 0.0;
        let mut finite = true;

        for frame in 0..frames {
            let sample = lattice
                .next_sample_with_excitation(gesture.excitation_at_frame(frame, TEST_RATE as u32));
            finite &= sample.is_finite();
            peak_abs = f32::max(peak_abs, sample.abs());
            sum_squares += sample * sample;
        }

        let rms = if frames > 0 {
            (sum_squares / frames as f32).sqrt()
        } else {
            0.0
        };

        GfmRenderStats {
            frames,
            finite,
            rms,
            peak_abs,
            diagnostics: lattice.diagnostics(),
        }
    }

    fn render_performance_pair_metrics(
        seed: u64,
        left_program: GfmPerformanceProgram,
        right_program: GfmPerformanceProgram,
        gesture: GfmPerformanceGesture,
        frames: usize,
    ) -> (f32, f32) {
        let mut left = GfmLattice16::new(seed, left_program.params());
        let mut right = GfmLattice16::new(seed, right_program.params());
        let mut left_power = 0.0;
        let mut right_power = 0.0;
        let mut cross_power = 0.0;
        let mut diff_power = 0.0;

        for frame in 0..frames {
            let excitation = gesture.excitation_at_frame(frame, TEST_RATE as u32);
            let left_sample = left.next_sample_with_excitation(excitation);
            let right_sample = right.next_sample_with_excitation(excitation);
            left_power += left_sample * left_sample;
            right_power += right_sample * right_sample;
            cross_power += left_sample * right_sample;
            let diff = left_sample - right_sample;
            diff_power += diff * diff;
        }

        let diff_rms = (diff_power / frames as f32).sqrt();
        let denominator = (left_power * right_power).sqrt().max(0.000_001);
        (diff_rms, cross_power / denominator)
    }

    fn render_pair_metrics(
        seed: u64,
        left_params: GfmParams,
        right_params: GfmParams,
        frames: usize,
    ) -> (f32, f32) {
        let mut left = GfmLattice16::new(seed, left_params);
        let mut right = GfmLattice16::new(seed, right_params);
        let mut left_power = 0.0;
        let mut right_power = 0.0;
        let mut cross_power = 0.0;
        let mut diff_power = 0.0;

        for _ in 0..frames {
            let left_sample = left.next_sample();
            let right_sample = right.next_sample();
            left_power += left_sample * left_sample;
            right_power += right_sample * right_sample;
            cross_power += left_sample * right_sample;
            let diff = left_sample - right_sample;
            diff_power += diff * diff;
        }

        let diff_rms = (diff_power / frames as f32).sqrt();
        let denominator = (left_power * right_power).sqrt().max(0.000_001);
        (diff_rms, cross_power / denominator)
    }
}
