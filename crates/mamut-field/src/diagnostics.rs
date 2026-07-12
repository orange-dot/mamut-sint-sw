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
    pub strike_count: u64,
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
