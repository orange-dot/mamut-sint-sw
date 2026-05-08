use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GfmTopologyKey {
    pub(crate) posture: GfmPosture,
    pub(crate) spatial_bucket: u16,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct GfmCell {
    pub(crate) phase: f32,
    pub(crate) velocity: f32,
    pub(crate) base_velocity: f32,
    pub(crate) detune: f32,
    pub(crate) energy: f32,
    pub(crate) strain: f32,
    pub(crate) fracture: f32,
    pub(crate) heat: f32,
    pub(crate) coherence: f32,
    pub(crate) health: GfmCellHealth,
    pub(crate) unsafe_run: u16,
    pub(crate) stable_run: u16,
    pub(crate) rupture_cooldown: u16,
}

impl GfmCell {
    pub(crate) const fn silent() -> Self {
        Self {
            phase: 0.0,
            velocity: 0.0,
            base_velocity: 0.0,
            detune: 0.0,
            energy: 0.0,
            strain: 0.0,
            fracture: 0.0,
            heat: 0.0,
            coherence: 1.0,
            health: GfmCellHealth::Healthy,
            unsafe_run: 0,
            stable_run: 0,
            rupture_cooldown: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct GfmNeighbor {
    pub(crate) x: u16,
    pub(crate) y: u16,
    pub(crate) weight: f32,
}

impl GfmNeighbor {
    pub(crate) const fn empty() -> Self {
        Self {
            x: 0,
            y: 0,
            weight: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct GfmCellTopology {
    pub(crate) neighbors: [GfmNeighbor; GFM_K_NEIGHBORS],
}

impl GfmCellTopology {
    pub(crate) const fn empty() -> Self {
        Self {
            neighbors: [GfmNeighbor::empty(); GFM_K_NEIGHBORS],
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct GfmCellScratch {
    pub(crate) energy_gradient: f32,
    pub(crate) strain_gradient: f32,
    pub(crate) heat_gradient: f32,
    pub(crate) coherence_gradient: f32,
    pub(crate) phase_pull: f32,
    pub(crate) rupture_votes: usize,
    pub(crate) rupture_allowed: bool,
}

impl GfmCellScratch {
    pub(crate) const fn empty() -> Self {
        Self {
            energy_gradient: 0.0,
            strain_gradient: 0.0,
            heat_gradient: 0.0,
            coherence_gradient: 0.0,
            phase_pull: 0.0,
            rupture_votes: 0,
            rupture_allowed: false,
        }
    }
}

pub(crate) fn seeded_cell<const W: usize, const H: usize>(
    x: usize,
    y: usize,
    params: GfmParams,
    rng: &mut u64,
) -> GfmCell {
    let nx = if W > 1 {
        x as f32 / (W - 1) as f32
    } else {
        0.0
    };
    let ny = if H > 1 {
        y as f32 / (H - 1) as f32
    } else {
        0.0
    };
    let random_phase = next_unit(rng);
    let random_detune = next_bipolar(rng);
    let center_x = 0.5;
    let center_y = 0.5;
    let distance = ((nx - center_x).powi(2) + (ny - center_y).powi(2)).sqrt();
    let center_bias = (1.0 - distance * 2.2).clamp(0.0, 1.0);
    let base_hz = 64.0 + nx * 112.0 + ny * 173.0 + center_bias * 54.0;
    let base_velocity = (base_hz / params.sample_rate_hz.max(1.0)).clamp(0.000_08, 0.45);

    GfmCell {
        phase: random_phase,
        velocity: base_velocity,
        base_velocity,
        detune: random_detune * 0.42,
        energy: 0.10 + center_bias * 0.40 + next_unit(rng) * 0.035,
        strain: 0.04 + center_bias * 0.08 + next_unit(rng) * 0.025,
        fracture: 0.0,
        heat: params.heat_dispersion * (0.04 + next_unit(rng) * 0.03),
        coherence: (0.74 + center_bias * 0.18 + next_unit(rng) * 0.06).clamp(0.0, 1.0),
        health: GfmCellHealth::Healthy,
        unsafe_run: 0,
        stable_run: 0,
        rupture_cooldown: 0,
    }
}

pub(crate) fn build_topology<const W: usize, const H: usize>(
    x: usize,
    y: usize,
    seed: u64,
    posture: GfmPosture,
    spatial_spread: f32,
) -> GfmCellTopology {
    let local_weight = match posture {
        GfmPosture::Skeleton => 1.00,
        GfmPosture::Horizont => 0.76,
        GfmPosture::Pec => 0.92,
        GfmPosture::Baklja => 1.08,
    };
    let long_weight = match posture {
        GfmPosture::Skeleton => 0.42,
        GfmPosture::Horizont => 0.74,
        GfmPosture::Pec => 0.48,
        GfmPosture::Baklja => 0.36,
    };
    let spread = (2.0 + spatial_spread.clamp(0.0, 1.0) * 7.0).round() as usize;
    let skew = ((seed >> ((x + y) % 17)) as usize)
        .wrapping_add(x * 31)
        .wrapping_add(y * 17);

    GfmCellTopology {
        neighbors: [
            GfmNeighbor {
                x: wrap_index(x as isize - 1, W) as u16,
                y: y as u16,
                weight: local_weight,
            },
            GfmNeighbor {
                x: wrap_index(x as isize + 1, W) as u16,
                y: y as u16,
                weight: local_weight,
            },
            GfmNeighbor {
                x: x as u16,
                y: wrap_index(y as isize - 1, H) as u16,
                weight: local_weight,
            },
            GfmNeighbor {
                x: x as u16,
                y: wrap_index(y as isize + 1, H) as u16,
                weight: local_weight,
            },
            GfmNeighbor {
                x: wrap_index(x as isize + spread as isize + (skew % 3) as isize, W) as u16,
                y: wrap_index(y as isize + 3 + (skew % 5) as isize, H) as u16,
                weight: long_weight,
            },
            GfmNeighbor {
                x: wrap_index(x as isize - 3 - (skew % 7) as isize, W) as u16,
                y: wrap_index(y as isize + spread as isize, H) as u16,
                weight: long_weight * 0.86,
            },
            GfmNeighbor {
                x: wrap_index(x as isize + 5 + (y % 4) as isize, W) as u16,
                y: wrap_index(y as isize - 4 - (x % 5) as isize, H) as u16,
                weight: long_weight * 0.72,
            },
        ],
    }
}

pub(crate) fn center_excitation<const W: usize, const H: usize>(
    x: usize,
    y: usize,
    center_x: usize,
    center_y: usize,
    params: GfmParams,
) -> f32 {
    let dx = toroidal_distance(x, center_x, W) as f32;
    let dy = toroidal_distance(y, center_y, H) as f32;
    let distance = (dx * dx + dy * dy).sqrt();
    let radius = match params.posture {
        GfmPosture::Horizont => 4.2,
        GfmPosture::Pec => 2.8,
        GfmPosture::Baklja => 1.5,
        GfmPosture::Skeleton => 2.4,
    };
    let force = (1.0 - distance / radius).clamp(0.0, 1.0);
    let posture_drive = match params.posture {
        GfmPosture::Baklja => 1.55,
        GfmPosture::Pec => 1.08,
        GfmPosture::Skeleton | GfmPosture::Horizont => 1.0,
    };
    force
        * posture_drive
        * (0.000_16 + params.grav_coupling * 0.000_04 + params.heat_dispersion * 0.000_06)
}

pub(crate) fn excitation_footprint<const W: usize, const H: usize>(
    x: usize,
    y: usize,
    center_x: usize,
    center_y: usize,
    params: GfmParams,
) -> f32 {
    let dx = toroidal_distance(x, center_x, W) as f32;
    let dy = toroidal_distance(y, center_y, H) as f32;
    let distance = (dx * dx + dy * dy).sqrt();
    let radius = match params.posture {
        GfmPosture::Horizont => 4.6,
        GfmPosture::Pec => 3.0,
        GfmPosture::Baklja => 1.8,
        GfmPosture::Skeleton => 2.5,
    };

    let normalized = (1.0 - distance / radius).clamp(0.0, 1.0);
    normalized * normalized
}

pub(crate) fn posture_velocity_tilt<const W: usize, const H: usize>(
    x: usize,
    y: usize,
    params: GfmParams,
) -> f32 {
    let nx = if W > 1 {
        x as f32 / (W - 1) as f32
    } else {
        0.0
    };
    let ny = if H > 1 {
        y as f32 / (H - 1) as f32
    } else {
        0.0
    };

    match params.posture {
        GfmPosture::Skeleton => 1.0,
        GfmPosture::Horizont => 0.78 + nx * 0.36,
        GfmPosture::Pec => 0.86 + ((nx - 0.5).abs() + (ny - 0.5).abs()) * 0.58,
        GfmPosture::Baklja => 0.94 + ((x * 31 + y * 17) % 17) as f32 * (0.30 / 16.0),
    }
}

pub(crate) fn update_health(cell: &mut GfmCell, params: GfmParams) {
    let unsafe_now = !cell.phase.is_finite()
        || !cell.velocity.is_finite()
        || !cell.energy.is_finite()
        || !cell.strain.is_finite()
        || cell.energy > params.suspect_energy_ceiling
        || cell.strain > params.suspect_strain_ceiling
        || cell.velocity.abs() > 0.45;

    if unsafe_now {
        cell.unsafe_run = cell.unsafe_run.saturating_add(1);
        cell.stable_run = 0;
    } else {
        cell.stable_run = cell.stable_run.saturating_add(1);
        cell.unsafe_run = 0;
    }

    match cell.health {
        GfmCellHealth::Healthy => {
            if unsafe_now {
                cell.health = GfmCellHealth::Suspect;
            }
        }
        GfmCellHealth::Suspect => {
            cell.energy *= 1.0 - params.suspect_damping * 0.42;
            cell.strain *= 1.0 - params.suspect_damping * 0.30;
            cell.velocity *= 1.0 - params.suspect_damping * 0.08;
            if cell.unsafe_run >= params.quarantine_after_samples {
                cell.health = GfmCellHealth::Quarantined;
            } else if cell.stable_run >= params.recovery_after_samples / 2 {
                cell.health = GfmCellHealth::Recovering;
            }
        }
        GfmCellHealth::Quarantined => {
            cell.energy *= 1.0 - params.suspect_damping * 0.62;
            cell.strain *= 1.0 - params.suspect_damping * 0.52;
            cell.fracture *= 1.0 - params.suspect_damping * 0.48;
            cell.velocity *= 1.0 - params.suspect_damping * 0.14;
            if cell.stable_run >= params.recovery_after_samples {
                cell.health = GfmCellHealth::Recovering;
            }
        }
        GfmCellHealth::Recovering => {
            cell.energy *= 1.0 - params.suspect_damping * 0.12;
            cell.strain *= 1.0 - params.suspect_damping * 0.10;
            if unsafe_now {
                cell.health = GfmCellHealth::Suspect;
            } else if cell.stable_run >= params.recovery_after_samples {
                cell.health = GfmCellHealth::Healthy;
            }
        }
    }
}

pub(crate) fn damping_for(health: GfmCellHealth, params: GfmParams) -> f32 {
    match health {
        GfmCellHealth::Healthy => 0.0,
        GfmCellHealth::Suspect => params.suspect_damping * 0.0040,
        GfmCellHealth::Quarantined => params.suspect_damping * 0.0110,
        GfmCellHealth::Recovering => params.suspect_damping * 0.0022,
    }
}

pub(crate) fn coupling_scale(health: GfmCellHealth, params: GfmParams) -> f32 {
    match health {
        GfmCellHealth::Healthy => 1.0,
        GfmCellHealth::Suspect => params.suspect_coupling_scale,
        GfmCellHealth::Quarantined => params.suspect_coupling_scale * 0.18,
        GfmCellHealth::Recovering => (params.suspect_coupling_scale + 1.0) * 0.5,
    }
}

pub(crate) fn output_scale(health: GfmCellHealth, params: GfmParams) -> f32 {
    match health {
        GfmCellHealth::Healthy => 1.0,
        GfmCellHealth::Suspect => params.suspect_output_scale,
        GfmCellHealth::Quarantined => params.suspect_output_scale * 0.12,
        GfmCellHealth::Recovering => (params.suspect_output_scale + 1.0) * 0.5,
    }
}

pub(crate) fn field_decay(health: GfmCellHealth, params: GfmParams) -> f32 {
    let health_decay = match health {
        GfmCellHealth::Healthy => 1.0,
        GfmCellHealth::Suspect => 0.90,
        GfmCellHealth::Quarantined => 0.42,
        GfmCellHealth::Recovering => 0.78,
    };
    health_decay * (1.0 - params.heat_dispersion * 0.028).clamp(0.88, 1.0)
}
