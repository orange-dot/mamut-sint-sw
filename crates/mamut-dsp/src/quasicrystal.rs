use std::f32::consts::TAU;

use crate::safety::flush_tiny_sample;

/// Q32 slope clamp bounds: `sigma` lives in `[0.45, 0.75]`.
pub const MOZAIK_SLOPE_MIN_Q32: u32 = 0x7333_3333;
pub const MOZAIK_SLOPE_MAX_Q32: u32 = 0xC000_0000;

/// Rational slope detents in Q32. These are 2^-32 approximations, so the
/// "periodic" words they generate have astronomically long true periods;
/// musically they are periodic.
pub const MOZAIK_SLOPE_DETENT_HALF_Q32: u32 = 0x8000_0000;
pub const MOZAIK_SLOPE_DETENT_THREE_FIFTHS_Q32: u32 = 0x9999_999A;
pub const MOZAIK_SLOPE_DETENT_FIVE_EIGHTHS_Q32: u32 = 0xA000_0000;
pub const MOZAIK_SLOPE_DETENT_TWO_THIRDS_Q32: u32 = 0xAAAA_AAAB;

/// The golden slope `1/tau = (sqrt(5) - 1) / 2` in Q32: the Fibonacci word.
pub const MOZAIK_SLOPE_GOLDEN_Q32: u32 = 0x9E37_79B9;

/// All shipped slope detents, ascending: `1/2, 3/5, 1/tau, 5/8, 2/3`.
pub const MOZAIK_SLOPE_DETENTS_Q32: [u32; 5] = [
    MOZAIK_SLOPE_DETENT_HALF_Q32,
    MOZAIK_SLOPE_DETENT_THREE_FIFTHS_Q32,
    MOZAIK_SLOPE_GOLDEN_Q32,
    MOZAIK_SLOPE_DETENT_FIVE_EIGHTHS_Q32,
    MOZAIK_SLOPE_DETENT_TWO_THIRDS_Q32,
];

/// Tile-length floor in samples. When `M * d_kind / d_mean` computes below
/// this, the floor holds and the mean tile rate is biased upward; the bias is
/// measured in the evidence doc, not hidden.
pub const MOZAIK_MIN_TILE_SAMPLES: f32 = 4.0;

pub const MOZAIK_MIN_F0_HZ: f32 = 20.0;
pub const MOZAIK_MAX_F0_HZ: f32 = 8000.0;

/// Contrast `gamma = d_L / d_S` clamp bounds and default (`tau`).
pub const MOZAIK_MIN_CONTRAST: f32 = 1.0;
pub const MOZAIK_MAX_CONTRAST: f32 = 2.2;
pub const MOZAIK_DEFAULT_CONTRAST: f32 = 1.618_034;

const MOZAIK_FALLBACK_F0_HZ: f32 = 220.0;
const MOZAIK_MAX_SAMPLE_RATE_HZ: f32 = 1.0e7;
const Q32_ONE: f64 = 4_294_967_296.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MozaikTileKind {
    Long,
    Short,
}

/// The cut-and-project word core: a Bresenham accumulator in Q32.
///
/// The carry of `frac + slope` is the tile kind (carry => `L`), which equals
/// `floor((n+1)*sigma + phi) - floor(n*sigma + phi)` with zero floating-point
/// drift. The phason `phi` is the accumulator's seat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuasicrystalWord {
    frac: u32,
    slope_q32: u32,
}

impl QuasicrystalWord {
    pub fn new(slope_q32: u32, phason_q32: u32) -> Self {
        Self {
            frac: phason_q32,
            slope_q32: clamp_slope_q32(slope_q32),
        }
    }

    pub fn slope_q32(&self) -> u32 {
        self.slope_q32
    }

    pub fn set_slope_q32(&mut self, slope_q32: u32) {
        self.slope_q32 = clamp_slope_q32(slope_q32);
    }

    pub fn frac_q32(&self) -> u32 {
        self.frac
    }

    pub fn shift_phason_q32(&mut self, delta_q32: u32) {
        self.frac = self.frac.wrapping_add(delta_q32);
    }

    pub fn reseat(&mut self, phason_q32: u32) {
        self.frac = phason_q32;
    }

    pub fn next_tile_kind(&mut self) -> MozaikTileKind {
        let (next, carry) = self.frac.overflowing_add(self.slope_q32);
        self.frac = next;
        if carry {
            MozaikTileKind::Long
        } else {
            MozaikTileKind::Short
        }
    }
}

/// Quasicrystal oscillator: the word core driving a train of full-tile Hann
/// pulses (`+` for `L`, `-` for `S`). Aperiodic yet ordered at irrational
/// slopes, harmonic at rational detents; the phason is a constant-pitch
/// modulation axis. Seedless and purely structural: no RNG anywhere.
///
/// Phason changes latch at tile boundaries, so phason motion is click-free by
/// construction. Tile kind and length are computed only at boundaries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuasicrystalOsc {
    word: QuasicrystalWord,
    slope_sigma: f32,
    contrast: f32,
    gain: f32,
    applied_phason_q32: u32,
    pending_phason_q32: Option<u32>,
    tile_pos: f32,
    tile_len: f32,
    tile_sign: f32,
    tiles_emitted: u32,
}

impl QuasicrystalOsc {
    pub fn new() -> Self {
        Self {
            word: QuasicrystalWord::new(MOZAIK_SLOPE_GOLDEN_Q32, 0),
            slope_sigma: slope_q32_to_sigma(MOZAIK_SLOPE_GOLDEN_Q32),
            contrast: MOZAIK_DEFAULT_CONTRAST,
            gain: 1.0,
            applied_phason_q32: 0,
            pending_phason_q32: None,
            tile_pos: 0.0,
            tile_len: 0.0,
            tile_sign: 1.0,
            tiles_emitted: 0,
        }
    }

    pub fn slope_q32(&self) -> u32 {
        self.word.slope_q32()
    }

    pub fn slope_sigma(&self) -> f32 {
        self.slope_sigma
    }

    pub fn contrast(&self) -> f32 {
        self.contrast
    }

    pub fn gain(&self) -> f32 {
        self.gain
    }

    pub fn phason_q32(&self) -> u32 {
        self.applied_phason_q32
    }

    pub fn tiles_emitted(&self) -> u32 {
        self.tiles_emitted
    }

    /// Set the slope from a float `sigma`; non-finite requests fall back to
    /// the golden slope. Takes effect from the next tile onward (the word
    /// consumes the slope only at boundaries).
    pub fn set_slope(&mut self, sigma: f32) {
        if !sigma.is_finite() {
            self.set_slope_q32(MOZAIK_SLOPE_GOLDEN_Q32);
            return;
        }
        let clamped = f64::from(sigma).clamp(0.45, 0.75);
        let q32 = (clamped * Q32_ONE).round() as u64;
        self.set_slope_q32(q32.min(u64::from(u32::MAX)) as u32);
    }

    pub fn set_slope_q32(&mut self, slope_q32: u32) {
        let clamped = clamp_slope_q32(slope_q32);
        // The cached `sigma` only changes when the clamped slope does, so skip
        // the f64 conversion in steady state (the hot path sets the same slope
        // every sample, and identically across voices).
        if clamped != self.word.slope_q32() {
            self.word.set_slope_q32(clamped);
            self.slope_sigma = slope_q32_to_sigma(clamped);
        }
    }

    /// Set the contrast `gamma = d_L / d_S`; non-finite falls back to the
    /// default `tau`. Takes effect from the next tile onward.
    pub fn set_contrast(&mut self, gamma: f32) {
        self.contrast = if gamma.is_finite() {
            gamma.clamp(MOZAIK_MIN_CONTRAST, MOZAIK_MAX_CONTRAST)
        } else {
            MOZAIK_DEFAULT_CONTRAST
        };
    }

    pub fn set_gain(&mut self, gain: f32) {
        self.gain = if gain.is_finite() {
            gain.clamp(0.0, 1.0)
        } else {
            1.0
        };
    }

    /// Request an absolute phason position (free-running wrap); non-finite
    /// falls back to zero. Latched at the next tile boundary.
    pub fn set_phason(&mut self, phason: f32) {
        let phason = if phason.is_finite() {
            f64::from(phason).rem_euclid(1.0)
        } else {
            0.0
        };
        self.set_phason_q32((phason * Q32_ONE) as u64 as u32);
    }

    /// Request an absolute Q32 phason position, latched at the next tile
    /// boundary.
    pub fn set_phason_q32(&mut self, phason_q32: u32) {
        self.pending_phason_q32 = Some(phason_q32);
    }

    /// Shift the requested phason by a Q32 delta (drift), latched at the next
    /// tile boundary. Repeated shifts within one tile accumulate.
    pub fn shift_phason_q32(&mut self, delta_q32: u32) {
        let base = self.pending_phason_q32.unwrap_or(self.applied_phason_q32);
        self.pending_phason_q32 = Some(base.wrapping_add(delta_q32));
    }

    /// Reseat the accumulator to `phason_q32` and discard tile state; the
    /// next sample starts a fresh tile. This is the voice-trigger reset.
    pub fn reset_to_phason_q32(&mut self, phason_q32: u32) {
        self.word.reseat(phason_q32);
        self.applied_phason_q32 = phason_q32;
        self.pending_phason_q32 = None;
        self.tile_pos = 0.0;
        self.tile_len = 0.0;
        self.tile_sign = 1.0;
        self.tiles_emitted = 0;
    }

    pub fn next_sample(&mut self, f0_hz: f32, sample_rate_hz: f32) -> f32 {
        if self.tile_pos >= self.tile_len {
            self.start_tile(f0_hz, sample_rate_hz);
        }
        let unit_pos = self.tile_pos / self.tile_len;
        let hann = 0.5 * (1.0 - (TAU * unit_pos).cos());
        self.tile_pos += 1.0;
        flush_tiny_sample(self.tile_sign * self.gain * hann)
    }

    fn start_tile(&mut self, f0_hz: f32, sample_rate_hz: f32) {
        let leftover = (self.tile_pos - self.tile_len).max(0.0);
        if let Some(target) = self.pending_phason_q32.take() {
            let delta = target.wrapping_sub(self.applied_phason_q32);
            self.word.shift_phason_q32(delta);
            self.applied_phason_q32 = target;
        }
        let kind = self.word.next_tile_kind();
        let f0 = sanitize_f0(f0_hz);
        let sample_rate = sanitize_sample_rate(sample_rate_hz);
        let mean_tile_samples = sample_rate / (2.0 * f0);
        let mean_duration_factor = (1.0 - self.slope_sigma) + self.slope_sigma * self.contrast;
        let (kind_factor, sign) = match kind {
            MozaikTileKind::Long => (self.contrast, 1.0),
            MozaikTileKind::Short => (1.0, -1.0),
        };
        self.tile_len =
            (mean_tile_samples * kind_factor / mean_duration_factor).max(MOZAIK_MIN_TILE_SAMPLES);
        self.tile_pos = leftover;
        self.tile_sign = sign;
        self.tiles_emitted = self.tiles_emitted.wrapping_add(1);
    }
}

impl Default for QuasicrystalOsc {
    fn default() -> Self {
        Self::new()
    }
}

fn clamp_slope_q32(slope_q32: u32) -> u32 {
    slope_q32.clamp(MOZAIK_SLOPE_MIN_Q32, MOZAIK_SLOPE_MAX_Q32)
}

fn slope_q32_to_sigma(slope_q32: u32) -> f32 {
    (f64::from(slope_q32) / Q32_ONE) as f32
}

fn sanitize_f0(f0_hz: f32) -> f32 {
    if f0_hz.is_finite() {
        f0_hz.clamp(MOZAIK_MIN_F0_HZ, MOZAIK_MAX_F0_HZ)
    } else {
        MOZAIK_FALLBACK_F0_HZ
    }
}

fn sanitize_sample_rate(sample_rate_hz: f32) -> f32 {
    if sample_rate_hz.is_finite() {
        sample_rate_hz.clamp(1.0, MOZAIK_MAX_SAMPLE_RATE_HZ)
    } else {
        1.0
    }
}
