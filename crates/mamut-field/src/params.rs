use super::*;

pub const GFM_K_NEIGHBORS: usize = 7;
pub const GFM_V1_WIDTH: usize = 16;
pub const GFM_V1_HEIGHT: usize = 16;

pub type GfmLattice16 = GfmLattice<GFM_V1_WIDTH, GFM_V1_HEIGHT>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GfmExcitation {
    pub pressure: f32,
    pub heat: f32,
    pub rupture_bias: f32,
}

impl GfmExcitation {
    pub const fn none() -> Self {
        Self {
            pressure: 0.0,
            heat: 0.0,
            rupture_bias: 0.0,
        }
    }

    pub(crate) fn sanitized(self) -> Self {
        Self {
            pressure: sanitize_f32(self.pressure, 0.0).clamp(0.0, 1.0),
            heat: sanitize_f32(self.heat, 0.0).clamp(0.0, 1.0),
            rupture_bias: sanitize_f32(self.rupture_bias, 0.0).clamp(0.0, 1.0),
        }
    }
}

impl Default for GfmExcitation {
    fn default() -> Self {
        Self::none()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum GfmPosture {
    #[default]
    Skeleton,
    Horizont,
    Pec,
    Baklja,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GfmParams {
    pub sample_rate_hz: f32,
    pub grav_coupling: f32,
    pub heat_dispersion: f32,
    pub output_gain: f32,
    pub posture: GfmPosture,
    pub spatial_spread: f32,
    pub omega_dispersion: f32,
    pub thermal_noise: f32,
    pub ruin: f32,
    pub rupture_threshold: f32,
    pub rupture_response: f32,
    pub rupture_quorum: usize,
    pub suspect_energy_ceiling: f32,
    pub suspect_strain_ceiling: f32,
    pub suspect_coupling_scale: f32,
    pub suspect_output_scale: f32,
    pub suspect_damping: f32,
    pub quarantine_after_samples: u16,
    pub recovery_after_samples: u16,
}

impl GfmParams {
    pub fn slice_a(sample_rate_hz: f32) -> Self {
        Self {
            sample_rate_hz,
            grav_coupling: 0.72,
            heat_dispersion: 0.28,
            output_gain: 0.42,
            posture: GfmPosture::Skeleton,
            spatial_spread: 0.36,
            omega_dispersion: 0.18,
            thermal_noise: 0.0,
            ruin: 0.0,
            rupture_threshold: 1.24,
            rupture_response: 0.0,
            rupture_quorum: 4,
            suspect_energy_ceiling: 1.36,
            suspect_strain_ceiling: 1.18,
            suspect_coupling_scale: 0.34,
            suspect_output_scale: 0.40,
            suspect_damping: 0.18,
            quarantine_after_samples: 160,
            recovery_after_samples: 360,
        }
    }

    pub fn horizont(sample_rate_hz: f32) -> Self {
        Self {
            grav_coupling: 0.34,
            heat_dispersion: 0.18,
            output_gain: 0.46,
            posture: GfmPosture::Horizont,
            spatial_spread: 0.82,
            omega_dispersion: 0.12,
            thermal_noise: 0.02,
            ruin: 0.03,
            rupture_threshold: 1.36,
            rupture_response: 0.02,
            rupture_quorum: 5,
            suspect_energy_ceiling: 1.42,
            suspect_strain_ceiling: 1.20,
            suspect_coupling_scale: 0.38,
            suspect_output_scale: 0.42,
            suspect_damping: 0.16,
            quarantine_after_samples: 220,
            recovery_after_samples: 420,
            ..Self::slice_a(sample_rate_hz)
        }
    }

    pub fn pec(sample_rate_hz: f32) -> Self {
        Self {
            grav_coupling: 0.68,
            heat_dispersion: 0.98,
            output_gain: 0.36,
            posture: GfmPosture::Pec,
            spatial_spread: 0.38,
            omega_dispersion: 0.98,
            thermal_noise: 0.48,
            ruin: 0.04,
            rupture_threshold: 1.56,
            rupture_response: 0.02,
            rupture_quorum: 6,
            suspect_energy_ceiling: 1.28,
            suspect_strain_ceiling: 1.08,
            suspect_coupling_scale: 0.30,
            suspect_output_scale: 0.34,
            suspect_damping: 0.24,
            quarantine_after_samples: 140,
            recovery_after_samples: 360,
            ..Self::slice_a(sample_rate_hz)
        }
    }

    pub fn baklja(sample_rate_hz: f32) -> Self {
        Self {
            grav_coupling: 1.42,
            heat_dispersion: 0.36,
            output_gain: 0.34,
            posture: GfmPosture::Baklja,
            spatial_spread: 0.30,
            omega_dispersion: 0.24,
            thermal_noise: 0.035,
            ruin: 0.98,
            rupture_threshold: 0.34,
            rupture_response: 0.86,
            rupture_quorum: 2,
            suspect_energy_ceiling: 1.18,
            suspect_strain_ceiling: 0.96,
            suspect_coupling_scale: 0.24,
            suspect_output_scale: 0.28,
            suspect_damping: 0.30,
            quarantine_after_samples: 96,
            recovery_after_samples: 300,
            ..Self::slice_a(sample_rate_hz)
        }
    }

    pub fn stress(sample_rate_hz: f32) -> Self {
        Self {
            grav_coupling: 1.92,
            heat_dispersion: 0.94,
            output_gain: 0.32,
            posture: GfmPosture::Baklja,
            spatial_spread: 0.66,
            omega_dispersion: 0.84,
            thermal_noise: 0.22,
            ruin: 0.86,
            rupture_threshold: 0.70,
            rupture_response: 0.62,
            rupture_quorum: 4,
            suspect_energy_ceiling: 0.24,
            suspect_strain_ceiling: 0.20,
            suspect_coupling_scale: 0.18,
            suspect_output_scale: 0.22,
            suspect_damping: 0.38,
            quarantine_after_samples: 48,
            recovery_after_samples: 180,
            ..Self::slice_a(sample_rate_hz)
        }
    }

    pub(crate) fn sanitized(self) -> Self {
        Self {
            sample_rate_hz: sanitize_positive(self.sample_rate_hz, 48_000.0),
            grav_coupling: sanitize_f32(self.grav_coupling, 0.72).clamp(0.0, 2.6),
            heat_dispersion: sanitize_f32(self.heat_dispersion, 0.28).clamp(0.0, 1.2),
            output_gain: sanitize_f32(self.output_gain, 0.42).clamp(0.0, 1.6),
            posture: self.posture,
            spatial_spread: sanitize_f32(self.spatial_spread, 0.36).clamp(0.0, 1.0),
            omega_dispersion: sanitize_f32(self.omega_dispersion, 0.18).clamp(0.0, 1.0),
            thermal_noise: sanitize_f32(self.thermal_noise, 0.0).clamp(0.0, 1.0),
            ruin: sanitize_f32(self.ruin, 0.0).clamp(0.0, 1.0),
            rupture_threshold: sanitize_f32(self.rupture_threshold, 1.24).clamp(0.10, 2.4),
            rupture_response: sanitize_f32(self.rupture_response, 0.0).clamp(0.0, 1.0),
            rupture_quorum: self.rupture_quorum.clamp(1, GFM_K_NEIGHBORS),
            suspect_energy_ceiling: sanitize_f32(self.suspect_energy_ceiling, 1.36)
                .clamp(0.20, 4.0),
            suspect_strain_ceiling: sanitize_f32(self.suspect_strain_ceiling, 1.18)
                .clamp(0.20, 4.0),
            suspect_coupling_scale: sanitize_f32(self.suspect_coupling_scale, 0.34).clamp(0.0, 1.0),
            suspect_output_scale: sanitize_f32(self.suspect_output_scale, 0.40).clamp(0.0, 1.0),
            suspect_damping: sanitize_f32(self.suspect_damping, 0.18).clamp(0.0, 1.0),
            quarantine_after_samples: self.quarantine_after_samples.max(1),
            recovery_after_samples: self.recovery_after_samples.max(1),
        }
    }

    pub(crate) fn topology_key(self) -> GfmTopologyKey {
        GfmTopologyKey {
            posture: self.posture,
            spatial_bucket: (self.spatial_spread.clamp(0.0, 1.0) * 256.0).round() as u16,
        }
    }
}

impl Default for GfmParams {
    fn default() -> Self {
        Self::slice_a(48_000.0)
    }
}
