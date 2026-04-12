use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
    time::Duration,
};

use anyhow::{Context, Result, anyhow};
use cpal::{
    FromSample, SampleFormat, SizedSample, Stream, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use mamut_dsp::StereoBlockMut;
use mamut_engine::{ControllerEvent, Engine, EngineConfig, NoteEvent, ProcessBlock, Scheduled};
use mamut_params::MacroId;
use mamut_patch::{PatchFileV1, load_patch_toml, validate_patch_v1};
use midir::{Ignore, MidiInput, MidiInputConnection};

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();

    match args.first().map(String::as_str) {
        Some("list-factory") => list_factory_patches(),
        Some("validate") => {
            let path = args
                .get(1)
                .map(PathBuf::from)
                .unwrap_or_else(default_patch_path);
            let patch = load_patch_from_path(&path)?;
            validate_patch_v1(&patch).context("patch validation failed")?;
            println!("valid: {}", path.display());
            Ok(())
        }
        Some("dry-run") => {
            let path = args
                .get(1)
                .map(PathBuf::from)
                .unwrap_or_else(default_patch_path);
            dry_run(&path)
        }
        Some("play") => {
            let path = args
                .iter()
                .skip(1)
                .find(|arg| !arg.starts_with("--"))
                .map(PathBuf::from)
                .unwrap_or_else(default_patch_path);
            let force_demo = args.iter().any(|arg| arg == "--demo");
            play(&path, force_demo)
        }
        None => dry_run(&default_patch_path()),
        Some(other) => Err(anyhow!(
            "unknown command `{other}`; expected `list-factory`, `validate`, `dry-run`, or `play`"
        )),
    }
}

fn list_factory_patches() -> Result<()> {
    let factory_dir = workspace_root().join("patches/factory");
    for entry in fs::read_dir(&factory_dir).with_context(|| {
        format!(
            "failed to read factory patch directory {}",
            factory_dir.display()
        )
    })? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            println!("{}", entry.path().display());
        }
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
    ];

    engine.process_block(ProcessBlock {
        frame_count: 256,
        note_events: &note_events,
        controller_events: &controller_events,
        macro_state: None,
        output: Some(StereoBlockMut::new(&mut left, &mut right)),
    });

    let snapshot = engine.snapshot();
    println!("patch: {}", path.display());
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

fn play(path: &Path, force_demo: bool) -> Result<()> {
    let patch = load_patch_from_path(path)?;
    validate_patch_v1(&patch).context("patch validation failed")?;

    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .context("no default output device available")?;
    let supported_config = device
        .default_output_config()
        .context("failed to query default output config")?;
    let sample_format = supported_config.sample_format();
    let stream_config: StreamConfig = supported_config.clone().into();
    let sample_rate_hz = stream_config.sample_rate.0 as f32;
    let channels = usize::from(stream_config.channels);
    let bend_range = patch.performance_response.bend_range_semitones as f32;

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

    let midi_connection = open_first_midi_input(tx.clone(), bend_range)?;
    let using_demo = force_demo || midi_connection.is_none();
    if using_demo {
        spawn_demo_performance(tx.clone());
    }

    println!("play patch: {}", path.display());
    println!(
        "audio: {} @ {} Hz, {} channels",
        device
            .name()
            .unwrap_or_else(|_| "unknown-device".to_string()),
        stream_config.sample_rate.0,
        channels
    );
    if let Some(connection) = midi_connection.as_ref() {
        println!("midi: connected ({})", connection.port_name);
    } else {
        println!("midi: no input connected, running demo performer");
    }
    println!(
        "cc map: 1=mod wheel, 64=sustain, 71=heat, 73=bloom, 74=gravitacija, 75=ruin, 76=swarm"
    );
    println!("press Ctrl+C to stop");

    stream.play().context("failed to start output stream")?;
    keep_running_forever(stream, midi_connection);
    Ok(())
}

fn keep_running_forever(_stream: Stream, _midi: Option<OpenedMidiConnection>) {
    loop {
        thread::sleep(Duration::from_secs(1));
    }
}

fn load_patch_from_path(path: &Path) -> Result<PatchFileV1> {
    let input = fs::read_to_string(path)
        .with_context(|| format!("failed to read patch file {}", path.display()))?;
    load_patch_toml(&input)
        .with_context(|| format!("failed to parse patch file {}", path.display()))
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

#[derive(Debug, Clone, Copy)]
enum RuntimeCommand {
    Note(NoteEvent),
    Controller(ControllerEvent),
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

fn open_first_midi_input(
    tx: mpsc::Sender<RuntimeCommand>,
    bend_range: f32,
) -> Result<Option<OpenedMidiConnection>> {
    let mut midi_input =
        MidiInput::new("mamut-standalone").context("failed to create MIDI input")?;
    midi_input.ignore(Ignore::None);
    let ports = midi_input.ports();
    let Some(port) = ports.first() else {
        return Ok(None);
    };

    let port_name = midi_input
        .port_name(port)
        .unwrap_or_else(|_| "unknown-midi-port".to_string());
    let connection = midi_input
        .connect(
            port,
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

fn spawn_demo_performance(tx: mpsc::Sender<RuntimeCommand>) {
    thread::spawn(move || {
        let notes = [36_u8, 43, 48, 55, 60, 67];
        let macro_cycle = [
            (MacroId::Bloom, 0.65),
            (MacroId::Heat, 0.58),
            (MacroId::Gravitacija, 0.52),
            (MacroId::Ruin, 0.38),
            (MacroId::Swarm, 0.44),
        ];

        let mut step = 0_usize;
        loop {
            let note = notes[step % notes.len()];
            let (macro_id, macro_value) = macro_cycle[step % macro_cycle.len()];
            let velocity = 0.62 + (step % 3) as f32 * 0.10;

            let _ = tx.send(RuntimeCommand::Controller(ControllerEvent::Macro {
                id: macro_id,
                value: macro_value,
            }));
            let _ = tx.send(RuntimeCommand::Controller(ControllerEvent::ModWheel {
                amount: ((step % 5) as f32) / 5.0,
            }));
            let _ = tx.send(RuntimeCommand::Note(NoteEvent::NoteOn { note, velocity }));
            thread::sleep(Duration::from_millis(420));
            let _ = tx.send(RuntimeCommand::Controller(
                ControllerEvent::ChannelAftertouch {
                    pressure: 0.20 + ((step % 4) as f32) * 0.12,
                },
            ));
            thread::sleep(Duration::from_millis(360));
            let _ = tx.send(RuntimeCommand::Note(NoteEvent::NoteOff { note }));
            thread::sleep(Duration::from_millis(140));
            step = step.wrapping_add(1);
        }
    });
}
