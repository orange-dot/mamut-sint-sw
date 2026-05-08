use super::*;

pub const GFM_PERFORMANCE_BASELINE_SEED: u64 = 0x6A46_4D40;
pub const GFM_PERFORMANCE_DURATION_SECONDS: usize = 14;

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
        let params = match id {
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
        };

        Self { id, params }
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
