pub mod bcs;

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

    fn sanitized(self) -> Self {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GfmPosture {
    Skeleton,
    Horizont,
    Pec,
    Baklja,
}

impl Default for GfmPosture {
    fn default() -> Self {
        Self::Skeleton
    }
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

    fn sanitized(self) -> Self {
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

    fn topology_key(self) -> GfmTopologyKey {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GfmCellHealth {
    Healthy,
    Suspect,
    Quarantined,
    Recovering,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GfmHealthHistogram {
    pub healthy: usize,
    pub suspect: usize,
    pub quarantined: usize,
    pub recovering: usize,
}

impl GfmHealthHistogram {
    pub fn total(self) -> usize {
        self.healthy + self.suspect + self.quarantined + self.recovering
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GfmDiagnostics {
    pub frame_index: u64,
    pub last_output: f32,
    pub peak_abs_output: f32,
    pub max_energy: f32,
    pub max_strain: f32,
    pub last_rupture_count: usize,
    pub max_rupture_count: usize,
    pub suspect_damping_events: u64,
    pub health: GfmHealthHistogram,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GfmRenderStats {
    pub frames: usize,
    pub finite: bool,
    pub rms: f32,
    pub peak_abs: f32,
    pub diagnostics: GfmDiagnostics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GfmTopologyKey {
    posture: GfmPosture,
    spatial_bucket: u16,
}

#[derive(Debug, Clone, Copy)]
struct GfmCell {
    phase: f32,
    velocity: f32,
    base_velocity: f32,
    detune: f32,
    energy: f32,
    strain: f32,
    fracture: f32,
    heat: f32,
    coherence: f32,
    health: GfmCellHealth,
    unsafe_run: u16,
    stable_run: u16,
    rupture_cooldown: u16,
}

impl GfmCell {
    const fn silent() -> Self {
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
struct GfmNeighbor {
    x: u16,
    y: u16,
    weight: f32,
}

impl GfmNeighbor {
    const fn empty() -> Self {
        Self {
            x: 0,
            y: 0,
            weight: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct GfmCellTopology {
    neighbors: [GfmNeighbor; GFM_K_NEIGHBORS],
}

impl GfmCellTopology {
    const fn empty() -> Self {
        Self {
            neighbors: [GfmNeighbor::empty(); GFM_K_NEIGHBORS],
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct GfmCellScratch {
    energy_gradient: f32,
    strain_gradient: f32,
    heat_gradient: f32,
    coherence_gradient: f32,
    phase_pull: f32,
    rupture_votes: usize,
    rupture_allowed: bool,
}

impl GfmCellScratch {
    const fn empty() -> Self {
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
            health: self.health_histogram(),
        }
    }

    pub fn next_sample(&mut self) -> f32 {
        self.next_sample_with_excitation(GfmExcitation::none())
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

                cell.energy = sanitize_f32(cell.energy, 0.0).clamp(0.0, 2.8);
                cell.strain = sanitize_f32(cell.strain, 0.0).clamp(0.0, 2.8);
                cell.heat = sanitize_f32(cell.heat, 0.0).clamp(0.0, 2.2);
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
        let center_x = W / 2;
        let center_y = H / 2;
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

fn seeded_cell<const W: usize, const H: usize>(
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

fn build_topology<const W: usize, const H: usize>(
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

fn center_excitation<const W: usize, const H: usize>(
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

fn excitation_footprint<const W: usize, const H: usize>(
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

fn posture_velocity_tilt<const W: usize, const H: usize>(
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

fn update_health(cell: &mut GfmCell, params: GfmParams) {
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

fn damping_for(health: GfmCellHealth, params: GfmParams) -> f32 {
    match health {
        GfmCellHealth::Healthy => 0.0,
        GfmCellHealth::Suspect => params.suspect_damping * 0.0040,
        GfmCellHealth::Quarantined => params.suspect_damping * 0.0110,
        GfmCellHealth::Recovering => params.suspect_damping * 0.0022,
    }
}

fn coupling_scale(health: GfmCellHealth, params: GfmParams) -> f32 {
    match health {
        GfmCellHealth::Healthy => 1.0,
        GfmCellHealth::Suspect => params.suspect_coupling_scale,
        GfmCellHealth::Quarantined => params.suspect_coupling_scale * 0.18,
        GfmCellHealth::Recovering => (params.suspect_coupling_scale + 1.0) * 0.5,
    }
}

fn output_scale(health: GfmCellHealth, params: GfmParams) -> f32 {
    match health {
        GfmCellHealth::Healthy => 1.0,
        GfmCellHealth::Suspect => params.suspect_output_scale,
        GfmCellHealth::Quarantined => params.suspect_output_scale * 0.12,
        GfmCellHealth::Recovering => (params.suspect_output_scale + 1.0) * 0.5,
    }
}

fn field_decay(health: GfmCellHealth, params: GfmParams) -> f32 {
    let health_decay = match health {
        GfmCellHealth::Healthy => 1.0,
        GfmCellHealth::Suspect => 0.90,
        GfmCellHealth::Quarantined => 0.42,
        GfmCellHealth::Recovering => 0.78,
    };
    health_decay * (1.0 - params.heat_dispersion * 0.028).clamp(0.88, 1.0)
}

fn seed_to_state(seed: u64) -> u64 {
    let state = seed ^ 0x9E37_79B9_7F4A_7C15;
    if state == 0 {
        0xA076_1D64_78BD_642F
    } else {
        state
    }
}

fn next_u64(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state ^ (*state >> 33)
}

fn next_unit(state: &mut u64) -> f32 {
    let bits = (next_u64(state) >> 40) as u32;
    bits as f32 / 0x00FF_FFFF_u32 as f32
}

fn next_bipolar(state: &mut u64) -> f32 {
    next_unit(state) * 2.0 - 1.0
}

fn wrap_index(index: isize, len: usize) -> usize {
    let len = len as isize;
    index.rem_euclid(len) as usize
}

fn toroidal_distance(a: usize, b: usize, len: usize) -> usize {
    let direct = a.abs_diff(b);
    direct.min(len - direct)
}

fn wrap_phase(phase: f32) -> f32 {
    phase - phase.floor()
}

fn sanitize_positive(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

fn sanitize_f32(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

fn sanitize_sample(sample: f32) -> f32 {
    if sample.is_finite() {
        sample.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

fn soft_limit(sample: f32) -> f32 {
    (sample * 1.22).tanh().clamp(-1.0, 1.0)
}

fn fast_sin01(phase: f32) -> f32 {
    let phase = wrap_phase(phase);
    let triangle = if phase < 0.5 {
        phase * 4.0 - 1.0
    } else {
        3.0 - phase * 4.0
    };
    triangle * (1.5 - 0.5 * triangle.abs())
}

fn fast_cos01(phase: f32) -> f32 {
    fast_sin01(phase + 0.25)
}

#[cfg(test)]
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
