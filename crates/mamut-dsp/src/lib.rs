mod block;
mod envelope;
mod filter;
mod fx;
mod oscillator;
mod safety;
mod smoother;

pub use block::StereoBlockMut;
pub use envelope::{AdsrEnvelope, AdsrTiming, EnvelopeStage};
pub use filter::StateVariableFilter;
pub use fx::{SimpleChorus, SimpleReverb};
pub use oscillator::{NoiseRng, Oscillator};
pub use safety::{
    DENORMAL_FLUSH_ABS, DcBlocker, MASTER_SAFETY_CEILING, MASTER_SAFETY_KNEE, StereoDcBlocker,
    db_to_gain, flush_tiny_sample, master_safety_limit, midi_note_hz, mix, sanitize_block,
    sanitize_sample, soft_clip,
};
pub use smoother::LinearSmoother;

#[cfg(test)]
mod tests;
