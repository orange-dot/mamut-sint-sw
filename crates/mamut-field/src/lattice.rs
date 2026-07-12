use super::*;

pub(crate) const GFM_CELL_ENERGY_CEILING: f32 = 2.8;
pub(crate) const GFM_CELL_STRAIN_CEILING: f32 = 2.8;
pub(crate) const GFM_CELL_HEAT_CEILING: f32 = 2.2;

pub const GFM_STEREO_PROBE_OFFSET_COLUMNS: usize = 2;

const GFM_STRIKE_WINDOW_CELLS: isize = 3;
const GFM_STRIKE_RADIUS: f32 = 3.0;
const GFM_STRIKE_ENERGY_DEPOSIT: f32 = 1.10;
const GFM_STRIKE_HEAT_DEPOSIT: f32 = 0.45;
const GFM_STRIKE_STRAIN_DEPOSIT: f32 = 0.14;

#[derive(Debug, Clone)]
pub struct GfmLattice<const W: usize, const H: usize> {
    params: GfmParams,
    seed: u64,
    rng_state: u64,
    cells: [[GfmCell; W]; H],
    topology: [[GfmCellTopology; W]; H],
    scratch: [[GfmCellScratch; W]; H],
    topology_key: GfmTopologyKey,
    topology_dirty: bool,
    frame_index: u64,
    last_output: f32,
    peak_abs_output: f32,
    max_energy: f32,
    max_strain: f32,
    last_rupture_count: usize,
    max_rupture_count: usize,
    suspect_damping_events: u64,
    strike_count: u64,
    recent_strikes: [Option<GfmStrikeMarker>; GFM_TERRAIN_RECENT_STRIKES],
    recent_strike_cursor: usize,
}

impl<const W: usize, const H: usize> GfmLattice<W, H> {
    pub fn new(seed: u64, params: GfmParams) -> Self {
        assert!(W > 0, "GFM lattice width must be positive");
        assert!(H > 0, "GFM lattice height must be positive");

        let params = params.sanitized();
        let topology_key = params.topology_key();
        let mut lattice = Self {
            params,
            seed,
            rng_state: seed_to_state(seed),
            cells: [[GfmCell::silent(); W]; H],
            topology: [[GfmCellTopology::empty(); W]; H],
            scratch: [[GfmCellScratch::empty(); W]; H],
            topology_key,
            topology_dirty: true,
            frame_index: 0,
            last_output: 0.0,
            peak_abs_output: 0.0,
            max_energy: 0.0,
            max_strain: 0.0,
            last_rupture_count: 0,
            max_rupture_count: 0,
            suspect_damping_events: 0,
            strike_count: 0,
            recent_strikes: [None; GFM_TERRAIN_RECENT_STRIKES],
            recent_strike_cursor: 0,
        };
        lattice.reset(seed);
        lattice
    }

    pub fn reset(&mut self, seed: u64) {
        self.seed = seed;
        self.rng_state = seed_to_state(seed);
        self.frame_index = 0;
        self.last_output = 0.0;
        self.peak_abs_output = 0.0;
        self.max_energy = 0.0;
        self.max_strain = 0.0;
        self.last_rupture_count = 0;
        self.max_rupture_count = 0;
        self.suspect_damping_events = 0;
        self.strike_count = 0;
        self.recent_strikes = [None; GFM_TERRAIN_RECENT_STRIKES];
        self.recent_strike_cursor = 0;

        let mut rng = self.rng_state;
        for y in 0..H {
            for x in 0..W {
                self.cells[y][x] = seeded_cell::<W, H>(x, y, self.params, &mut rng);
                self.scratch[y][x] = GfmCellScratch::empty();
            }
        }
        self.rng_state = rng;
        self.topology_dirty = true;
        self.refresh_topology_if_needed();
    }

    pub fn set_params(&mut self, params: GfmParams) {
        let params = params.sanitized();
        let topology_key = params.topology_key();
        if topology_key != self.topology_key {
            self.topology_key = topology_key;
            self.topology_dirty = true;
        }
        self.params = params;
    }

    pub fn params(&self) -> GfmParams {
        self.params
    }

    pub fn probe_position(&self) -> (usize, usize) {
        (W / 2, H / 2)
    }

    /// Left/right stereo probe tap centers: the mono center cross offset by
    /// `GFM_STEREO_PROBE_OFFSET_COLUMNS` columns to each side (toroidal).
    pub fn stereo_probe_positions(&self) -> ((usize, usize), (usize, usize)) {
        let center_x = W / 2;
        let center_y = H / 2;
        let offset = GFM_STEREO_PROBE_OFFSET_COLUMNS as isize;
        (
            (wrap_index(center_x as isize - offset, W), center_y),
            (wrap_index(center_x as isize + offset, W), center_y),
        )
    }

    pub const fn seed(&self) -> u64 {
        self.seed
    }

    pub fn health_histogram(&self) -> GfmHealthHistogram {
        let mut histogram = GfmHealthHistogram::default();
        for y in 0..H {
            for x in 0..W {
                match self.cells[y][x].health {
                    GfmCellHealth::Healthy => histogram.healthy += 1,
                    GfmCellHealth::Suspect => histogram.suspect += 1,
                    GfmCellHealth::Quarantined => histogram.quarantined += 1,
                    GfmCellHealth::Recovering => histogram.recovering += 1,
                }
            }
        }
        histogram
    }

    pub fn diagnostics(&self) -> GfmDiagnostics {
        GfmDiagnostics {
            frame_index: self.frame_index,
            last_output: self.last_output,
            peak_abs_output: self.peak_abs_output,
            max_energy: self.max_energy,
            max_strain: self.max_strain,
            last_rupture_count: self.last_rupture_count,
            max_rupture_count: self.max_rupture_count,
            suspect_damping_events: self.suspect_damping_events,
            strike_count: self.strike_count,
            health: self.health_histogram(),
        }
    }

    /// Deposit one strike into the cell fields through a small bounded
    /// footprint. Event-boundary work: no allocation, no RNG draw, fixed
    /// iteration bound, clamped to the same per-cell ceilings the update
    /// loop enforces.
    pub fn inject_strike(&mut self, strike: GfmStrike) {
        debug_assert!(
            W <= 256 && H <= 256,
            "GfmStrike u8 coordinates cannot address a lattice axis past 256"
        );
        let strike = strike.sanitized();
        if strike.is_silent() {
            return;
        }

        let center_x = (strike.x as usize) % W;
        let center_y = (strike.y as usize) % H;
        for dy in -GFM_STRIKE_WINDOW_CELLS..=GFM_STRIKE_WINDOW_CELLS {
            for dx in -GFM_STRIKE_WINDOW_CELLS..=GFM_STRIKE_WINDOW_CELLS {
                let distance = ((dx * dx + dy * dy) as f32).sqrt();
                let normalized = (1.0 - distance / GFM_STRIKE_RADIUS).clamp(0.0, 1.0);
                let footprint = normalized * normalized;
                if footprint <= 0.0 {
                    continue;
                }
                let x = wrap_index(center_x as isize + dx, W);
                let y = wrap_index(center_y as isize + dy, H);
                let cell = &mut self.cells[y][x];
                cell.energy = (cell.energy
                    + strike.pressure * footprint * GFM_STRIKE_ENERGY_DEPOSIT)
                    .clamp(0.0, GFM_CELL_ENERGY_CEILING);
                cell.heat = (cell.heat + strike.heat * footprint * GFM_STRIKE_HEAT_DEPOSIT)
                    .clamp(0.0, GFM_CELL_HEAT_CEILING);
                cell.strain = (cell.strain
                    + strike.rupture_bias * footprint * GFM_STRIKE_STRAIN_DEPOSIT)
                    .clamp(0.0, GFM_CELL_STRAIN_CEILING);
            }
        }
        self.strike_count = self.strike_count.wrapping_add(1);
        self.recent_strikes[self.recent_strike_cursor] = Some(GfmStrikeMarker {
            x: center_x as u8,
            y: center_y as u8,
            frame_index: self.frame_index,
        });
        self.recent_strike_cursor = (self.recent_strike_cursor + 1) % GFM_TERRAIN_RECENT_STRIKES;
    }

    /// Quantized read-only terrain snapshot for inspection surfaces.
    /// On-demand work (one bounded pass over the cells, no allocation, no
    /// RNG draw, no state change) — never called from the per-sample path.
    pub fn terrain_snapshot(&self) -> GfmTerrainSnapshot<W, H> {
        let mut snapshot = GfmTerrainSnapshot::empty();
        snapshot.frame_index = self.frame_index;

        for y in 0..H {
            for x in 0..W {
                let cell = self.cells[y][x];
                snapshot.energy[y][x] = quantize_unit(cell.energy / GFM_CELL_ENERGY_CEILING);
                snapshot.heat[y][x] = quantize_unit(cell.heat / GFM_CELL_HEAT_CEILING);
                snapshot.fracture[y][x] = quantize_unit(cell.fracture);
                snapshot.health[y][x] = cell.health;
                snapshot.recently_ruptured[y][x] = cell.rupture_cooldown > 0;
            }
        }

        let probe = self.probe_position();
        let (left_probe, right_probe) = self.stereo_probe_positions();
        snapshot.probe = (probe.0 as u8, probe.1 as u8);
        snapshot.stereo_probes = (
            (left_probe.0 as u8, left_probe.1 as u8),
            (right_probe.0 as u8, right_probe.1 as u8),
        );
        for index in 0..GFM_TERRAIN_RECENT_STRIKES {
            let slot = (self.recent_strike_cursor + GFM_TERRAIN_RECENT_STRIKES - 1 - index)
                % GFM_TERRAIN_RECENT_STRIKES;
            snapshot.recent_strikes[index] = self.recent_strikes[slot];
        }

        snapshot
    }

    pub fn next_sample(&mut self) -> f32 {
        self.next_sample_with_excitation(GfmExcitation::none())
    }

    pub fn next_sample_stereo(&mut self) -> (f32, f32) {
        self.next_sample_stereo_with_excitation(GfmExcitation::none())
    }

    /// One lattice step read through the two offset stereo tap sets. The
    /// field update is identical to the mono path (probe reads are read-only
    /// passes); only the readout differs. Diagnostics record the mono
    /// fold-down as `last_output` and the louder channel as the peak.
    pub fn next_sample_stereo_with_excitation(&mut self, excitation: GfmExcitation) -> (f32, f32) {
        let params = self.params.sanitized();
        let excitation = excitation.sanitized();
        self.refresh_topology_if_needed();
        self.sample_neighbors(params, excitation);
        self.update_fields_health_and_phase(params, excitation);
        let (left_position, right_position) = self.stereo_probe_positions();
        let left =
            sanitize_sample(self.read_probe_output_at(params, left_position.0, left_position.1));
        let right =
            sanitize_sample(self.read_probe_output_at(params, right_position.0, right_position.1));

        self.last_output = (left + right) * 0.5;
        self.peak_abs_output = self.peak_abs_output.max(left.abs().max(right.abs()));
        self.frame_index = self.frame_index.wrapping_add(1);

        (left, right)
    }

    pub fn next_sample_with_excitation(&mut self, excitation: GfmExcitation) -> f32 {
        // GFM authoritative 10-step order:
        // 1. Read direct numeric parameters and excitation for this audio frame.
        // 2. Refresh topology only when topology inputs changed.
        // 3. Sample each cell's K=7 neighbor fields with decay.
        // 4. Compute gradients from neighbor aggregate to local field.
        // 5. Update energy, strain, heat, coherence, and headroom placeholder.
        // 6. Evaluate rupture thresholds and local inhibition.
        // 7. Update cell health and apply damping/quarantine rules.
        // 8. Integrate phase.
        // 9. Read the fixed center probe output.
        // 10. Sanitize output and update diagnostics.
        let params = self.params.sanitized();
        let excitation = excitation.sanitized();
        self.refresh_topology_if_needed();
        self.sample_neighbors(params, excitation);
        self.update_fields_health_and_phase(params, excitation);
        let output = sanitize_sample(self.read_probe_output(params));

        self.last_output = output;
        self.peak_abs_output = self.peak_abs_output.max(output.abs());
        self.frame_index = self.frame_index.wrapping_add(1);

        output
    }

    fn refresh_topology_if_needed(&mut self) {
        if !self.topology_dirty {
            return;
        }

        for y in 0..H {
            for x in 0..W {
                self.topology[y][x] = build_topology::<W, H>(
                    x,
                    y,
                    self.seed,
                    self.params.posture,
                    self.params.spatial_spread,
                );
            }
        }
        self.topology_dirty = false;
    }

    fn sample_neighbors(&mut self, params: GfmParams, excitation: GfmExcitation) {
        let center_x = W / 2;
        let center_y = H / 2;

        for y in 0..H {
            for x in 0..W {
                let cell = self.cells[y][x];
                let topology = self.topology[y][x];
                let mut total_weight = 0.0;
                let mut energy = 0.0;
                let mut strain = 0.0;
                let mut heat = 0.0;
                let mut coherence = 0.0;
                let mut phase_x = 0.0;
                let mut phase_y = 0.0;
                let mut rupture_votes = 0;

                for neighbor in topology.neighbors {
                    let neighbor_cell = self.cells[neighbor.y as usize][neighbor.x as usize];
                    let health_scale = coupling_scale(neighbor_cell.health, params);
                    let decay = field_decay(neighbor_cell.health, params);
                    let weight = neighbor.weight * health_scale * decay;
                    total_weight += weight;
                    energy += neighbor_cell.energy * weight;
                    strain += neighbor_cell.strain * weight;
                    heat += neighbor_cell.heat * weight;
                    coherence += neighbor_cell.coherence * weight;
                    phase_x += fast_cos01(neighbor_cell.phase) * weight;
                    phase_y += fast_sin01(neighbor_cell.phase) * weight;

                    let neighbor_excitation = excitation_footprint::<W, H>(
                        neighbor.x as usize,
                        neighbor.y as usize,
                        center_x,
                        center_y,
                        params,
                    );
                    let neighbor_rupture_pressure = neighbor_cell.strain
                        + neighbor_cell.fracture * 0.72
                        + neighbor_cell.energy * params.ruin * 0.20;
                    let threshold = params.rupture_threshold
                        - excitation.rupture_bias * neighbor_excitation * 0.18;
                    if neighbor_rupture_pressure > threshold {
                        rupture_votes += 1;
                    }
                }

                if total_weight > 0.0 {
                    let inv_weight = 1.0 / total_weight;
                    energy *= inv_weight;
                    strain *= inv_weight;
                    heat *= inv_weight;
                    coherence *= inv_weight;
                    phase_x *= inv_weight;
                    phase_y *= inv_weight;
                }

                let phase_pull = if phase_x.abs() + phase_y.abs() > 0.000_001 {
                    let self_sin = fast_sin01(cell.phase);
                    let self_cos = fast_cos01(cell.phase);
                    ((phase_y * self_cos - phase_x * self_sin) * 0.5).clamp(-0.5, 0.5)
                } else {
                    0.0
                };
                let local_excitation =
                    excitation_footprint::<W, H>(x, y, center_x, center_y, params);
                let self_rupture_pressure = cell.strain
                    + cell.fracture * 0.35
                    + cell.energy * params.ruin * 0.24
                    + params.ruin * 0.10
                    + excitation.pressure * local_excitation * params.ruin * 0.20;
                let self_threshold =
                    params.rupture_threshold - excitation.rupture_bias * local_excitation * 0.28;
                let self_ready = self_rupture_pressure > self_threshold;
                let inhibition_open = cell.rupture_cooldown == 0;

                self.scratch[y][x] = GfmCellScratch {
                    energy_gradient: energy - cell.energy,
                    strain_gradient: strain - cell.strain,
                    heat_gradient: heat - cell.heat,
                    coherence_gradient: coherence - cell.coherence,
                    phase_pull,
                    rupture_votes,
                    rupture_allowed: params.posture == GfmPosture::Baklja
                        && self_ready
                        && rupture_votes >= params.rupture_quorum
                        && inhibition_open,
                };
            }
        }
    }

    fn update_fields_health_and_phase(&mut self, params: GfmParams, excitation: GfmExcitation) {
        let mut rng = self.rng_state;
        let mut rupture_count = 0;
        let center_x = W / 2;
        let center_y = H / 2;

        for y in 0..H {
            for x in 0..W {
                let mut cell = self.cells[y][x];
                let scratch = self.scratch[y][x];
                let center_force = center_excitation::<W, H>(x, y, center_x, center_y, params);
                let excitation_force =
                    excitation_footprint::<W, H>(x, y, center_x, center_y, params);
                let pressure_drive = excitation.pressure * excitation_force;
                let heat_drive = excitation.heat * excitation_force;
                let thermal = next_bipolar(&mut rng) * params.thermal_noise;
                let health_damping = damping_for(cell.health, params);
                let coupling = params.grav_coupling * coupling_scale(cell.health, params);
                let posture_heat = match params.posture {
                    GfmPosture::Skeleton => 0.0,
                    GfmPosture::Horizont => 0.000_012,
                    GfmPosture::Pec => 0.000_28,
                    GfmPosture::Baklja => 0.000_04,
                };

                cell.energy += scratch.energy_gradient * (0.014 + params.heat_dispersion * 0.036);
                cell.energy += center_force;
                cell.energy += pressure_drive * (0.0012 + params.grav_coupling * 0.000_34);
                cell.energy += cell.fracture * params.rupture_response * 0.012;
                cell.energy += thermal.abs() * (0.000_018 + params.heat_dispersion * 0.000_052);
                cell.energy *=
                    1.0 - (0.000_50 + params.heat_dispersion * 0.000_52 + health_damping);

                let phase_stress = scratch.phase_pull.abs() * (0.34 + coupling * 0.42);
                cell.strain += phase_stress * 0.0048;
                cell.strain += scratch.strain_gradient * (0.012 + params.heat_dispersion * 0.020);
                cell.strain += scratch.rupture_votes as f32 * params.ruin * 0.000_08;
                cell.strain += pressure_drive * params.ruin * 0.0016;
                cell.strain += cell.energy * params.ruin * 0.000_44;
                cell.strain *=
                    1.0 - (0.000_80 + params.heat_dispersion * 0.001_2 + health_damping * 0.58);

                cell.heat += scratch.heat_gradient * (0.010 + params.heat_dispersion * 0.022);
                cell.heat += cell.energy * params.heat_dispersion * 0.000_38 + posture_heat;
                cell.heat += heat_drive * (0.0018 + params.heat_dispersion * 0.0014);
                cell.heat += thermal.abs() * 0.000_18;
                cell.heat *= 1.0 - (0.000_62 + health_damping * 0.24);

                cell.coherence += scratch.coherence_gradient * 0.020;
                cell.coherence += (1.0 - phase_stress * 1.55 - cell.coherence) * 0.0038;
                cell.coherence -= cell.fracture * 0.0016;

                if scratch.rupture_allowed {
                    rupture_count += 1;
                    cell.fracture =
                        (cell.fracture + 0.48 + params.rupture_response * 0.48).min(1.0);
                    cell.energy += params.rupture_response * 0.120;
                    cell.strain *= 0.58;
                    cell.rupture_cooldown = (10.0 + params.ruin * 24.0).round() as u16;
                } else {
                    cell.fracture *= 1.0 - (0.0018 + params.heat_dispersion * 0.0016);
                }

                if cell.rupture_cooldown > 0 {
                    cell.rupture_cooldown -= 1;
                }

                cell.energy = sanitize_f32(cell.energy, 0.0).clamp(0.0, GFM_CELL_ENERGY_CEILING);
                cell.strain = sanitize_f32(cell.strain, 0.0).clamp(0.0, GFM_CELL_STRAIN_CEILING);
                cell.heat = sanitize_f32(cell.heat, 0.0).clamp(0.0, GFM_CELL_HEAT_CEILING);
                cell.coherence = sanitize_f32(cell.coherence, 0.0).clamp(0.0, 1.0);
                cell.fracture = sanitize_f32(cell.fracture, 0.0).clamp(0.0, 1.0);

                update_health(&mut cell, params);
                if cell.health != GfmCellHealth::Healthy {
                    self.suspect_damping_events = self.suspect_damping_events.wrapping_add(1);
                }

                let posture_tilt = posture_velocity_tilt::<W, H>(x, y, params);
                let target_velocity = (cell.base_velocity
                    * posture_tilt
                    * (1.0 + cell.detune * params.omega_dispersion))
                    .clamp(0.000_08, 0.45);
                cell.velocity += (target_velocity - cell.velocity)
                    * (0.000_85 + params.heat_dispersion * 0.000_55);
                cell.velocity += scratch.phase_pull * coupling * 0.000_34;
                cell.velocity += thermal * 0.000_020;
                cell.velocity += pressure_drive * 0.000_020;
                cell.velocity += cell.fracture * params.rupture_response * 0.000_46;
                cell.velocity = sanitize_f32(cell.velocity, target_velocity).clamp(0.000_08, 0.45);
                cell.phase = wrap_phase(cell.phase + cell.velocity);

                self.max_energy = self.max_energy.max(cell.energy);
                self.max_strain = self.max_strain.max(cell.strain);
                self.cells[y][x] = cell;
            }
        }

        self.rng_state = rng;
        self.last_rupture_count = rupture_count;
        self.max_rupture_count = self.max_rupture_count.max(rupture_count);
    }

    fn read_probe_output(&self, params: GfmParams) -> f32 {
        self.read_probe_output_at(params, W / 2, H / 2)
    }

    fn read_probe_output_at(&self, params: GfmParams, center_x: usize, center_y: usize) -> f32 {
        let taps = [
            (center_x, center_y, 1.00),
            ((center_x + W - 1) % W, center_y, 0.34),
            ((center_x + 1) % W, center_y, 0.34),
            (center_x, (center_y + H - 1) % H, 0.34),
            (center_x, (center_y + 1) % H, 0.34),
        ];
        let mut output = 0.0;
        let mut weight_sum = 0.0;

        for (x, y, weight) in taps {
            let cell = self.cells[y][x];
            let output_scale = output_scale(cell.health, params);
            let phase = cell.phase;
            let field_amp =
                (cell.energy * (0.62 + cell.coherence * 0.54) + cell.fracture * 0.16).min(1.8);
            let harmonic = fast_sin01(phase * 2.0 + cell.heat * 0.27) * cell.strain.min(1.0) * 0.18;
            let posture_component = match params.posture {
                GfmPosture::Skeleton | GfmPosture::Horizont => 0.0,
                GfmPosture::Pec => {
                    let bell = fast_sin01(phase * (3.0 + cell.heat * 0.42) + cell.detune * 0.13);
                    let shimmer = fast_sin01(phase * 7.0 + cell.heat * 0.71) * 0.35;
                    (bell + shimmer) * cell.heat.min(1.0) * 0.20
                }
                GfmPosture::Baklja => {
                    let edge_gate = (cell.fracture + cell.strain * params.ruin).clamp(0.0, 1.0);
                    let bright_edge = fast_sin01(phase * 5.0 + cell.fracture * 0.37);
                    let hard_edge = if fast_sin01(phase + cell.fracture * 0.19) > 0.64 {
                        1.0
                    } else {
                        -1.0
                    };
                    (bright_edge * 0.26 + hard_edge * 0.11) * edge_gate
                }
            };
            output += (fast_sin01(phase) * field_amp + harmonic + posture_component)
                * weight
                * output_scale;
            weight_sum += weight;
        }

        let normalized = if weight_sum > 0.0 {
            output / weight_sum
        } else {
            0.0
        };

        soft_limit(normalized * params.output_gain)
    }
}
