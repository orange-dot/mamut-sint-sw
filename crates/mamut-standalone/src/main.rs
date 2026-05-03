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
use eframe::{NativeOptions, egui};
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

mod audio_runtime;
mod cli;
mod commands;
mod devices;
mod gui;
mod runtime;
mod session;
mod types;

pub(crate) use audio_runtime::*;
pub(crate) use cli::*;
pub(crate) use commands::*;
pub(crate) use devices::*;
pub(crate) use gui::*;
pub(crate) use runtime::*;
pub(crate) use types::*;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        eprintln!();
        print_usage();
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();

    match args.first().map(String::as_str) {
        Some("list-factory") => list_factory_patches(),
        Some("list-audio") => list_audio_devices(),
        Some("list-midi") => list_midi_devices(),
        Some("validate") => {
            let path = resolve_patch_argument(args.get(1).map(String::as_str))?;
            let patch = load_patch_from_path(&path)?;
            validate_patch_v1(&patch).context("patch validation failed")?;
            println!("valid: {}", path.display());
            Ok(())
        }
        Some("dry-run") => {
            let options = parse_dry_run_options(&args[1..])?;
            dry_run(&options)
        }
        Some("play") => {
            let options = parse_play_options(&args[1..])?;
            play(&options)
        }
        Some("help") | Some("--help") | Some("-h") => {
            print_usage();
            Ok(())
        }
        None => dry_run(&DryRunOptions {
            patch_path: default_patch_path(),
            gfm_layer_seed: None,
            bcs_layer_scenario: None,
        }),
        Some(other) => Err(anyhow!("unknown command `{other}`")),
    }
}

fn print_usage() {
    eprintln!(
        "usage:
  mamut-standalone
  mamut-standalone list-factory
  mamut-standalone list-audio
  mamut-standalone list-midi
  mamut-standalone validate [factory-name-or-path]
  mamut-standalone dry-run [--gfm-layer-seed <u64-or-0xHEX>] [--bcs-layer-scenario <scenario>] [factory-name-or-path]
  mamut-standalone play [--demo] [--headless] --audio-device <alsa-index-or-hw:card,device> [--alsa-period-frames <n>] [--alsa-buffer-frames <n>] [--alsa-start-threshold-frames <n>] [--midi-device <name-or-index>] [--midi-channel <1..16>] [--controller-profile <path>] [--trace-midi] [--gfm-layer-seed <u64-or-0xHEX>] [--bcs-layer-scenario <scenario>] [factory-name-or-path]

interactive play controls:
  help
  status
  patches
  favorites
  favorite <0..7>
  patch <factory-name-or-path>
  next
  prev
  demo-patch
  macro <gravitacija|bloom|heat|ruin|swarm> <0..1>
  panic
  reset-controllers
  record <seconds> [path]
  record-stop
  audio [alsa-index-or-hw:card,device]
  midi [name-or-index]
  demo
  quit"
    );
}

#[cfg(test)]
mod tests;
