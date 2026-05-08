#![allow(clippy::expect_used, clippy::panic)]

use super::*;
use mamut_dsp::{DENORMAL_FLUSH_ABS, MASTER_SAFETY_CEILING};
use mamut_field::{
    GFM_PERFORMANCE_BASELINE_SEED, GFM_PERFORMANCE_DURATION_SECONDS, GFM_V1_HEIGHT, GFM_V1_WIDTH,
    GfmHealthHistogram, GfmLattice16, GfmPerformanceGesture, GfmPerformanceProgram, GfmProgramId,
    sample_to_pcm16,
};
use mamut_patch::{
    CrossMixMode, NoiseColor, Osc2PitchMode, OscPhaseMode, SpectralTable, load_patch_toml,
};

const MOLTEN_HORIZON: &str = include_str!("../../../../patches/factory/molten-horizon.toml");
const CATHEDRAL_BLOOM: &str = include_str!("../../../../patches/factory/cathedral-bloom.toml");
const EMBER_VAULT: &str = include_str!("../../../../patches/factory/ember-vault.toml");
const FURNACE_CHOIR: &str = include_str!("../../../../patches/factory/furnace-choir.toml");
const GLASS_TIDE: &str = include_str!("../../../../patches/factory/glass-tide.toml");
const GRANITE_PLAIN: &str = include_str!("../../../../patches/factory/granite-plain.toml");
const GRAVITY_WAKE: &str = include_str!("../../../../patches/factory/gravity-wake.toml");
const RAZOR_THAW: &str = include_str!("../../../../patches/factory/razor-thaw.toml");
const SAWYER_REZZ: &str = include_str!("../../../../patches/factory/sawyer-rezz.toml");
const GFM_TEST_RATE_HZ: u32 = 1_000;
const ENGINE_LAYER_TEST_RATE_HZ: u32 = 8_000;
const ENGINE_LAYER_RECOVERY_RATE_HZ: u32 = 48_000;
const ENGINE_LAYER_BLOCK_FRAMES: usize = 256;
const FACTORY_PATCHES: [(&str, &str); 9] = [
    ("cathedral-bloom", CATHEDRAL_BLOOM),
    ("ember-vault", EMBER_VAULT),
    ("furnace-choir", FURNACE_CHOIR),
    ("glass-tide", GLASS_TIDE),
    ("granite-plain", GRANITE_PLAIN),
    ("gravity-wake", GRAVITY_WAKE),
    ("molten-horizon", MOLTEN_HORIZON),
    ("razor-thaw", RAZOR_THAW),
    ("sawyer-rezz", SAWYER_REZZ),
];

mod helpers;
use helpers::*;

mod bcs_layer;
mod direct_params;
mod gfm_field_voice;
mod gfm_layer;
mod output_safety;
mod runtime_state;
mod voice_runtime;
