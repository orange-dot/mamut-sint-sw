use std::env;
use std::fs::{File, create_dir_all};
use std::io::{BufWriter, Write};
use std::process;

use mamut_dsp::StereoBlockMut;
use mamut_engine::{
    ControllerEvent, Engine, EngineConfig, GfmLayerMode, NoteEvent, ProcessBlock, Scheduled,
    ScheduledControllerEvent, ScheduledNoteEvent, select_gfm_program_for_patch,
};
use mamut_field::{GFM_PERFORMANCE_BASELINE_SEED, GfmDiagnostics, GfmProgramId, sample_to_pcm16};
use mamut_patch::{PatchFileV1, load_patch_toml};

const BLOCK_FRAMES: usize = 256;
const OUTPUT_DIR: &str = "target/gfm-render";
const AB_PATCHES: [(&str, &str); 3] = [
    (
        "cathedral-bloom",
        include_str!("../../../patches/factory/cathedral-bloom.toml"),
    ),
    (
        "ember-vault",
        include_str!("../../../patches/factory/ember-vault.toml"),
    ),
    (
        "razor-thaw",
        include_str!("../../../patches/factory/razor-thaw.toml"),
    ),
];
const LIVE_AMOUNT: f32 = 0.68;
const LIVE_PRESSURE: f32 = 0.62;
const NOTE_PATTERN: [(u8, f32); 8] = [
    (36, 0.90),
    (43, 0.72),
    (52, 0.58),
    (58, 0.66),
    (67, 0.84),
    (79, 0.62),
    (72, 0.70),
    (65, 0.52),
];
const NOTE_INTERVAL_SECONDS: f32 = 0.30;
const NOTE_HOLD_SECONDS: f32 = 0.22;

#[derive(Debug, Clone)]
struct RenderOptions {
    patch_slug: Option<String>,
    quality: RenderQuality,
}

impl RenderOptions {
    fn from_args() -> std::io::Result<Self> {
        let mut patch_slug = None;
        let mut quality = RenderQuality::Smoke;
        let mut args = env::args().skip(1);

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--patch" => {
                    patch_slug =
                        Some(args.next().ok_or_else(|| {
                            std::io::Error::other("--patch requires a patch slug")
                        })?);
                }
                "--quality" => {
                    let value = args.next().ok_or_else(|| {
                        std::io::Error::other("--quality requires smoke or listen")
                    })?;
                    quality = RenderQuality::parse(&value)?;
                }
                "--help" | "-h" => {
                    print_usage();
                    process::exit(0);
                }
                _ => {
                    return Err(std::io::Error::other(format!("unknown argument: {arg}")));
                }
            }
        }

        Ok(Self {
            patch_slug,
            quality,
        })
    }

    fn includes_patch(&self, slug: &str) -> bool {
        self.patch_slug
            .as_deref()
            .is_none_or(|filter| filter == slug)
    }

    fn take_path(&self, take: &str, patch_slug: &str) -> String {
        format!(
            "{OUTPUT_DIR}/{}_{take}_{patch_slug}.wav",
            self.quality.file_prefix()
        )
    }
}

#[derive(Debug, Clone, Copy)]
enum RenderQuality {
    Smoke,
    Listen,
}

impl RenderQuality {
    fn parse(value: &str) -> std::io::Result<Self> {
        match value {
            "smoke" => Ok(Self::Smoke),
            "listen" => Ok(Self::Listen),
            _ => Err(std::io::Error::other(format!("unknown quality: {value}"))),
        }
    }

    const fn sample_rate_hz(self) -> u32 {
        match self {
            Self::Smoke => 8_000,
            Self::Listen => 48_000,
        }
    }

    const fn render_seconds(self) -> usize {
        match self {
            Self::Smoke => 3,
            Self::Listen => 6,
        }
    }

    const fn file_prefix(self) -> &'static str {
        match self {
            Self::Smoke => "strike",
            Self::Listen => "strike_listen",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ScriptedNote {
    on_frame: usize,
    off_frame: usize,
    note: u8,
    velocity: f32,
}

#[derive(Debug, Clone)]
struct TakeStats {
    signature: u64,
    rms: f32,
    peak_abs: f32,
    finite: bool,
    diagnostics: Option<GfmDiagnostics>,
    mono: Vec<f32>,
}

fn main() -> std::io::Result<()> {
    let options = RenderOptions::from_args()?;
    create_dir_all(OUTPUT_DIR)?;

    let mut matched_patch = false;
    for (slug, patch_source) in AB_PATCHES {
        if !options.includes_patch(slug) {
            continue;
        }
        matched_patch = true;
        let patch = load_patch_toml(patch_source).map_err(std::io::Error::other)?;
        let Some(program_id) = select_gfm_program_for_patch(&patch).program_id else {
            return Err(std::io::Error::other(format!(
                "patch did not select a GFM program: {slug}"
            )));
        };

        let center = render_take(
            &options,
            patch.clone(),
            slug,
            "center",
            false,
            Some(options.take_path("center", slug)),
        )?;
        let strikes = render_take(
            &options,
            patch.clone(),
            slug,
            "notes",
            true,
            Some(options.take_path("notes", slug)),
        )?;
        let repeat = render_take(&options, patch, slug, "notes-repeat", true, None)?;

        if !center.finite || !strikes.finite {
            return Err(std::io::Error::other(format!(
                "note-strike A/B render produced non-finite output: {slug}"
            )));
        }
        if strikes.signature == center.signature {
            return Err(std::io::Error::other(format!(
                "note strikes did not change the PCM signature against the center-only baseline: {slug}"
            )));
        }
        if strikes.signature != repeat.signature {
            return Err(std::io::Error::other(format!(
                "note-strike render is not deterministic across runs: {slug}"
            )));
        }
        let (diff_rms, correlation) = pair_metrics(&center.mono, &strikes.mono);
        println!(
            "pair patch={slug} center_vs_notes diff_rms={diff_rms:.6} correlation={correlation:.6}"
        );
        let strike_diagnostics = strikes.diagnostics.ok_or_else(|| {
            std::io::Error::other(format!("armed layer reported no diagnostics: {slug}"))
        })?;
        match program_id {
            GfmProgramId::HorizontPerformance | GfmProgramId::PecPerformance => {
                if strike_diagnostics.max_rupture_count != 0 {
                    return Err(std::io::Error::other(format!(
                        "{program_id:?} note-strike take must stay rupture-free: {slug}"
                    )));
                }
            }
            GfmProgramId::BakljaPerformance => {
                if strike_diagnostics.max_rupture_count > 102 {
                    return Err(std::io::Error::other(format!(
                        "Baklja note-strike rupture count exceeded bound: {}",
                        strike_diagnostics.max_rupture_count
                    )));
                }
            }
        }
    }
    if !matched_patch {
        return Err(std::io::Error::other(format!(
            "unknown patch slug: {}",
            options.patch_slug.unwrap_or_default()
        )));
    }

    Ok(())
}

fn print_usage() {
    println!(
        "usage: cargo run --locked -p mamut-engine --example gfm_engine_note_strike_ab_render -- [--patch cathedral-bloom|ember-vault|razor-thaw] [--quality smoke|listen]"
    );
}

fn scripted_notes(sample_rate_hz: u32, render_seconds: usize) -> Vec<ScriptedNote> {
    let interval = (sample_rate_hz as f32 * NOTE_INTERVAL_SECONDS) as usize;
    let hold = (sample_rate_hz as f32 * NOTE_HOLD_SECONDS) as usize;
    let total = sample_rate_hz as usize * render_seconds;
    let release_tail = sample_rate_hz as usize / 2;
    let mut notes = Vec::new();
    let mut index = 0_usize;

    loop {
        let on_frame = index * interval.max(1);
        let off_frame = on_frame + hold;
        if off_frame + release_tail >= total {
            break;
        }
        let (note, velocity) = NOTE_PATTERN[index % NOTE_PATTERN.len()];
        notes.push(ScriptedNote {
            on_frame,
            off_frame,
            note,
            velocity,
        });
        index += 1;
    }
    notes
}

fn block_note_events(
    notes: &[ScriptedNote],
    block_start: usize,
    frame_count: usize,
) -> Vec<ScheduledNoteEvent> {
    let block_range = block_start..block_start + frame_count;
    let mut events = Vec::new();
    for scripted in notes {
        if block_range.contains(&scripted.on_frame) {
            events.push(Scheduled {
                frame_offset: scripted.on_frame - block_start,
                event: NoteEvent::NoteOn {
                    note: scripted.note,
                    velocity: scripted.velocity,
                },
            });
        }
        if block_range.contains(&scripted.off_frame) {
            events.push(Scheduled {
                frame_offset: scripted.off_frame - block_start,
                event: NoteEvent::NoteOff {
                    note: scripted.note,
                },
            });
        }
    }
    events.sort_by_key(|event| event.frame_offset);
    events
}

fn render_take(
    options: &RenderOptions,
    patch: PatchFileV1,
    patch_slug: &str,
    take_label: &str,
    note_strikes_enabled: bool,
    wav_path: Option<String>,
) -> std::io::Result<TakeStats> {
    let sample_rate_hz = options.quality.sample_rate_hz();
    let render_seconds = options.quality.render_seconds();
    let frames = sample_rate_hz as usize * render_seconds;
    let notes = scripted_notes(sample_rate_hz, render_seconds);
    let controller_events: [ScheduledControllerEvent; 2] = [
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::GfmLayerAmount {
                amount: LIVE_AMOUNT,
            },
        },
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::ChannelAftertouch {
                pressure: LIVE_PRESSURE,
            },
        },
    ];

    let mut engine = Engine::new(
        EngineConfig {
            sample_rate_hz: sample_rate_hz as f32,
            max_block_frames: BLOCK_FRAMES,
            voice_count: 6,
        },
        patch,
    )
    .map_err(std::io::Error::other)?;
    engine.set_gfm_layer_mode(GfmLayerMode::Enabled {
        seed: GFM_PERFORMANCE_BASELINE_SEED,
    });
    engine.set_gfm_note_strikes_enabled(note_strikes_enabled);

    let mut writer = match &wav_path {
        Some(path) => {
            let file = File::create(path)?;
            let mut writer = BufWriter::new(file);
            write_wav_header(&mut writer, sample_rate_hz, frames)?;
            Some(writer)
        }
        None => None,
    };

    let mut left = [0.0_f32; BLOCK_FRAMES];
    let mut right = [0.0_f32; BLOCK_FRAMES];
    let mut rendered = 0;
    let mut signature = 0xcbf2_9ce4_8422_2325_u64;
    let mut sum_squares = 0.0_f32;
    let mut peak_abs = 0.0_f32;
    let mut finite = true;
    let mut mono = Vec::with_capacity(frames);

    while rendered < frames {
        let frame_count = (frames - rendered).min(BLOCK_FRAMES);
        let note_events = block_note_events(&notes, rendered, frame_count);
        let block_controller_events: &[ScheduledControllerEvent] = if rendered == 0 {
            &controller_events
        } else {
            &[]
        };

        engine.process_block(ProcessBlock {
            frame_count,
            note_events: &note_events,
            controller_events: block_controller_events,
            macro_state: None,
            output: Some(StereoBlockMut::new(
                &mut left[..frame_count],
                &mut right[..frame_count],
            )),
        });

        let block = StereoBlockMut::new(&mut left[..frame_count], &mut right[..frame_count]);
        finite &= block.is_finite();
        peak_abs = peak_abs.max(block.peak_abs());
        sum_squares += block.rms().powi(2) * (block.frames() * 2) as f32;
        for index in 0..block.frames() {
            mono.push((block.left[index] + block.right[index]) * 0.5);
            for pcm in [
                sample_to_pcm16(block.left[index]),
                sample_to_pcm16(block.right[index]),
            ] {
                signature ^= pcm as u16 as u64;
                signature = signature.wrapping_mul(0x100_0000_01b3);
                if let Some(writer) = writer.as_mut() {
                    writer.write_all(&pcm.to_le_bytes())?;
                }
            }
        }

        rendered += frame_count;
    }
    if let Some(writer) = writer.as_mut() {
        writer.flush()?;
    }

    let rms = (sum_squares / (frames * 2) as f32).sqrt();
    let snapshot = engine.snapshot().gfm_layer;
    let stats = TakeStats {
        signature,
        rms,
        peak_abs,
        finite,
        diagnostics: snapshot.diagnostics,
        mono,
    };

    let path_label = wav_path.unwrap_or_else(|| "(signature only)".to_string());
    if let Some(diagnostics) = snapshot.diagnostics {
        println!(
            "{path_label} take={take_label} patch={patch_slug} program={:?} strikes_enabled={} live=({LIVE_AMOUNT:.2},{LIVE_PRESSURE:.2}) rms={:.4} peak={:.4} max_ruptures={} strike_count={} health={:?} signature={:016x}",
            snapshot.active_program_id,
            snapshot.note_strikes_enabled,
            stats.rms,
            stats.peak_abs,
            diagnostics.max_rupture_count,
            diagnostics.strike_count,
            diagnostics.health,
            stats.signature
        );
    } else {
        println!(
            "{path_label} take={take_label} patch={patch_slug} program={:?} strikes_enabled={} rms={:.4} peak={:.4} signature={:016x}",
            snapshot.active_program_id,
            snapshot.note_strikes_enabled,
            stats.rms,
            stats.peak_abs,
            stats.signature
        );
    }

    Ok(stats)
}

fn pair_metrics(left: &[f32], right: &[f32]) -> (f32, f32) {
    let frames = left.len().min(right.len()).max(1);
    let mut left_power = 0.0_f32;
    let mut right_power = 0.0_f32;
    let mut cross_power = 0.0_f32;
    let mut diff_power = 0.0_f32;

    for index in 0..frames {
        let left_sample = left[index];
        let right_sample = right[index];
        left_power += left_sample * left_sample;
        right_power += right_sample * right_sample;
        cross_power += left_sample * right_sample;
        let diff = left_sample - right_sample;
        diff_power += diff * diff;
    }

    let diff_rms = (diff_power / frames as f32).sqrt();
    let denominator = (left_power * right_power).sqrt().max(0.000_001);
    (diff_rms, cross_power / denominator)
}

fn write_wav_header(
    writer: &mut impl Write,
    sample_rate_hz: u32,
    frames: usize,
) -> std::io::Result<()> {
    let data_len = (frames as u32) * 4;
    let byte_rate = sample_rate_hz * 4;

    writer.write_all(b"RIFF")?;
    writer.write_all(&(36 + data_len).to_le_bytes())?;
    writer.write_all(b"WAVE")?;
    writer.write_all(b"fmt ")?;
    writer.write_all(&16_u32.to_le_bytes())?;
    writer.write_all(&1_u16.to_le_bytes())?;
    writer.write_all(&2_u16.to_le_bytes())?;
    writer.write_all(&sample_rate_hz.to_le_bytes())?;
    writer.write_all(&byte_rate.to_le_bytes())?;
    writer.write_all(&4_u16.to_le_bytes())?;
    writer.write_all(&16_u16.to_le_bytes())?;
    writer.write_all(b"data")?;
    writer.write_all(&data_len.to_le_bytes())?;

    Ok(())
}
