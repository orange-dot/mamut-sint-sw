use std::{
    env, fs,
    io::{self, BufRead, IsTerminal, Write},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use anyhow::{Context, Result, anyhow};
use cpal::{
    FromSample, SampleFormat, SizedSample, Stream, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use mamut_dsp::StereoBlockMut;
use mamut_engine::{
    ControllerEvent, Engine, EngineConfig, EngineSnapshot, NoteEvent, ProcessBlock, Scheduled,
};
use mamut_params::MacroId;
use mamut_patch::{PatchFileV1, load_patch_toml, validate_patch_v1};
use midir::{Ignore, MidiInput, MidiInputConnection, MidiInputPort};

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
            let path = resolve_patch_argument(args.get(1).map(String::as_str))?;
            dry_run(&path)
        }
        Some("play") => {
            let options = parse_play_options(&args[1..])?;
            play(&options)
        }
        Some("help") | Some("--help") | Some("-h") => {
            print_usage();
            Ok(())
        }
        None => dry_run(&default_patch_path()),
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
  mamut-standalone dry-run [factory-name-or-path]
  mamut-standalone play [--demo] [--audio-device <name-or-index>] [--midi-device <name-or-index>] [factory-name-or-path]

interactive play controls:
  help
  status
  patches
  favorites
  patch <factory-name-or-path>
  next
  prev
  demo-patch
  macro <gravitacija|bloom|heat|ruin|swarm> <0..1>
  audio [name-or-index]
  midi [name-or-index]
  demo
  quit"
    );
}

#[derive(Debug, Clone)]
struct FactoryPatchEntry {
    stem: String,
    slug: String,
    path: PathBuf,
    patch_name: String,
    description: Option<String>,
    favorite: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct PlayOptions {
    patch_path: PathBuf,
    force_demo: bool,
    audio_selector: Option<String>,
    midi_selector: Option<String>,
}

struct NamedOutputDevice {
    device: cpal::Device,
    name: String,
    is_default: bool,
}

#[derive(Clone)]
struct NamedMidiPort {
    port: MidiInputPort,
    name: String,
}

#[derive(Debug)]
enum RuntimeCommand {
    Note(NoteEvent),
    Controller(ControllerEvent),
    LoadPatch(PatchFileV1),
    RequestSnapshot(mpsc::Sender<EngineSnapshot>),
}

#[derive(Debug, Clone, PartialEq)]
enum RuntimeUiCommand {
    Noop,
    Help,
    Status,
    Patches,
    Favorites,
    Patch(String),
    NextFavorite,
    PrevFavorite,
    DemoPatch,
    Macro(MacroId, f32),
    AudioList,
    AudioSelect(String),
    MidiList,
    MidiSelect(String),
    Demo,
    Quit,
}

struct AudioRuntime {
    tx: mpsc::Sender<RuntimeCommand>,
    stream: Stream,
    audio_device_name: String,
    sample_rate_hz: u32,
    channels: usize,
    bend_range: f32,
    patch_name: String,
}

enum PerformanceDriver {
    Demo { stop: Arc<AtomicBool> },
    Midi(OpenedMidiConnection),
    Idle,
}

impl PerformanceDriver {
    fn is_demo(&self) -> bool {
        matches!(self, Self::Demo { .. })
    }

    fn detail(&self) -> String {
        match self {
            Self::Demo { .. } => "demo performer active".to_string(),
            Self::Midi(connection) => format!("connected ({})", connection.port_name),
            Self::Idle => "no active input".to_string(),
        }
    }

    fn stop(&mut self) {
        let previous = std::mem::replace(self, Self::Idle);
        match previous {
            Self::Demo { stop } => {
                stop.store(true, Ordering::Relaxed);
            }
            Self::Midi(_) | Self::Idle => {}
        }
    }
}

struct RuntimeSession {
    patch_path: PathBuf,
    patch_name: String,
    audio_selector: Option<String>,
    midi_selector: Option<String>,
    bend_range: f32,
    tx: mpsc::Sender<RuntimeCommand>,
    stream: Stream,
    driver: PerformanceDriver,
    audio_device_name: String,
    sample_rate_hz: u32,
    channels: usize,
}

impl RuntimeSession {
    fn new(options: &PlayOptions) -> Result<Self> {
        let runtime = build_audio_runtime(&options.patch_path, options.audio_selector.as_deref())?;
        let mut session = Self {
            patch_path: options.patch_path.clone(),
            patch_name: runtime.patch_name,
            audio_selector: options.audio_selector.clone(),
            midi_selector: options.midi_selector.clone(),
            bend_range: runtime.bend_range,
            tx: runtime.tx,
            stream: runtime.stream,
            driver: PerformanceDriver::Idle,
            audio_device_name: runtime.audio_device_name,
            sample_rate_hz: runtime.sample_rate_hz,
            channels: runtime.channels,
        };
        session.stream.play().context("failed to start output stream")?;
        session.driver = session.open_startup_driver(options.force_demo)?;
        Ok(session)
    }

    fn open_startup_driver(&self, force_demo: bool) -> Result<PerformanceDriver> {
        if force_demo {
            return Ok(PerformanceDriver::Demo {
                stop: spawn_demo_performance(self.tx.clone()),
            });
        }

        if let Some(connection) =
            open_midi_input(self.tx.clone(), self.bend_range, self.midi_selector.as_deref())?
        {
            Ok(PerformanceDriver::Midi(connection))
        } else {
            Ok(PerformanceDriver::Demo {
                stop: spawn_demo_performance(self.tx.clone()),
            })
        }
    }

    fn print_startup_summary(&self) {
        println!(
            "play patch: {} ({})",
            self.patch_name,
            self.patch_path.display()
        );
        println!(
            "audio: {} @ {} Hz, {} channels",
            self.audio_device_name, self.sample_rate_hz, self.channels
        );
        println!("mode: {}", self.driver.detail());
        println!(
            "cc map: 1=mod wheel, 64=sustain, 71=heat, 73=bloom, 74=gravitacija, 75=ruin, 76=swarm"
        );
        println!("interactive controls active; type `help` for commands");
    }

    fn command_loop(&mut self) -> Result<()> {
        self.print_startup_summary();
        self.print_status()?;

        let stdin = io::stdin();
        let mut lines = stdin.lock().lines();

        loop {
            print!("epm1> ");
            io::stdout().flush().context("failed to flush prompt")?;

            let Some(line_result) = lines.next() else {
                println!();
                println!("stdin closed; stopping session");
                break;
            };

            let line = line_result.context("failed to read command input")?;
            match parse_runtime_ui_command(&line) {
                Ok(RuntimeUiCommand::Noop) => continue,
                Ok(RuntimeUiCommand::Help) => print_runtime_help(),
                Ok(RuntimeUiCommand::Status) => self.print_status()?,
                Ok(RuntimeUiCommand::Patches) => list_factory_patches()?,
                Ok(RuntimeUiCommand::Favorites) => list_favorite_patches()?,
                Ok(RuntimeUiCommand::Patch(argument)) => {
                    let path = resolve_patch_argument(Some(argument.as_str()))?;
                    self.switch_patch(path)?;
                    self.print_status()?;
                }
                Ok(RuntimeUiCommand::NextFavorite) => {
                    self.switch_favorite(1)?;
                    self.print_status()?;
                }
                Ok(RuntimeUiCommand::PrevFavorite) => {
                    self.switch_favorite(-1)?;
                    self.print_status()?;
                }
                Ok(RuntimeUiCommand::DemoPatch) => {
                    self.switch_patch(default_patch_path())?;
                    self.print_status()?;
                }
                Ok(RuntimeUiCommand::Macro(id, value)) => {
                    self.set_macro(id, value)?;
                    println!("macro {} -> {:.3}", macro_display_name(id), value);
                }
                Ok(RuntimeUiCommand::AudioList) => list_audio_devices()?,
                Ok(RuntimeUiCommand::AudioSelect(selector)) => {
                    self.switch_audio(selector)?;
                    self.print_status()?;
                }
                Ok(RuntimeUiCommand::MidiList) => list_midi_devices()?,
                Ok(RuntimeUiCommand::MidiSelect(selector)) => {
                    self.switch_midi(selector)?;
                    self.print_status()?;
                }
                Ok(RuntimeUiCommand::Demo) => {
                    self.enable_demo();
                    self.print_status()?;
                }
                Ok(RuntimeUiCommand::Quit) => break,
                Err(error) => eprintln!("command error: {error}"),
            }
        }

        Ok(())
    }

    fn block_forever(&self) {
        loop {
            thread::sleep(Duration::from_secs(1));
        }
    }

    fn switch_patch(&mut self, path: PathBuf) -> Result<()> {
        let patch = load_patch_from_path(&path)?;
        validate_patch_v1(&patch).context("patch validation failed")?;
        self.patch_name = patch.meta.patch_name.clone();
        self.bend_range = patch.performance_response.bend_range_semitones as f32;
        self.tx
            .send(RuntimeCommand::LoadPatch(patch))
            .map_err(|_| anyhow!("audio runtime is no longer available"))?;
        self.patch_path = path.clone();
        println!("loaded patch: {} ({})", self.patch_name, path.display());
        Ok(())
    }

    fn set_macro(&self, id: MacroId, value: f32) -> Result<()> {
        self.tx
            .send(RuntimeCommand::Controller(ControllerEvent::Macro {
                id,
                value: value.clamp(0.0, 1.0),
            }))
            .map_err(|_| anyhow!("audio runtime is no longer available"))
    }

    fn print_status(&self) -> Result<()> {
        let snapshot = self.request_snapshot()?;
        let description = snapshot
            .patch_description
            .as_deref()
            .unwrap_or("no description");
        let tags = if snapshot.patch_tags.is_empty() {
            "none".to_string()
        } else {
            snapshot.patch_tags.join(", ")
        };

        println!("patch: {} - {}", snapshot.patch_name, description);
        println!(
            "favorite: {}",
            if snapshot.patch_favorite { "yes" } else { "no" }
        );
        println!(
            "audio: {} @ {} Hz, {} channels",
            self.audio_device_name, self.sample_rate_hz, self.channels
        );
        println!("mode: {}", self.driver.detail());
        println!("tags: {tags}");
        println!(
            "voices: active={} sustain={} held={:?} peak={:.3}",
            snapshot.active_voice_count,
            snapshot.sustain_down,
            snapshot.held_notes,
            snapshot.peak_output
        );
        println!(
            "macros: gravitacija={:.3} bloom={:.3} heat={:.3} ruin={:.3} swarm={:.3}",
            snapshot.effective_macros.gravitacija,
            snapshot.effective_macros.bloom,
            snapshot.effective_macros.heat,
            snapshot.effective_macros.ruin,
            snapshot.effective_macros.swarm
        );
        println!(
            "identity: horizont_open={:.3} pec_mass={:.3} baklja_ready={:.3} grav_pull={:.3}",
            snapshot.identity.horizont_open,
            snapshot.identity.pec_mass,
            snapshot.identity.baklja_ready,
            snapshot.identity.grav_pull
        );
        println!(
            "derived: mass={:.3} strain={:.3} headroom={:.3} threshold={:.3}",
            snapshot.derived.mass,
            snapshot.derived.strain,
            snapshot.derived.headroom,
            snapshot.derived.rupture_threshold
        );
        Ok(())
    }

    fn request_snapshot(&self) -> Result<EngineSnapshot> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(RuntimeCommand::RequestSnapshot(reply_tx))
            .map_err(|_| anyhow!("audio runtime is no longer available"))?;
        reply_rx
            .recv_timeout(Duration::from_millis(500))
            .map_err(|_| anyhow!("timed out waiting for engine snapshot"))
    }

    fn switch_audio(&mut self, selector: String) -> Result<()> {
        let keep_demo = self.driver.is_demo();
        let runtime = build_audio_runtime(&self.patch_path, Some(selector.as_str()))?;
        let replacement_driver = if keep_demo {
            PerformanceDriver::Demo {
                stop: spawn_demo_performance(runtime.tx.clone()),
            }
        } else if let Some(connection) =
            open_midi_input(runtime.tx.clone(), runtime.bend_range, self.midi_selector.as_deref())?
        {
            PerformanceDriver::Midi(connection)
        } else {
            PerformanceDriver::Demo {
                stop: spawn_demo_performance(runtime.tx.clone()),
            }
        };

        runtime
            .stream
            .play()
            .context("failed to start replacement audio stream")?;

        let mut old_driver = std::mem::replace(&mut self.driver, replacement_driver);
        old_driver.stop();
        let old_stream = std::mem::replace(&mut self.stream, runtime.stream);
        drop(old_stream);

        self.tx = runtime.tx;
        self.audio_device_name = runtime.audio_device_name;
        self.sample_rate_hz = runtime.sample_rate_hz;
        self.channels = runtime.channels;
        self.bend_range = runtime.bend_range;
        self.patch_name = runtime.patch_name;
        self.audio_selector = Some(selector.clone());

        println!("audio switched to `{selector}`");
        Ok(())
    }

    fn switch_midi(&mut self, selector: String) -> Result<()> {
        let connection = open_midi_input(self.tx.clone(), self.bend_range, Some(selector.as_str()))?
            .ok_or_else(|| anyhow!("no MIDI input devices available"))?;
        let mut old_driver =
            std::mem::replace(&mut self.driver, PerformanceDriver::Midi(connection));
        old_driver.stop();
        self.midi_selector = Some(selector.clone());
        println!("midi switched to `{selector}`");
        Ok(())
    }

    fn enable_demo(&mut self) {
        let demo = PerformanceDriver::Demo {
            stop: spawn_demo_performance(self.tx.clone()),
        };
        let mut old_driver = std::mem::replace(&mut self.driver, demo);
        old_driver.stop();
        println!("demo performer enabled");
    }

    fn switch_favorite(&mut self, direction: isize) -> Result<()> {
        let path = adjacent_favorite_patch(&self.patch_path, direction)?;
        self.switch_patch(path)
    }
}

fn list_factory_patches() -> Result<()> {
    for entry in factory_patch_entries()? {
        let favorite = if entry.favorite { "*" } else { " " };
        let description = entry
            .description
            .as_deref()
            .unwrap_or("no description")
            .trim();
        println!(
            "{favorite} {:<18} {:<20} {}",
            entry.stem, entry.patch_name, description
        );
    }
    Ok(())
}

fn list_favorite_patches() -> Result<()> {
    for entry in favorite_patch_entries()? {
        let description = entry
            .description
            .as_deref()
            .unwrap_or("no description")
            .trim();
        println!(
            "* {:<18} {:<20} {}",
            entry.stem, entry.patch_name, description
        );
    }
    Ok(())
}

fn list_audio_devices() -> Result<()> {
    let host = cpal::default_host();
    let devices = collect_output_devices(&host)?;
    if devices.is_empty() {
        println!("no output devices found");
        return Ok(());
    }

    for (index, entry) in devices.iter().enumerate() {
        let default_marker = if entry.is_default { " [default]" } else { "" };
        println!("{index}: {}{default_marker}", entry.name);
    }
    Ok(())
}

fn list_midi_devices() -> Result<()> {
    let midi_input = match MidiInput::new("mamut-standalone") {
        Ok(midi_input) => midi_input,
        Err(error) => {
            println!("MIDI support unavailable: {error}");
            return Ok(());
        }
    };
    let ports = collect_midi_ports(&midi_input)?;
    if ports.is_empty() {
        println!("no MIDI input devices found");
        return Ok(());
    }

    for (index, port) in ports.iter().enumerate() {
        println!("{index}: {}", port.name);
    }
    Ok(())
}

fn dry_run(path: &Path) -> Result<()> {
    let patch = load_patch_from_path(path)?;
    validate_patch_v1(&patch).context("patch validation failed")?;
    let mut engine = Engine::new(EngineConfig::default(), patch)?;
    let mut left = vec![0.0_f32; 256];
    let mut right = vec![0.0_f32; 256];

    let note_events = [
        Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: 60,
                velocity: 0.88,
            },
        },
        Scheduled {
            frame_offset: 200,
            event: NoteEvent::NoteOff { note: 60 },
        },
    ];
    let controller_events = [
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::ModWheel { amount: 0.45 },
        },
        Scheduled {
            frame_offset: 64,
            event: ControllerEvent::ChannelAftertouch { pressure: 0.30 },
        },
        Scheduled {
            frame_offset: 96,
            event: ControllerEvent::Macro {
                id: MacroId::Gravitacija,
                value: 0.64,
            },
        },
        Scheduled {
            frame_offset: 160,
            event: ControllerEvent::Macro {
                id: MacroId::Ruin,
                value: 0.42,
            },
        },
    ];

    engine.process_block(ProcessBlock {
        frame_count: 256,
        note_events: &note_events,
        controller_events: &controller_events,
        macro_state: None,
        output: Some(StereoBlockMut::new(&mut left, &mut right)),
    });

    let snapshot = engine.snapshot();
    println!("patch: {} ({})", snapshot.patch_name, path.display());
    println!("description: {}", snapshot.patch_description.unwrap_or_default());
    println!("active voices: {}", snapshot.active_voice_count);
    println!("held notes: {:?}", snapshot.held_notes);
    println!(
        "macros: gravitacija={:.3} bloom={:.3} heat={:.3} ruin={:.3} swarm={:.3}",
        snapshot.effective_macros.gravitacija,
        snapshot.effective_macros.bloom,
        snapshot.effective_macros.heat,
        snapshot.effective_macros.ruin,
        snapshot.effective_macros.swarm
    );
    println!(
        "identity: horizont_open={:.3} pec_mass={:.3} baklja_ready={:.3} grav_pull={:.3}",
        snapshot.identity.horizont_open,
        snapshot.identity.pec_mass,
        snapshot.identity.baklja_ready,
        snapshot.identity.grav_pull
    );
    println!(
        "derived: mass={:.3} strain={:.3} headroom={:.3} rupture_threshold={:.3}",
        snapshot.derived.mass,
        snapshot.derived.strain,
        snapshot.derived.headroom,
        snapshot.derived.rupture_threshold
    );
    println!(
        "direct: cutoff_hz={:.2} sync={:.3} crossmod={:.3} body_drive={:.3} peak={:.3}",
        snapshot.direct.cutoff_hz,
        snapshot.direct.sync_amount,
        snapshot.direct.crossmod_amount,
        snapshot.direct.body_drive,
        snapshot.peak_output
    );

    Ok(())
}

fn play(options: &PlayOptions) -> Result<()> {
    let mut session = RuntimeSession::new(options)?;
    if io::stdin().is_terminal() {
        session.command_loop()
    } else {
        session.print_startup_summary();
        println!("stdin is not a terminal; running without interactive controls");
        session.block_forever();
        #[allow(unreachable_code)]
        Ok(())
    }
}

fn build_audio_runtime(patch_path: &Path, audio_selector: Option<&str>) -> Result<AudioRuntime> {
    let patch = load_patch_from_path(patch_path)?;
    validate_patch_v1(&patch).context("patch validation failed")?;

    let host = cpal::default_host();
    let device = select_output_device(&host, audio_selector)?;
    let audio_device_name = device
        .name()
        .unwrap_or_else(|_| "unknown-device".to_string());
    let supported_config = device
        .default_output_config()
        .context("failed to query default output config")?;
    let sample_format = supported_config.sample_format();
    let stream_config: StreamConfig = supported_config.clone().into();
    let sample_rate_hz = stream_config.sample_rate.0 as f32;
    let channels = usize::from(stream_config.channels);
    let bend_range = patch.performance_response.bend_range_semitones as f32;
    let patch_name = patch.meta.patch_name.clone();

    let (tx, rx) = mpsc::channel::<RuntimeCommand>();
    let engine = Engine::new(
        EngineConfig {
            sample_rate_hz,
            max_block_frames: 2_048,
            voice_count: 6,
        },
        patch,
    )?;

    let audio_state = AudioThreadState::new(engine, rx);
    let stream = match sample_format {
        SampleFormat::F32 => {
            build_output_stream::<f32>(&device, &stream_config, channels, audio_state)?
        }
        SampleFormat::I16 => {
            build_output_stream::<i16>(&device, &stream_config, channels, audio_state)?
        }
        SampleFormat::U16 => {
            build_output_stream::<u16>(&device, &stream_config, channels, audio_state)?
        }
        other => return Err(anyhow!("unsupported output sample format: {other:?}")),
    };

    Ok(AudioRuntime {
        tx,
        stream,
        audio_device_name,
        sample_rate_hz: stream_config.sample_rate.0,
        channels,
        bend_range,
        patch_name,
    })
}

fn print_runtime_help() {
    println!("runtime commands:");
    println!("  help                     show this command list");
    println!("  status                   show current patch, mode, macros, and activity");
    println!("  patches                  list factory patches");
    println!("  favorites                list the live-ready favorite patches");
    println!("  patch <name-or-path>     load a factory patch or explicit TOML path");
    println!("  next                     load the next favorite patch");
    println!("  prev                     load the previous favorite patch");
    println!("  demo-patch               jump to the default opener patch");
    println!("  macro <name> <0..1>      set one public macro");
    println!("  audio                    list available audio outputs");
    println!("  audio <name-or-index>    switch audio output (resets live performance state)");
    println!("  midi                     list available MIDI inputs");
    println!("  midi <name-or-index>     switch to a MIDI input");
    println!("  demo                     switch the session to the demo performer");
    println!("  quit                     stop playback and exit");
}

fn parse_play_options(args: &[String]) -> Result<PlayOptions> {
    let mut patch_arg: Option<String> = None;
    let mut force_demo = false;
    let mut audio_selector = None;
    let mut midi_selector = None;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--demo" => {
                force_demo = true;
                index += 1;
            }
            "--audio-device" => {
                let value = args
                    .get(index + 1)
                    .context("missing value after --audio-device")?;
                audio_selector = Some(value.clone());
                index += 2;
            }
            "--midi-device" => {
                let value = args
                    .get(index + 1)
                    .context("missing value after --midi-device")?;
                midi_selector = Some(value.clone());
                index += 2;
            }
            option if option.starts_with("--") => {
                return Err(anyhow!("unknown play option `{option}`"));
            }
            patch if patch_arg.is_none() => {
                patch_arg = Some(patch.to_string());
                index += 1;
            }
            extra => {
                return Err(anyhow!(
                    "unexpected extra argument `{extra}`; pass at most one patch name or path"
                ));
            }
        }
    }

    Ok(PlayOptions {
        patch_path: resolve_patch_argument(patch_arg.as_deref())?,
        force_demo,
        audio_selector,
        midi_selector,
    })
}

fn parse_runtime_ui_command(input: &str) -> Result<RuntimeUiCommand> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(RuntimeUiCommand::Noop);
    }

    let mut parts = trimmed.split_whitespace();
    let command = parts.next().unwrap_or_default();

    match command {
        "help" | "h" => Ok(RuntimeUiCommand::Help),
        "status" | "s" => Ok(RuntimeUiCommand::Status),
        "patches" | "list-patches" => Ok(RuntimeUiCommand::Patches),
        "favorites" | "favs" => Ok(RuntimeUiCommand::Favorites),
        "patch" => {
            let argument = parts.collect::<Vec<_>>().join(" ");
            if argument.trim().is_empty() {
                Err(anyhow!("patch command requires a factory name or path"))
            } else {
                Ok(RuntimeUiCommand::Patch(argument))
            }
        }
        "next" | "next-favorite" => Ok(RuntimeUiCommand::NextFavorite),
        "prev" | "previous" | "prev-favorite" => Ok(RuntimeUiCommand::PrevFavorite),
        "demo-patch" => Ok(RuntimeUiCommand::DemoPatch),
        "macro" => {
            let macro_name = parts.next().context("macro command requires a macro name")?;
            let value = parts.next().context("macro command requires a value")?;
            if parts.next().is_some() {
                return Err(anyhow!("macro command accepts exactly two arguments"));
            }
            let macro_id = parse_macro_id(macro_name)?;
            let value = value
                .parse::<f32>()
                .with_context(|| format!("invalid macro value `{value}`"))?;
            Ok(RuntimeUiCommand::Macro(macro_id, value.clamp(0.0, 1.0)))
        }
        "audio" => {
            let selector = parts.collect::<Vec<_>>().join(" ");
            if selector.trim().is_empty() {
                Ok(RuntimeUiCommand::AudioList)
            } else {
                Ok(RuntimeUiCommand::AudioSelect(selector))
            }
        }
        "midi" => {
            let selector = parts.collect::<Vec<_>>().join(" ");
            if selector.trim().is_empty() {
                Ok(RuntimeUiCommand::MidiList)
            } else {
                Ok(RuntimeUiCommand::MidiSelect(selector))
            }
        }
        "demo" => Ok(RuntimeUiCommand::Demo),
        "quit" | "exit" => Ok(RuntimeUiCommand::Quit),
        other => Err(anyhow!("unknown runtime command `{other}`")),
    }
}

fn parse_macro_id(value: &str) -> Result<MacroId> {
    match value.trim().to_ascii_lowercase().as_str() {
        "gravitacija" => Ok(MacroId::Gravitacija),
        "bloom" => Ok(MacroId::Bloom),
        "heat" => Ok(MacroId::Heat),
        "ruin" => Ok(MacroId::Ruin),
        "swarm" => Ok(MacroId::Swarm),
        _ => Err(anyhow!(
            "unknown macro `{value}`; expected gravitacija, bloom, heat, ruin, or swarm"
        )),
    }
}

fn macro_display_name(id: MacroId) -> &'static str {
    match id {
        MacroId::Gravitacija => "Gravitacija",
        MacroId::Bloom => "Bloom",
        MacroId::Heat => "Heat",
        MacroId::Ruin => "Ruin",
        MacroId::Swarm => "Swarm",
    }
}

fn load_patch_from_path(path: &Path) -> Result<PatchFileV1> {
    let input = fs::read_to_string(path)
        .with_context(|| format!("failed to read patch file {}", path.display()))?;
    load_patch_toml(&input)
        .with_context(|| format!("failed to parse patch file {}", path.display()))
}

fn resolve_patch_argument(argument: Option<&str>) -> Result<PathBuf> {
    let Some(argument) = argument else {
        return Ok(default_patch_path());
    };

    let direct_path = PathBuf::from(argument);
    if direct_path.exists() {
        return Ok(direct_path);
    }

    let factory_dir = workspace_root().join("patches/factory");
    let factory_candidate = if argument.ends_with(".toml") {
        factory_dir.join(argument)
    } else {
        factory_dir.join(format!("{argument}.toml"))
    };
    if factory_candidate.exists() {
        return Ok(factory_candidate);
    }

    let argument_slug = slugify(argument);
    for entry in factory_patch_entries()? {
        if entry.stem.eq_ignore_ascii_case(argument) || entry.slug == argument_slug {
            return Ok(entry.path);
        }
    }

    Err(anyhow!(
        "unknown patch `{argument}`; use a path or run `list-factory` for available factory names"
    ))
}

fn factory_patch_entries() -> Result<Vec<FactoryPatchEntry>> {
    let factory_dir = workspace_root().join("patches/factory");
    let mut entries = Vec::new();

    for entry in fs::read_dir(&factory_dir).with_context(|| {
        format!(
            "failed to read factory patch directory {}",
            factory_dir.display()
        )
    })? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }

        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("toml") {
            continue;
        }

        let patch = load_patch_from_path(&path)?;
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or_else(|| anyhow!("invalid factory patch filename {}", path.display()))?
            .to_string();
        let favorite = patch
            .ui
            .as_ref()
            .and_then(|ui| ui.favorite)
            .unwrap_or(false);
        entries.push(FactoryPatchEntry {
            stem: stem.clone(),
            slug: slugify(&patch.meta.patch_name),
            path,
            patch_name: patch.meta.patch_name,
            description: patch.meta.description,
            favorite,
        });
    }

    entries.sort_by(|left, right| left.stem.cmp(&right.stem));
    Ok(entries)
}

fn favorite_patch_entries() -> Result<Vec<FactoryPatchEntry>> {
    let favorites: Vec<FactoryPatchEntry> = factory_patch_entries()?
        .into_iter()
        .filter(|entry| entry.favorite)
        .collect();
    if favorites.is_empty() {
        return Err(anyhow!("no favorite patches are marked in the factory bank"));
    }
    Ok(favorites)
}

fn adjacent_favorite_patch(current_path: &Path, direction: isize) -> Result<PathBuf> {
    let favorites = favorite_patch_entries()?;
    let current_stem = current_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let current_index = favorites
        .iter()
        .position(|entry| entry.stem == current_stem);

    let target_index = match current_index {
        Some(index) => wrap_index(index, direction, favorites.len()),
        None if direction >= 0 => 0,
        None => favorites.len().saturating_sub(1),
    };

    favorites
        .get(target_index)
        .map(|entry| entry.path.clone())
        .ok_or_else(|| anyhow!("favorite patch selection failed"))
}

fn wrap_index(index: usize, direction: isize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let len = len as isize;
    let index = index as isize;
    ((index + direction).rem_euclid(len)) as usize
}

fn collect_output_devices(host: &cpal::Host) -> Result<Vec<NamedOutputDevice>> {
    let default_name = host
        .default_output_device()
        .and_then(|device| device.name().ok());
    let mut devices = Vec::new();
    for device in host
        .output_devices()
        .context("failed to enumerate output devices")?
    {
        let name = device
            .name()
            .unwrap_or_else(|_| "unknown-device".to_string());
        let is_default = default_name
            .as_deref()
            .is_some_and(|default_name| default_name == name);
        devices.push(NamedOutputDevice {
            device,
            name,
            is_default,
        });
    }
    Ok(devices)
}

fn select_output_device(host: &cpal::Host, selector: Option<&str>) -> Result<cpal::Device> {
    if selector.is_none() {
        return host
            .default_output_device()
            .context("no default output device available");
    }

    let devices = collect_output_devices(host)?;
    if devices.is_empty() {
        return Err(anyhow!("no output devices available"));
    }

    let names: Vec<String> = devices.iter().map(|entry| entry.name.clone()).collect();
    let index = select_named_index(selector.unwrap_or_default(), &names, "audio output device")?;
    Ok(devices
        .into_iter()
        .nth(index)
        .map(|entry| entry.device)
        .ok_or_else(|| anyhow!("audio output device index {index} is out of range"))?)
}

fn collect_midi_ports(midi_input: &MidiInput) -> Result<Vec<NamedMidiPort>> {
    let mut ports = Vec::new();
    for port in midi_input.ports() {
        let name = midi_input
            .port_name(&port)
            .unwrap_or_else(|_| "unknown-midi-port".to_string());
        ports.push(NamedMidiPort { port, name });
    }
    Ok(ports)
}

fn open_midi_input(
    tx: mpsc::Sender<RuntimeCommand>,
    bend_range: f32,
    selector: Option<&str>,
) -> Result<Option<OpenedMidiConnection>> {
    let mut midi_input = match MidiInput::new("mamut-standalone") {
        Ok(midi_input) => midi_input,
        Err(error) => {
            return if selector.is_some() {
                Err(anyhow!("failed to create MIDI input: {error}"))
            } else {
                Ok(None)
            };
        }
    };
    midi_input.ignore(Ignore::None);
    let ports = collect_midi_ports(&midi_input)?;

    if ports.is_empty() {
        return if selector.is_some() {
            Err(anyhow!("no MIDI input devices available"))
        } else {
            Ok(None)
        };
    }

    let port_index = if let Some(selector) = selector {
        let names: Vec<String> = ports.iter().map(|entry| entry.name.clone()).collect();
        select_named_index(selector, &names, "MIDI input device")?
    } else {
        0
    };

    let selected = ports
        .into_iter()
        .nth(port_index)
        .ok_or_else(|| anyhow!("MIDI input device index {port_index} is out of range"))?;
    let port_name = selected.name.clone();
    let connection = midi_input
        .connect(
            &selected.port,
            "mamut-midi-in",
            move |_stamp, message, _| {
                if let Some(command) = parse_midi_message(message, bend_range) {
                    let _ = tx.send(command);
                }
            },
            (),
        )
        .map_err(|error| anyhow!("failed to open MIDI input connection: {error}"))?;

    Ok(Some(OpenedMidiConnection {
        port_name,
        _connection: connection,
    }))
}

fn select_named_index(selector: &str, names: &[String], kind: &str) -> Result<usize> {
    if let Ok(index) = selector.parse::<usize>() {
        return if index < names.len() {
            Ok(index)
        } else {
            Err(anyhow!(
                "{kind} index {index} is out of range; available count is {}",
                names.len()
            ))
        };
    }

    if let Some((index, _)) = names
        .iter()
        .enumerate()
        .find(|(_, name)| name.eq_ignore_ascii_case(selector))
    {
        return Ok(index);
    }

    let selector_lower = selector.to_ascii_lowercase();
    let matches: Vec<usize> = names
        .iter()
        .enumerate()
        .filter_map(|(index, name)| {
            name.to_ascii_lowercase()
                .contains(&selector_lower)
                .then_some(index)
        })
        .collect();

    match matches.as_slice() {
        [index] => Ok(*index),
        [] => Err(anyhow!(
            "no {kind} matched `{selector}`; run the relevant list command to inspect available devices"
        )),
        _ => Err(anyhow!(
            "selector `{selector}` is ambiguous for {kind}; use a numeric index or a more specific name"
        )),
    }
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut last_was_hyphen = false;

    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_was_hyphen = false;
        } else if !last_was_hyphen && !slug.is_empty() {
            slug.push('-');
            last_was_hyphen = true;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    slug
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

fn default_patch_path() -> PathBuf {
    workspace_root().join("patches/factory/molten-horizon.toml")
}

struct AudioThreadState {
    engine: Engine,
    rx: mpsc::Receiver<RuntimeCommand>,
    left: Vec<f32>,
    right: Vec<f32>,
    note_events: Vec<Scheduled<NoteEvent>>,
    controller_events: Vec<Scheduled<ControllerEvent>>,
}

impl AudioThreadState {
    fn new(engine: Engine, rx: mpsc::Receiver<RuntimeCommand>) -> Self {
        Self {
            engine,
            rx,
            left: Vec::new(),
            right: Vec::new(),
            note_events: Vec::new(),
            controller_events: Vec::new(),
        }
    }

    fn render_to<T>(&mut self, output: &mut [T], channels: usize)
    where
        T: SizedSample + FromSample<f32>,
    {
        let frame_count = output.len() / channels.max(1);
        self.ensure_scratch(frame_count);
        self.note_events.clear();
        self.controller_events.clear();

        while let Ok(command) = self.rx.try_recv() {
            match command {
                RuntimeCommand::Note(event) => self.note_events.push(Scheduled {
                    frame_offset: 0,
                    event,
                }),
                RuntimeCommand::Controller(event) => self.controller_events.push(Scheduled {
                    frame_offset: 0,
                    event,
                }),
                RuntimeCommand::LoadPatch(patch) => {
                    self.note_events.clear();
                    self.controller_events.clear();
                    if let Err(error) = self.engine.load_patch(patch) {
                        eprintln!("patch load failed in audio runtime: {error}");
                    }
                }
                RuntimeCommand::RequestSnapshot(reply) => {
                    let _ = reply.send(self.engine.snapshot());
                }
            }
        }

        self.engine.process_block(ProcessBlock {
            frame_count,
            note_events: &self.note_events,
            controller_events: &self.controller_events,
            macro_state: None,
            output: Some(StereoBlockMut::new(
                &mut self.left[..frame_count],
                &mut self.right[..frame_count],
            )),
        });

        for (frame_index, frame) in output.chunks_mut(channels).enumerate() {
            let left = self.left[frame_index];
            let right = self.right[frame_index];
            let mono = (left + right) * 0.5;

            frame[0] = T::from_sample(left);
            if channels > 1 {
                frame[1] = T::from_sample(right);
            }
            for sample in frame.iter_mut().skip(2) {
                *sample = T::from_sample(mono);
            }
        }
    }

    fn ensure_scratch(&mut self, frame_count: usize) {
        if self.left.len() < frame_count {
            self.left.resize(frame_count, 0.0);
        }
        if self.right.len() < frame_count {
            self.right.resize(frame_count, 0.0);
        }
    }
}

fn build_output_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    channels: usize,
    mut audio_state: AudioThreadState,
) -> Result<Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _| audio_state.render_to(data, channels),
        move |error| eprintln!("audio stream error: {error}"),
        None,
    )?;
    Ok(stream)
}

struct OpenedMidiConnection {
    port_name: String,
    _connection: MidiInputConnection<()>,
}

fn parse_midi_message(message: &[u8], bend_range: f32) -> Option<RuntimeCommand> {
    let status = *message.first()? & 0xF0;
    match status {
        0x80 if message.len() >= 2 => Some(RuntimeCommand::Note(NoteEvent::NoteOff {
            note: message[1],
        })),
        0x90 if message.len() >= 3 => {
            if message[2] == 0 {
                Some(RuntimeCommand::Note(NoteEvent::NoteOff {
                    note: message[1],
                }))
            } else {
                Some(RuntimeCommand::Note(NoteEvent::NoteOn {
                    note: message[1],
                    velocity: message[2] as f32 / 127.0,
                }))
            }
        }
        0xB0 if message.len() >= 3 => {
            let cc = message[1];
            let value = message[2] as f32 / 127.0;
            let event = match cc {
                1 => ControllerEvent::ModWheel { amount: value },
                64 => ControllerEvent::Sustain { down: value >= 0.5 },
                71 => ControllerEvent::Macro {
                    id: MacroId::Heat,
                    value,
                },
                73 => ControllerEvent::Macro {
                    id: MacroId::Bloom,
                    value,
                },
                74 => ControllerEvent::Macro {
                    id: MacroId::Gravitacija,
                    value,
                },
                75 => ControllerEvent::Macro {
                    id: MacroId::Ruin,
                    value,
                },
                76 => ControllerEvent::Macro {
                    id: MacroId::Swarm,
                    value,
                },
                _ => return None,
            };
            Some(RuntimeCommand::Controller(event))
        }
        0xD0 if message.len() >= 2 => Some(RuntimeCommand::Controller(
            ControllerEvent::ChannelAftertouch {
                pressure: message[1] as f32 / 127.0,
            },
        )),
        0xE0 if message.len() >= 3 => {
            let value = u16::from(message[1]) | (u16::from(message[2]) << 7);
            let normalized = (value as f32 - 8_192.0) / 8_192.0;
            Some(RuntimeCommand::Controller(ControllerEvent::PitchBend {
                semitones: normalized.clamp(-1.0, 1.0) * bend_range,
            }))
        }
        _ => None,
    }
}

fn spawn_demo_performance(tx: mpsc::Sender<RuntimeCommand>) -> Arc<AtomicBool> {
    let stop = Arc::new(AtomicBool::new(false));
    let thread_stop = Arc::clone(&stop);
    thread::spawn(move || {
        let notes = [36_u8, 43, 48, 55, 60, 67];
        let macro_cycle = [
            (MacroId::Bloom, 0.65),
            (MacroId::Heat, 0.58),
            (MacroId::Gravitacija, 0.52),
            (MacroId::Ruin, 0.38),
            (MacroId::Swarm, 0.44),
            (MacroId::Gravitacija, 0.74),
            (MacroId::Ruin, 0.62),
        ];

        let mut step = 0_usize;
        loop {
            if thread_stop.load(Ordering::Relaxed) {
                break;
            }

            let note = notes[step % notes.len()];
            let (macro_id, macro_value) = macro_cycle[step % macro_cycle.len()];
            let velocity = 0.62 + (step % 3) as f32 * 0.10;

            if tx
                .send(RuntimeCommand::Controller(ControllerEvent::Macro {
                    id: macro_id,
                    value: macro_value,
                }))
                .is_err()
            {
                break;
            }
            if tx
                .send(RuntimeCommand::Controller(ControllerEvent::ModWheel {
                    amount: ((step % 5) as f32) / 5.0,
                }))
                .is_err()
            {
                break;
            }
            if tx
                .send(RuntimeCommand::Note(NoteEvent::NoteOn { note, velocity }))
                .is_err()
            {
                break;
            }
            if sleep_interruptibly(&thread_stop, Duration::from_millis(420)) {
                break;
            }
            if tx
                .send(RuntimeCommand::Controller(
                    ControllerEvent::ChannelAftertouch {
                        pressure: 0.20 + ((step % 4) as f32) * 0.12,
                    },
                ))
                .is_err()
            {
                break;
            }
            if sleep_interruptibly(&thread_stop, Duration::from_millis(360)) {
                break;
            }
            if tx
                .send(RuntimeCommand::Note(NoteEvent::NoteOff { note }))
                .is_err()
            {
                break;
            }
            if sleep_interruptibly(&thread_stop, Duration::from_millis(140)) {
                break;
            }
            step = step.wrapping_add(1);
        }
    });
    stop
}

fn sleep_interruptibly(stop: &AtomicBool, duration: Duration) -> bool {
    let mut remaining = duration;
    while remaining > Duration::ZERO {
        if stop.load(Ordering::Relaxed) {
            return true;
        }
        let slice = remaining.min(Duration::from_millis(50));
        thread::sleep(slice);
        remaining = remaining.saturating_sub(slice);
    }
    stop.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_factory_patch_by_stem() {
        let path = resolve_patch_argument(Some("molten-horizon")).expect("factory patch resolves");
        assert!(path.ends_with("patches/factory/molten-horizon.toml"));
    }

    #[test]
    fn resolve_factory_patch_by_patch_name_slug() {
        let path = resolve_patch_argument(Some("Molten Horizon")).expect("patch name resolves");
        assert!(path.ends_with("patches/factory/molten-horizon.toml"));
    }

    #[test]
    fn parse_play_options_supports_device_selection() {
        let args = vec![
            "--demo".to_string(),
            "--audio-device".to_string(),
            "2".to_string(),
            "--midi-device".to_string(),
            "Launchkey".to_string(),
            "razor-thaw".to_string(),
        ];

        let options = parse_play_options(&args).expect("play options parse");
        assert!(options.force_demo);
        assert_eq!(options.audio_selector.as_deref(), Some("2"));
        assert_eq!(options.midi_selector.as_deref(), Some("Launchkey"));
        assert!(
            options
                .patch_path
                .ends_with("patches/factory/razor-thaw.toml")
        );
    }

    #[test]
    fn parse_runtime_ui_command_supports_patch_and_macro_commands() {
        assert_eq!(
            parse_runtime_ui_command("patch Cathedral Bloom").expect("patch command parses"),
            RuntimeUiCommand::Patch("Cathedral Bloom".to_string())
        );
        assert_eq!(
            parse_runtime_ui_command("macro gravitacija 0.74").expect("macro command parses"),
            RuntimeUiCommand::Macro(MacroId::Gravitacija, 0.74)
        );
        assert_eq!(
            parse_runtime_ui_command("audio Scarlett").expect("audio command parses"),
            RuntimeUiCommand::AudioSelect("Scarlett".to_string())
        );
        assert_eq!(
            parse_runtime_ui_command("midi 1").expect("midi command parses"),
            RuntimeUiCommand::MidiSelect("1".to_string())
        );
        assert_eq!(
            parse_runtime_ui_command("next").expect("next parses"),
            RuntimeUiCommand::NextFavorite
        );
        assert_eq!(
            parse_runtime_ui_command("demo-patch").expect("demo patch parses"),
            RuntimeUiCommand::DemoPatch
        );
    }

    #[test]
    fn factory_bank_has_productized_patch_set() {
        let entries = factory_patch_entries().expect("factory entries load");
        assert!(entries.len() >= 8);
        assert!(entries.iter().any(|entry| entry.favorite));
    }

    #[test]
    fn favorite_navigation_wraps_across_live_set() {
        let next = adjacent_favorite_patch(Path::new("patches/factory/razor-thaw.toml"), 1)
            .expect("favorite next resolves");
        assert!(next.ends_with("patches/factory/cathedral-bloom.toml"));

        let prev = adjacent_favorite_patch(Path::new("patches/factory/molten-horizon.toml"), -1)
            .expect("favorite prev resolves");
        assert!(prev.ends_with("patches/factory/ember-vault.toml"));
    }
}
