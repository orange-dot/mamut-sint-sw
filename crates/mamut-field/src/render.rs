use super::*;

pub fn deterministic_macro_sweep(sample_rate_hz: f32) -> [GfmParams; 12] {
    let mut low_grav = GfmParams::slice_a(sample_rate_hz);
    low_grav.grav_coupling = 0.0;
    low_grav.heat_dispersion = 0.0;

    let mut medium_grav = GfmParams::slice_a(sample_rate_hz);
    medium_grav.grav_coupling = 0.8;
    medium_grav.heat_dispersion = 0.35;

    let mut high_grav = GfmParams::slice_a(sample_rate_hz);
    high_grav.grav_coupling = 2.0;
    high_grav.heat_dispersion = 0.50;

    let mut hot = GfmParams::slice_a(sample_rate_hz);
    hot.grav_coupling = 1.2;
    hot.heat_dispersion = 1.0;

    let mut cold_lock = GfmParams::slice_a(sample_rate_hz);
    cold_lock.grav_coupling = 2.0;
    cold_lock.heat_dispersion = 0.0;

    let mut diffuse = GfmParams::slice_a(sample_rate_hz);
    diffuse.grav_coupling = 0.2;
    diffuse.heat_dispersion = 1.0;

    [
        low_grav,
        medium_grav,
        high_grav,
        hot,
        cold_lock,
        diffuse,
        GfmParams::horizont(sample_rate_hz),
        GfmParams::pec(sample_rate_hz),
        GfmParams::baklja(sample_rate_hz),
        GfmParams::stress(sample_rate_hz),
        {
            let mut params = GfmParams::stress(sample_rate_hz);
            params.suspect_damping = 0.52;
            params.suspect_coupling_scale = 0.08;
            params
        },
        {
            let mut params = GfmParams::baklja(sample_rate_hz);
            params.rupture_quorum = GFM_K_NEIGHBORS;
            params.rupture_threshold = 1.05;
            params
        },
    ]
}

pub fn render_stats<const W: usize, const H: usize>(
    lattice: &mut GfmLattice<W, H>,
    frames: usize,
) -> GfmRenderStats {
    let mut sum_squares = 0.0;
    let mut peak_abs = 0.0;
    let mut finite = true;

    for _ in 0..frames {
        let sample = lattice.next_sample();
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

pub fn sample_to_pcm16(sample: f32) -> i16 {
    (sanitize_sample(sample) * i16::MAX as f32).round() as i16
}
