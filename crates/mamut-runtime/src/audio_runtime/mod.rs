use super::*;

mod alsa_loop;
pub use alsa_loop::run_alsa_playback_loop;

mod sample_io;
pub use sample_io::{
    AlsaPlaybackWriter, convert_f32_samples_to_s32, f32_sample_to_s32, recover_alsa_playback_error,
};

mod engine_thread;
pub use engine_thread::{
    ControllerCoalesceKey, EngineThreadState, coalesce_controller_events, controller_coalesce_key,
    spawn_engine_thread,
};

mod recording;
pub use recording::{
    FloatStereoWavWriter, OpenedMidiConnection, OutputRecorder, OutputRecordingWorker,
    drain_queue_into_output, push_frames_into_queue, push_recording_channels_into_queue,
    push_rendered_channels_into_queue, run_output_recording_writer,
    write_channels_into_stereo_frames, write_float_stereo_wav_header, write_frames_into_output,
    write_output_frame, write_u16_le, write_u32_le, zero_fill_partial_output_tail,
    zero_fill_remaining_output,
};

mod midi_parse;
pub use midi_parse::{
    parse_midi_message, parse_profile_cc_binding, sound_lab_direct_param_value,
    sound_lab_midi_overlay_events,
};

mod demo;
pub use demo::{run_demo_performance, sleep_interruptibly};
