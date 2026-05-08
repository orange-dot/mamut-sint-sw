mod additive;
mod block;
mod envelope;
mod filter;
mod fx;
mod math;
mod mixing;
mod modulation;
mod noise;
mod oscillator;
mod phase;
mod safety;
mod smoother;
mod waveform;
mod wavetable;

pub use additive::{additive_partial_count, additive_ratio, additive_weight};
pub use block::{MonoBlockMut, StereoBlockMut};
pub use envelope::{AdsrEnvelope, AdsrTiming, EnvelopeStage};
pub use filter::StateVariableFilter;
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
pub use safety::{
    DENORMAL_FLUSH_ABS, DcBlocker, MASTER_SAFETY_CEILING, MASTER_SAFETY_KNEE, StereoDcBlocker,
    flush_tiny_sample, master_safety_limit, sanitize_block, sanitize_sample,
};
pub use smoother::LinearSmoother;
pub use waveform::{mixed_wave, mixed_wave_osc2, oscillator_preview_sample, sine_phase_sample};
pub use wavetable::{SPECTRAL_WAVETABLE_COUNT, SPECTRAL_WAVETABLE_SIZE, spectral_wavetable_sample};

#[cfg(test)]
mod tests;
