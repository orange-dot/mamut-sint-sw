use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow};
use mamut_engine::{ControllerEvent, Engine, EngineConfig, NoteEvent, ProcessBlock, Scheduled};
use mamut_patch::{PatchFileV1, load_patch_toml, validate_patch_v1};

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
        Some("dry-run") | None => {
            let path = args
                .get(1)
                .map(PathBuf::from)
                .unwrap_or_else(default_patch_path);
            dry_run(&path)
        }
        Some(other) => Err(anyhow!(
            "unknown command `{other}`; expected `list-factory`, `validate`, or `dry-run`"
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

    let note_events = [
        Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: 60,
                velocity: 0.88,
            },
        },
        Scheduled {
            frame_offset: 32,
            event: NoteEvent::NoteOff { note: 60 },
        },
    ];
    let controller_events = [
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::ModWheel { amount: 0.45 },
        },
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::ChannelAftertouch { pressure: 0.30 },
        },
    ];

    engine.process_block(ProcessBlock {
        frame_count: 128,
        note_events: &note_events,
        controller_events: &controller_events,
        macro_state: None,
        output: None,
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
        "direct: cutoff_hz={:.2} sync={:.3} crossmod={:.3} body_drive={:.3}",
        snapshot.direct.cutoff_hz,
        snapshot.direct.sync_amount,
        snapshot.direct.crossmod_amount,
        snapshot.direct.body_drive
    );

    Ok(())
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
