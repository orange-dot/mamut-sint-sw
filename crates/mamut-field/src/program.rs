use super::*;

pub const GFM_PERFORMANCE_BASELINE_SEED: u64 = 0x6A46_4D40;
pub const GFM_PERFORMANCE_DURATION_SECONDS: usize = 14;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GfmPerformanceControls {
    pub depth: f32,
    pub heat: f32,
    pub spread: f32,
    pub rupture: f32,
    pub recovery: f32,
    pub motion: f32,
    pub body: f32,
    pub brightness: f32,
}

impl GfmPerformanceControls {
    pub const DEFAULT: Self = Self {
        depth: 0.55,
        heat: 0.35,
        spread: 0.50,
        rupture: 0.25,
        recovery: 0.55,
        motion: 0.35,
        body: 0.50,
        brightness: 0.45,
    };

    pub fn sanitized(self) -> Self {
        Self {
            depth: sanitize_unit(self.depth, Self::DEFAULT.depth),
            heat: sanitize_unit(self.heat, Self::DEFAULT.heat),
            spread: sanitize_unit(self.spread, Self::DEFAULT.spread),
            rupture: sanitize_unit(self.rupture, Self::DEFAULT.rupture),
            recovery: sanitize_unit(self.recovery, Self::DEFAULT.recovery),
            motion: sanitize_unit(self.motion, Self::DEFAULT.motion),
            body: sanitize_unit(self.body, Self::DEFAULT.body),
            brightness: sanitize_unit(self.brightness, Self::DEFAULT.brightness),
        }
    }
}

impl Default for GfmPerformanceControls {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GfmProgramId {
    HorizontPerformance,
    PecPerformance,
    BakljaPerformance,
}

impl GfmProgramId {
    pub const ALL_PERFORMANCE: [Self; 3] = [
        Self::HorizontPerformance,
        Self::PecPerformance,
        Self::BakljaPerformance,
    ];

    pub const fn wav_file_name(self) -> &'static str {
        match self {
            Self::HorizontPerformance => "performance_horizont.wav",
            Self::PecPerformance => "performance_pec.wav",
            Self::BakljaPerformance => "performance_baklja.wav",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GfmPerformanceProgram {
    id: GfmProgramId,
    params: GfmParams,
}

impl GfmPerformanceProgram {
    pub fn new(id: GfmProgramId, sample_rate_hz: f32) -> Self {
        Self::with_controls(id, sample_rate_hz, GfmPerformanceControls::DEFAULT)
    }

    pub fn with_controls(
        id: GfmProgramId,
        sample_rate_hz: f32,
        controls: GfmPerformanceControls,
    ) -> Self {
        let params = apply_performance_controls(base_params(id, sample_rate_hz), controls);

        Self { id, params }
    }

    pub fn params_for_controls(
        id: GfmProgramId,
        sample_rate_hz: f32,
        controls: GfmPerformanceControls,
    ) -> GfmParams {
        apply_performance_controls(base_params(id, sample_rate_hz), controls)
    }

    pub fn all(sample_rate_hz: f32) -> [Self; 3] {
        [
            Self::new(GfmProgramId::HorizontPerformance, sample_rate_hz),
            Self::new(GfmProgramId::PecPerformance, sample_rate_hz),
            Self::new(GfmProgramId::BakljaPerformance, sample_rate_hz),
        ]
    }

    pub const fn id(self) -> GfmProgramId {
        self.id
    }

    pub const fn params(self) -> GfmParams {
        self.params
    }

    pub const fn wav_file_name(self) -> &'static str {
        self.id.wav_file_name()
    }
}

fn base_params(id: GfmProgramId, sample_rate_hz: f32) -> GfmParams {
    match id {
        GfmProgramId::HorizontPerformance => {
            let mut params = GfmParams::horizont(sample_rate_hz);
            params.output_gain = 0.92;
            params
        }
        GfmProgramId::PecPerformance => {
            let mut params = GfmParams::pec(sample_rate_hz);
            params.output_gain = 0.86;
            params
        }
        GfmProgramId::BakljaPerformance => {
            let mut params = GfmParams::baklja(sample_rate_hz);
            params.output_gain = 0.38;
            params.ruin = 0.86;
            params.rupture_threshold = 0.42;
            params.rupture_response = 0.72;
            params.suspect_energy_ceiling = 2.75;
            params.suspect_strain_ceiling = 2.75;
            params.suspect_damping = 0.38;
            params.recovery_after_samples = 180;
            params
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GfmPerformanceGesture;

impl GfmPerformanceGesture {
    pub const fn v0_4() -> Self {
        Self
    }

    pub const fn duration_seconds(self) -> usize {
        GFM_PERFORMANCE_DURATION_SECONDS
    }

    pub fn frames(self, sample_rate_hz: u32) -> usize {
        (sample_rate_hz as usize).saturating_mul(self.duration_seconds())
    }

    pub fn excitation_at_frame(self, frame: usize, sample_rate_hz: u32) -> GfmExcitation {
        let sample_rate_hz = sanitize_positive(sample_rate_hz as f32, 48_000.0);
        let seconds = frame as f32 / sample_rate_hz;
        let slow = performance_slow_press(seconds);
        let accent = performance_rupture_accent(seconds);
        let pressure = performance_warm_strike(seconds) * 0.50
            + slow * 0.32
            + performance_paired_strikes(seconds) * 0.42
            + accent * 0.52;
        let pressure = pressure.clamp(0.0, 0.60);

        GfmExcitation {
            pressure,
            heat: pressure * 0.50 + slow * 0.10,
            rupture_bias: pressure * 0.46 + accent * 0.10,
        }
    }
}

fn performance_warm_strike(seconds: f32) -> f32 {
    if seconds < 0.020 {
        seconds / 0.020
    } else if seconds < 0.58 {
        1.0 - (seconds - 0.020) / 0.56
    } else {
        0.0
    }
}

fn performance_slow_press(seconds: f32) -> f32 {
    if seconds < 2.0 {
        0.0
    } else if seconds < 3.0 {
        seconds - 2.0
    } else if seconds < 4.8 {
        1.0
    } else if seconds < 5.8 {
        1.0 - (seconds - 4.8)
    } else {
        0.0
    }
}

fn performance_paired_strikes(seconds: f32) -> f32 {
    let starts = [6.70_f32, 7.28];
    let mut pressure = 0.0;

    for start in starts {
        let local = seconds - start;
        if (0.0..0.34).contains(&local) {
            let strike = if local < 0.018 {
                local / 0.018
            } else {
                1.0 - (local - 0.018) / 0.322
            };
            pressure += strike.clamp(0.0, 1.0);
        }
    }

    pressure.clamp(0.0, 1.0)
}

fn performance_rupture_accent(seconds: f32) -> f32 {
    let local = seconds - 8.80;
    if !(0.0..0.48).contains(&local) {
        return 0.0;
    }

    if local < 0.016 {
        local / 0.016
    } else {
        1.0 - (local - 0.016) / 0.464
    }
}

fn apply_performance_controls(
    mut params: GfmParams,
    controls: GfmPerformanceControls,
) -> GfmParams {
    let controls = controls.sanitized();
    let defaults = GfmPerformanceControls::DEFAULT;
    let depth = controls.depth - defaults.depth;
    let heat = controls.heat - defaults.heat;
    let spread = controls.spread - defaults.spread;
    let rupture = controls.rupture - defaults.rupture;
    let recovery = controls.recovery - defaults.recovery;
    let motion = controls.motion - defaults.motion;
    let body = controls.body - defaults.body;
    let brightness = controls.brightness - defaults.brightness;

    params.output_gain *= 1.0 + depth * 0.72 + body * 0.18 + brightness * 0.10;
    params.heat_dispersion += heat * 0.54 + brightness * 0.12;
    params.spatial_spread += spread * 0.44;
    params.omega_dispersion += motion * 0.50 + brightness * 0.12;
    params.thermal_noise += heat.max(0.0) * 0.16 + motion.max(0.0) * 0.08;
    params.grav_coupling *= 1.0 + body * 0.58 + depth * 0.16;
    params.ruin += rupture * 0.34;
    params.rupture_response += rupture * 0.30 + brightness * 0.08;
    params.rupture_threshold -= rupture * 0.22;
    params.suspect_damping += (0.0 - recovery) * 0.16 + rupture.max(0.0) * 0.08;
    params.suspect_coupling_scale += recovery * 0.10 - rupture.max(0.0) * 0.08;
    params.suspect_output_scale += recovery * 0.08 - rupture.max(0.0) * 0.06;
    params.recovery_after_samples = scale_u16(params.recovery_after_samples, 1.0 - recovery * 0.44);
    params.quarantine_after_samples = scale_u16(
        params.quarantine_after_samples,
        1.0 - recovery * 0.20 + rupture * 0.12,
    );

    params.sanitized()
}

fn scale_u16(value: u16, scale: f32) -> u16 {
    let value = value as f32 * sanitize_positive(scale, 1.0);
    value.round().clamp(1.0, u16::MAX as f32) as u16
}

fn sanitize_unit(value: f32, fallback: f32) -> f32 {
    sanitize_f32(value, fallback).clamp(0.0, 1.0)
}
