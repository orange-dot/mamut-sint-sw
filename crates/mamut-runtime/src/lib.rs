use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::{self, BufRead, BufWriter, IsTerminal, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU8, AtomicU64, AtomicUsize, Ordering},
        mpsc::{self, RecvTimeoutError, TryRecvError},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use alsa::{
    Direction, ValueOr,
    pcm::{Access, Format, Frames, HwParams, IO, PCM},
};
use anyhow::{Context, Result, anyhow};
use crossbeam_queue::ArrayQueue;
use mamut_dsp::StereoBlockMut;
use mamut_engine::{
    BcsLayerMode, BcsLayerSnapshot, BcsScenario, ControllerEvent, DEFAULT_GFM_LAYER_SEED, Engine,
    EngineConfig, EngineSnapshot, GfmLayerMode, GfmVoiceProgramSelection, NoteEvent, ProcessBlock,
    Scheduled,
};
use mamut_params::{MacroId, ParamId, ParamUnit, param_by_key, param_spec};
use mamut_patch::{PatchFileV1, load_patch_toml, save_patch_toml, validate_patch_v1};
use midir::{Ignore, MidiInput, MidiInputConnection, MidiInputPort};
use rtrb::{Consumer, Producer, RingBuffer};
use serde::Deserialize;

pub mod audio_runtime;
pub mod cli;
pub mod commands;
pub mod devices;
pub mod midi_trace;
pub mod runtime;
pub mod session;
pub mod types;

pub use audio_runtime::*;
pub use cli::*;
pub use commands::*;
pub use devices::*;
pub use midi_trace::*;
pub use runtime::*;
pub use session::*;
pub use types::*;

pub fn macro_snapshot_value(snapshot: &EngineSnapshot, id: MacroId, effective: bool) -> f32 {
    let macros = if effective {
        snapshot.effective_macros
    } else {
        snapshot.live_macros
    };
    match id {
        MacroId::Gravitacija => macros.gravitacija,
        MacroId::Bloom => macros.bloom,
        MacroId::Heat => macros.heat,
        MacroId::Ruin => macros.ruin,
        MacroId::Swarm => macros.swarm,
    }
}

pub fn direct_param_raw_value(snapshot: &EngineSnapshot, id: ParamId) -> Option<f32> {
    let value = match id {
        ParamId::GravitacijaMacro => snapshot.live_macros.gravitacija,
        ParamId::BloomMacro => snapshot.live_macros.bloom,
        ParamId::HeatMacro => snapshot.live_macros.heat,
        ParamId::RuinMacro => snapshot.live_macros.ruin,
        ParamId::SwarmMacro => snapshot.live_macros.swarm,
        ParamId::Osc1SawLevel => snapshot.direct.osc1_wave_mix[0],
        ParamId::Osc1PulseLevel => snapshot.direct.osc1_wave_mix[1],
        ParamId::Osc1TriangleLevel => snapshot.direct.osc1_wave_mix[2],
        ParamId::Osc1NoiseLevel => snapshot.direct.osc1_wave_mix[3],
        ParamId::Osc1FineTuneCents => snapshot.direct.osc1_fine_tune_cents,
        ParamId::Osc1PulseWidth => snapshot.direct.osc1_pulse_width,
        ParamId::Osc1PwmDepth => snapshot.direct.osc1_pwm_depth,
        ParamId::Osc1PhaseMode => snapshot.direct.osc1_phase_mode,
        ParamId::Osc1StartPhase => snapshot.direct.osc1_start_phase,
        ParamId::Osc1SawBend => snapshot.direct.osc1_saw_bend,
        ParamId::Osc1TriangleFold => snapshot.direct.osc1_triangle_fold,
        ParamId::Osc1PulseEdge => snapshot.direct.osc1_pulse_edge,
        ParamId::Osc1Bandlimit => snapshot.direct.osc1_bandlimit,
        ParamId::Osc2SawLevel => snapshot.direct.osc2_wave_mix[0],
        ParamId::Osc2PulseLevel => snapshot.direct.osc2_wave_mix[1],
        ParamId::Osc2TriangleLevel => snapshot.direct.osc2_wave_mix[2],
        ParamId::Osc2IntervalSemitones => snapshot.direct.osc2_interval_semitones,
        ParamId::Osc2FineTuneCents => snapshot.direct.osc2_fine_tune_cents,
        ParamId::Osc2SyncAmount => snapshot.direct.sync_amount,
        ParamId::Osc2CrossmodAmount => snapshot.direct.crossmod_amount,
        ParamId::Osc2PulseWidth => snapshot.direct.osc2_pulse_width,
        ParamId::Osc2PwmDepth => snapshot.direct.osc2_pwm_depth,
        ParamId::Osc2PhaseMode => snapshot.direct.osc2_phase_mode,
        ParamId::Osc2StartPhase => snapshot.direct.osc2_start_phase,
        ParamId::Osc2Level => snapshot.direct.osc2_level,
        ParamId::Osc2PitchMode => snapshot.direct.osc2_pitch_mode,
        ParamId::Osc2Ratio => snapshot.direct.osc2_ratio,
        ParamId::Osc2SawBend => snapshot.direct.osc2_saw_bend,
        ParamId::Osc2TriangleFold => snapshot.direct.osc2_triangle_fold,
        ParamId::Osc2PulseEdge => snapshot.direct.osc2_pulse_edge,
        ParamId::Osc2Bandlimit => snapshot.direct.osc2_bandlimit,
        ParamId::SpectralLevel => snapshot.direct.spectral_level,
        ParamId::SpectralTable => snapshot.direct.spectral_table,
        ParamId::SpectralPosition => snapshot.direct.spectral_position,
        ParamId::SpectralMorph => snapshot.direct.spectral_morph,
        ParamId::SpectralRatio => snapshot.direct.spectral_ratio,
        ParamId::SpectralFineTuneCents => snapshot.direct.spectral_fine_tune_cents,
        ParamId::AdditiveLevel => snapshot.direct.additive_level,
        ParamId::AdditivePartialCount => snapshot.direct.additive_partial_count,
        ParamId::AdditiveHarmonicSpread => snapshot.direct.additive_harmonic_spread,
        ParamId::AdditiveOddEvenBalance => snapshot.direct.additive_odd_even_balance,
        ParamId::AdditiveInharmonicity => snapshot.direct.additive_inharmonicity,
        ParamId::AdditiveSpectralTilt => snapshot.direct.additive_spectral_tilt,
        ParamId::AdditiveRandomDetuneCents => snapshot.direct.additive_random_detune_cents,
        ParamId::SourcePwmRateHz => snapshot.direct.source_pwm_rate_hz,
        ParamId::NoiseColor => snapshot.direct.noise_color,
        ParamId::NoiseFilterLevel => snapshot.direct.noise_filter_level,
        ParamId::NoiseBodyLevel => snapshot.direct.noise_body_level,
        ParamId::AnalogDrift => snapshot.direct.analog_drift,
        ParamId::MicroJitter => snapshot.direct.micro_jitter,
        ParamId::FmAmount => snapshot.direct.fm_amount,
        ParamId::FmDirection => snapshot.direct.fm_direction,
        ParamId::PhaseModAmount => snapshot.direct.phase_mod_amount,
        ParamId::PhaseModDirection => snapshot.direct.phase_mod_direction,
        ParamId::RingModAmount => snapshot.direct.ring_mod_amount,
        ParamId::AmAmount => snapshot.direct.am_amount,
        ParamId::SyncDirection => snapshot.direct.sync_direction,
        ParamId::SyncSoftness => snapshot.direct.sync_softness,
        ParamId::CrossMixMode => snapshot.direct.cross_mix_mode,
        ParamId::CrossMixAmount => snapshot.direct.cross_mix_amount,
        ParamId::SubLevel => snapshot.direct.sub_level,
        ParamId::SubOctaveOffset => snapshot.direct.sub_octave_offset,
        ParamId::MixerPreFilterDrive => snapshot.direct.mixer_pre_filter_drive,
        ParamId::MixerBodyMix => snapshot.direct.mixer_body_mix,
        ParamId::FilterCutoffHz => snapshot.direct.cutoff_hz,
        ParamId::FilterResonance => snapshot.direct.resonance,
        ParamId::FilterDrive => snapshot.direct.filter_drive,
        ParamId::FilterKeytrack => snapshot.direct.filter_tracking,
        ParamId::FilterModel => snapshot.direct.filter_model,
        ParamId::AmpEnvAttackMs => snapshot.direct.amp_env.attack_ms,
        ParamId::AmpEnvDecayMs => snapshot.direct.amp_env.decay_ms,
        ParamId::AmpEnvSustain => snapshot.direct.amp_env.sustain,
        ParamId::AmpEnvReleaseMs => snapshot.direct.amp_env.release_ms,
        ParamId::FilterEnvAttackMs => snapshot.direct.filter_env.attack_ms,
        ParamId::FilterEnvDecayMs => snapshot.direct.filter_env.decay_ms,
        ParamId::FilterEnvSustain => snapshot.direct.filter_env.sustain,
        ParamId::FilterEnvReleaseMs => snapshot.direct.filter_env.release_ms,
        ParamId::FilterEnvDepth => snapshot.direct.filter_env_depth,
        ParamId::VoiceStereoWidth => snapshot.direct.stereo_width,
        ParamId::VoiceDetuneSpreadCents => snapshot.direct.detune_spread_cents,
        ParamId::VoiceVelocityToLevel => snapshot.direct.voice_velocity_to_level,
        ParamId::VoiceVelocityToFilter => snapshot.direct.voice_velocity_to_filter,
        ParamId::FinalStageBodyDrive => snapshot.direct.body_drive,
        ParamId::FinalStageAsymmetry => snapshot.direct.final_asymmetry,
        ParamId::FinalStageLowMidEmphasis => snapshot.direct.low_mid_emphasis,
        ParamId::FinalStageOutputTrimDb => snapshot.direct.output_trim_db,
        ParamId::ChorusEnabled => {
            if snapshot.direct.chorus_enabled {
                1.0
            } else {
                0.0
            }
        }
        ParamId::ChorusMix => snapshot.direct.chorus_mix,
        ParamId::ChorusDepth => snapshot.direct.chorus_depth,
        ParamId::ChorusRateHz => snapshot.direct.chorus_rate_hz,
        ParamId::ReverbEnabled => {
            if snapshot.direct.reverb_enabled {
                1.0
            } else {
                0.0
            }
        }
        ParamId::ReverbMix => snapshot.direct.reverb_mix,
        ParamId::ReverbSize => snapshot.direct.reverb_size,
        ParamId::ReverbDamping => snapshot.direct.reverb_damping,
        ParamId::PerformanceVelocityToLevel => snapshot.performance_response.velocity_to_level,
        ParamId::PerformanceVelocityToFilter => snapshot.performance_response.velocity_to_filter,
        ParamId::PerformanceAftertouchToGravitacija => {
            snapshot.performance_response.aftertouch_to_gravitacija
        }
        ParamId::PerformanceAftertouchToBaklja => {
            snapshot.performance_response.aftertouch_to_baklja
        }
        ParamId::PerformanceModWheelToBloom => snapshot.performance_response.mod_wheel_to_bloom,
        ParamId::PerformanceModWheelToSwarm => snapshot.performance_response.mod_wheel_to_swarm,
    };
    Some(value)
}

pub fn direct_param_display_value(snapshot: &EngineSnapshot, id: ParamId) -> Option<String> {
    direct_param_raw_value(snapshot, id).map(|value| format_param_value(id, value))
}

pub fn format_param_value(id: ParamId, value: f32) -> String {
    match param_spec(id).unit {
        ParamUnit::Hertz => format!("{value:.1} Hz"),
        ParamUnit::Milliseconds => format!("{value:.1} ms"),
        ParamUnit::Decibels => format!("{value:.1} dB"),
        ParamUnit::Cents => format!("{value:.1} cents"),
        ParamUnit::Semitones => format!("{value:.1} st"),
        ParamUnit::Boolean => {
            if value >= 0.5 {
                "on".to_string()
            } else {
                "off".to_string()
            }
        }
        ParamUnit::Indexed => indexed_param_value_label(id, value),
        ParamUnit::Normalized => format!("{value:.2}"),
    }
}

pub fn indexed_param_value_label(id: ParamId, value: f32) -> String {
    let index = value.round() as i32;
    let label = match id {
        ParamId::Osc1PhaseMode | ParamId::Osc2PhaseMode => match index {
            1 => "fixed",
            2 => "free",
            _ => "reset",
        },
        ParamId::Osc2PitchMode => {
            if index >= 1 {
                "ratio"
            } else {
                "semitone"
            }
        }
        ParamId::NoiseColor => match index {
            1 => "pink",
            2 => "dark",
            3 => "bright",
            _ => "white",
        },
        ParamId::FilterModel => match index {
            1 => "tpt",
            2 => "matter",
            _ => "legacy",
        },
        ParamId::SpectralTable => match index {
            1 => "vocal",
            2 => "metal",
            3 => "hollow",
            4 => "formant",
            _ => "sine",
        },
        ParamId::FmDirection | ParamId::PhaseModDirection | ParamId::SyncDirection => {
            if index >= 1 { "2->1" } else { "1->2" }
        }
        ParamId::CrossMixMode => match index {
            1 => "multiply",
            2 => "fold",
            3 => "max",
            4 => "diff",
            _ => "sum",
        },
        _ => return format!("{index}"),
    };
    label.to_string()
}
