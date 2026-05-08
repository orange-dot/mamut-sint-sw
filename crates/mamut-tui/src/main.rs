use std::env;

use anyhow::{Result, anyhow};
use mamut_runtime::*;
use mamut_tui::run_tui_session;

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
        Some("play") => {
            let options = parse_play_options(&args[1..])?;
            if options.gui {
                return Err(anyhow!(
                    "mamut-tui does not support --gui; use mamut-standalone"
                ));
            }
            if options.headless {
                return play(&options);
            }
            run_tui_session(RuntimeSession::new(&options)?)
        }
        Some("list-factory") => list_factory_patches(),
        Some("list-audio") => list_audio_devices(),
        Some("list-midi") => list_midi_devices(),
        Some("validate") => {
            let path = resolve_patch_argument(args.get(1).map(String::as_str))?;
            let patch = load_patch_from_path(&path)?;
            mamut_patch::validate_patch_v1(&patch)?;
            println!("valid: {}", path.display());
            Ok(())
        }
        Some("dry-run") => {
            let options = parse_dry_run_options(&args[1..])?;
            dry_run(&options)
        }
        Some("help") | Some("--help") | Some("-h") => {
            print_usage();
            Ok(())
        }
        None => Err(anyhow!("missing command `play`")),
        Some(other) => Err(anyhow!("unknown command `{other}`")),
    }
}

fn print_usage() {
    eprintln!(
        "usage:
  mamut-tui play [--demo] [--headless] --audio-device <alsa-index-or-hw:card,device> [factory-name-or-path]
  mamut-tui list-factory
  mamut-tui list-audio
  mamut-tui list-midi
  mamut-tui validate [factory-name-or-path]
  mamut-tui dry-run [factory-name-or-path]"
    );
}
