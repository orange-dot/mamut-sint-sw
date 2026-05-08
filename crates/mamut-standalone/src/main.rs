use std::{
    env,
    io::{self, IsTerminal},
};
#[cfg(test)]
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, anyhow};
#[cfg(test)]
use crossbeam_queue::ArrayQueue;
use gui::{display_available, run_performance_window};
#[cfg(test)]
use mamut_engine::{
    BcsLayerMode, BcsLayerSnapshot, BcsScenario, ControllerEvent, Engine, EngineConfig,
    EngineSnapshot, GfmLayerMode, NoteEvent, Scheduled,
};
#[cfg(test)]
use mamut_params::{MacroId, ParamId, param_spec};
use mamut_patch::validate_patch_v1;
use mamut_tui::run_tui_session;
#[cfg(test)]
use rtrb::RingBuffer;

mod gui;
#[cfg(test)]
pub(crate) use gui::{
    PerformanceTab, binding_display_value, pc4_control_display, pc4_knob_angle,
    sorted_bindings_for_section, sound_lab_param_value_from_normalized, sound_lab_source_for_cc,
};
#[cfg(test)]
pub(crate) use mamut_runtime::session;
pub(crate) use mamut_runtime::*;

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
            play_standalone(&options)
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

fn play_standalone(options: &PlayOptions) -> Result<()> {
    let mut session = RuntimeSession::new(options)?;
    if options.gui {
        if display_available() {
            run_performance_window(session)
        } else {
            Err(anyhow!(
                "--gui requested but DISPLAY/WAYLAND_DISPLAY is not available"
            ))
        }
    } else if options.headless {
        session.print_startup_summary();
        println!("headless mode enabled; running without interactive controls");
        session.block_forever()
    } else if !options.headless && io::stdin().is_terminal() {
        run_tui_session(session)
    } else {
        session.print_startup_summary();
        println!("stdin is not a terminal; running without interactive controls");
        session.block_forever()
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
  mamut-standalone play [--demo] [--gui] [--headless] --audio-device <alsa-index-or-hw:card,device> [--sample-rate <hz>] [--alsa-period-frames <n>] [--alsa-buffer-frames <n>] [--alsa-start-threshold-frames <n>] [--midi-device <name-or-index>] [--midi-channel <1..16>] [--controller-profile <path>] [--trace-midi] [--gfm-layer-seed <u64-or-0xHEX>] [--bcs-layer-scenario <scenario>] [factory-name-or-path]

play frontend:
  default: terminal TUI when stdin is interactive
  --gui: legacy egui performance window
  --headless: no interactive controls"
    );
}

#[cfg(test)]
mod tests;
