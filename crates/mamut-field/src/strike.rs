use super::*;

/// One spatial excitation strike. Strikes deposit into cell fields as bounded
/// event-boundary work; coordinates wrap toroidally into the lattice.
/// `u8` coordinates address lattices up to 256 cells per axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GfmStrike {
    pub x: u8,
    pub y: u8,
    pub pressure: f32,
    pub heat: f32,
    pub rupture_bias: f32,
}

impl GfmStrike {
    pub const fn none() -> Self {
        Self {
            x: 0,
            y: 0,
            pressure: 0.0,
            heat: 0.0,
            rupture_bias: 0.0,
        }
    }

    pub(crate) fn sanitized(self) -> Self {
        Self {
            x: self.x,
            y: self.y,
            pressure: sanitize_f32(self.pressure, 0.0).clamp(0.0, 1.0),
            heat: sanitize_f32(self.heat, 0.0).clamp(0.0, 1.0),
            rupture_bias: sanitize_f32(self.rupture_bias, 0.0).clamp(0.0, 1.0),
        }
    }

    pub(crate) fn is_silent(self) -> bool {
        self.pressure <= 0.0 && self.heat <= 0.0 && self.rupture_bias <= 0.0
    }
}

impl Default for GfmStrike {
    fn default() -> Self {
        Self::none()
    }
}

/// Deterministic seeded note-to-cell map: pitch class selects the x band,
/// octave selects the y band, and the lattice seed XOR-folds a bounded
/// per-patch offset on top. Collisions are acceptable; strike energy sums.
pub fn note_strike_position<const W: usize, const H: usize>(note: u8, seed: u64) -> (usize, usize) {
    if W == 0 || H == 0 {
        return (0, 0);
    }
    let note = note.min(127);
    let pitch_class = (note % 12) as usize;
    let octave = (note / 12) as usize;
    let x_base = (pitch_class * W) / 12;
    let y_base = (octave * H) / 11;
    let folded = seed ^ (seed >> 27) ^ (note as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    let dx = ((folded >> 8) % 3) as isize - 1;
    let dy = ((folded >> 16) % 3) as isize - 1;

    (
        wrap_index(x_base as isize + dx, W),
        wrap_index(y_base as isize + dy, H),
    )
}
