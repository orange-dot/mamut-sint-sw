use super::*;

pub const ENGINE_RENDER_BLOCK_FRAMES: usize = 256;
pub const AUDIO_QUEUE_CAPACITY_BLOCKS: usize = 4;
pub const AUDIO_QUEUE_TARGET_BLOCKS: usize = 2;
pub const AUDIO_QUEUE_CAPACITY_FRAMES: usize =
    ENGINE_RENDER_BLOCK_FRAMES * AUDIO_QUEUE_CAPACITY_BLOCKS;
pub const AUDIO_QUEUE_TARGET_FRAMES: usize = ENGINE_RENDER_BLOCK_FRAMES * AUDIO_QUEUE_TARGET_BLOCKS;
pub const SCOPE_QUEUE_CAPACITY_FRAMES: usize = ENGINE_RENDER_BLOCK_FRAMES * 16;
pub const ENGINE_IDLE_SLEEP: Duration = Duration::from_millis(1);
pub const ALSA_WAIT_TIMEOUT_MS: u32 = 100;
pub const ALSA_PLAYBACK_CHANNELS: usize = 2;
pub const ALSA_PLAYBACK_SAMPLE_RATE_HZ: u32 = 96_000;
pub const ALSA_PLAYBACK_SAMPLE_RATE_HZ_ALLOWED: [u32; 6] =
    [44_100, 48_000, 88_200, 96_000, 176_400, 192_000];
pub const ALSA_PERIOD_FRAMES_DEFAULT: usize = 256;
pub const ALSA_BUFFER_FRAMES_DEFAULT: usize = 1_024;
pub const ALSA_START_THRESHOLD_FRAMES_DEFAULT: usize = ALSA_BUFFER_FRAMES_DEFAULT;
pub const PERFORMANCE_UI_REFRESH: Duration = Duration::from_millis(75);
pub const MIDI_ACTIVITY_FLASH: Duration = Duration::from_millis(700);
pub const MIDI_STARTUP_GUARD: Duration = MIDI_ACTIVITY_FLASH;
pub const MIDI_INPUT_QUEUE_CAPACITY: usize = 512;
pub const MIDI_TRACE_QUEUE_CAPACITY: usize = 4096;
pub const MIDI_TRACE_RAW_BYTES: usize = 4;
pub const RUNTIME_CONTROL_QUEUE_CAPACITY: usize = 64;
pub const RECORDING_QUEUE_CAPACITY_FRAMES: usize = 192_000 * 4;
pub const RECORDING_REPLY_TIMEOUT: Duration = Duration::from_millis(500);
pub const DEFAULT_LIVE_TAKE_SECONDS: u64 = 30;
pub const DEFAULT_GFM_UI_SEED: u64 = DEFAULT_GFM_LAYER_SEED;
pub const DEFAULT_LIVE_TAKE_TAG: &str = "gravitacija";
pub const PC4_KNOB_START_ANGLE: f32 = 2.0 * std::f32::consts::PI / 3.0;
pub const PC4_KNOB_SWEEP_ANGLE: f32 = 5.0 * std::f32::consts::PI / 3.0;
pub const CAPTURE_DIR_ENV: &str = "MAMUT_CAPTURE_DIR";
pub const LIVE_SET_STEMS: [&str; 8] = [
    "molten-horizon",
    "cathedral-bloom",
    "ember-vault",
    "razor-thaw",
    "gravity-wake",
    "furnace-choir",
    "granite-plain",
    "glass-tide",
];

pub type StereoFrame = [f32; 2];
