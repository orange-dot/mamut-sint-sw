mod additive;
mod block;
mod envelope;
mod filter;
mod fx;
mod mixing;
mod noise;
mod oscillator;
mod safety;
mod smoother;
mod waveform;
mod wavetable;

pub use additive::{additive_partial_count, additive_ratio, additive_weight};
pub use block::StereoBlockMut;
pub use envelope::{AdsrEnvelope, AdsrTiming, EnvelopeStage};
pub use filter::StateVariableFilter;
pub use fx::{SimpleChorus, SimpleReverb};
pub use mixing::cross_mix_sample;
pub use noise::{NoiseRng, color_noise_sample};
pub use oscillator::Oscillator;
pub use safety::{
    DENORMAL_FLUSH_ABS, DcBlocker, MASTER_SAFETY_CEILING, MASTER_SAFETY_KNEE, StereoDcBlocker,
    db_to_gain, flush_tiny_sample, master_safety_limit, midi_note_hz, mix, sanitize_block,
    sanitize_sample, soft_clip,
};
pub use smoother::LinearSmoother;
pub use waveform::{mixed_wave, mixed_wave_osc2, oscillator_preview_sample, sine_phase_sample};
pub use wavetable::{SPECTRAL_WAVETABLE_COUNT, SPECTRAL_WAVETABLE_SIZE, spectral_wavetable_sample};

#[cfg(test)]
mod tests;
