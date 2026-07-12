mod additive;
mod bandlimited;
mod block;
mod delay;
mod envelope;
mod filter;
mod fx;
mod math;
mod mixing;
mod modulation;
mod noise;
mod oscillator;
mod phase;
mod quasicrystal;
mod safety;
mod smoother;
mod waveform;
mod waveshaper;
mod wavetable;

pub use additive::{additive_partial_count, additive_ratio, additive_weight};
pub use bandlimited::{
    BandlimitedTriangle, poly_blep, poly_blep_pulse_sample, poly_blep_saw_sample,
};
pub use block::{MonoBlockMut, StereoBlockMut};
pub use delay::{AllpassFilter, CombFilter, DelayLine, OnePoleDamping};
pub use envelope::{AdsrEnvelope, AdsrTiming, EnvelopeStage};
pub use filter::{NonlinearResonantSvf, StateVariableFilter, TptStateVariableFilter, TptSvfOutput};
pub use fx::{SimpleChorus, SimpleReverb};
pub use math::{
    bipolar_to_unipolar, cents_to_ratio, db_to_gain, equal_power_pan, gain_to_db, lerp,
    midi_note_hz, midi_note_to_hz, mix, semitones_to_ratio, smoothstep, soft_clip,
    unipolar_to_bipolar,
};
pub use mixing::cross_mix_sample;
pub use modulation::{
    Lfo, LfoShape, SampleHold, SlewLimiter, beats_to_seconds, hz_to_samples,
    tempo_division_rate_hz, tempo_rate_hz,
};
pub use noise::{NoiseRng, color_noise_sample};
pub use oscillator::Oscillator;
pub use phase::PhaseAccumulator;
pub use quasicrystal::{
    MOZAIK_DEFAULT_CONTRAST, MOZAIK_MAX_CONTRAST, MOZAIK_MAX_F0_HZ, MOZAIK_MIN_CONTRAST,
    MOZAIK_MIN_F0_HZ, MOZAIK_MIN_TILE_SAMPLES, MOZAIK_SLOPE_DETENT_FIVE_EIGHTHS_Q32,
    MOZAIK_SLOPE_DETENT_HALF_Q32, MOZAIK_SLOPE_DETENT_THREE_FIFTHS_Q32,
    MOZAIK_SLOPE_DETENT_TWO_THIRDS_Q32, MOZAIK_SLOPE_DETENTS_Q32, MOZAIK_SLOPE_GOLDEN_Q32,
    MOZAIK_SLOPE_MAX_Q32, MOZAIK_SLOPE_MIN_Q32, MozaikTileKind, QuasicrystalOsc, QuasicrystalWord,
};
pub use safety::{
    DENORMAL_FLUSH_ABS, DcBlocker, MASTER_SAFETY_CEILING, MASTER_SAFETY_KNEE, StereoDcBlocker,
    flush_tiny_sample, master_safety_limit, sanitize_block, sanitize_sample,
};
pub use smoother::LinearSmoother;
pub use waveform::{
    mixed_wave, mixed_wave_bandlimited, mixed_wave_osc2, mixed_wave_osc2_bandlimited,
    oscillator_preview_sample, sine_phase_sample,
};
pub use waveshaper::{
    asymmetric_diode, cubic_soft_clip, cubic_soft_clip_compensated, drive_gain_compensated,
    foldback, tanh_drive, tanh_drive_compensated,
};
pub use wavetable::{SPECTRAL_WAVETABLE_COUNT, SPECTRAL_WAVETABLE_SIZE, spectral_wavetable_sample};

#[cfg(test)]
mod tests;
