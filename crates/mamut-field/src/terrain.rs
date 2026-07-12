use super::*;

pub const GFM_TERRAIN_RECENT_STRIKES: usize = 8;

/// One recorded strike position, stamped with the lattice frame at which it
/// was injected. Consumers derive marker freshness from the frame delta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GfmStrikeMarker {
    pub x: u8,
    pub y: u8,
    pub frame_index: u64,
}

/// Bounded, quantized, read-only terrain snapshot of the GFM lattice,
/// produced on demand — never from the per-sample path. All fields are
/// fixed-size arrays; taking a snapshot allocates nothing. `u8` cell
/// coordinates mirror the strike-coordinate constraint (axes up to 256).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GfmTerrainSnapshot<const W: usize, const H: usize> {
    pub frame_index: u64,
    /// Cell energy quantized to `0..=255` across `0..=2.8` (the cell ceiling).
    pub energy: [[u8; W]; H],
    /// Cell heat quantized to `0..=255` across `0..=2.2` (the cell ceiling).
    pub heat: [[u8; W]; H],
    /// Cell fracture quantized to `0..=255` across `0..=1`.
    pub fracture: [[u8; W]; H],
    pub health: [[GfmCellHealth; W]; H],
    /// Cells whose rupture inhibition window is still open — the
    /// "last rupture" markers for the field view.
    pub recently_ruptured: [[bool; W]; H],
    pub probe: (u8, u8),
    pub stereo_probes: ((u8, u8), (u8, u8)),
    /// Most-recent-first strike markers; `None` slots are unused history.
    pub recent_strikes: [Option<GfmStrikeMarker>; GFM_TERRAIN_RECENT_STRIKES],
}

pub type GfmTerrainSnapshot16 = GfmTerrainSnapshot<GFM_V1_WIDTH, GFM_V1_HEIGHT>;

impl<const W: usize, const H: usize> GfmTerrainSnapshot<W, H> {
    pub(crate) fn empty() -> Self {
        Self {
            frame_index: 0,
            energy: [[0; W]; H],
            heat: [[0; W]; H],
            fracture: [[0; W]; H],
            health: [[GfmCellHealth::Healthy; W]; H],
            recently_ruptured: [[false; W]; H],
            probe: (0, 0),
            stereo_probes: ((0, 0), (0, 0)),
            recent_strikes: [None; GFM_TERRAIN_RECENT_STRIKES],
        }
    }

    pub fn health_histogram(&self) -> GfmHealthHistogram {
        let mut histogram = GfmHealthHistogram::default();
        for row in &self.health {
            for health in row {
                match health {
                    GfmCellHealth::Healthy => histogram.healthy += 1,
                    GfmCellHealth::Suspect => histogram.suspect += 1,
                    GfmCellHealth::Quarantined => histogram.quarantined += 1,
                    GfmCellHealth::Recovering => histogram.recovering += 1,
                }
            }
        }
        histogram
    }
}

pub(crate) fn quantize_unit(value: f32) -> u8 {
    (sanitize_f32(value, 0.0).clamp(0.0, 1.0) * 255.0).round() as u8
}
