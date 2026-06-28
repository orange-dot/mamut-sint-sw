use std::env;
use std::fs::{File, create_dir_all};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process;

use mamut_dsp::StereoBlockMut;
use mamut_engine::{
    ControllerEvent, Engine, EngineConfig, GfmLayerMode, NoteEvent, ProcessBlock, Scheduled,
    ScheduledControllerEvent, ScheduledNoteEvent, select_gfm_program_for_patch,
};
use mamut_field::{GFM_PERFORMANCE_BASELINE_SEED, GfmProgramId, sample_to_pcm16};
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
const GFM_LIVE_SCENARIOS: [GfmLiveScenario; 3] = [
    GfmLiveScenario {
        slug: "low",
        amount: 0.34,
        pressure: 0.32,
    },
    GfmLiveScenario {
        slug: "mid",
        amount: 0.68,
        pressure: 0.62,
    },
    GfmLiveScenario {
        slug: "high",
        amount: 1.00,
        pressure: 0.90,
    },
];

#[derive(Debug, Clone, Copy)]
struct GfmLiveScenario {
    slug: &'static str,
    amount: f32,
    pressure: f32,
}

#[derive(Debug, Clone)]
struct RenderOptions {
    patch_slug: Option<String>,
    scenario: ScenarioFilter,
    quality: RenderQuality,
}

impl RenderOptions {
    fn from_args() -> std::io::Result<Self> {
        let mut patch_slug = None;
        let mut scenario = ScenarioFilter::All;
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
                "--scenario" => {
                    let value = args.next().ok_or_else(|| {
                        std::io::Error::other("--scenario requires base, low, mid, high, or all")
                    })?;
                    scenario = ScenarioFilter::parse(&value)?;
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
            scenario,
            quality,
        })
    }

    fn includes_patch(&self, slug: &str) -> bool {
        self.patch_slug
            .as_deref()
            .is_none_or(|filter| filter == slug)
    }

    fn baseline_path(&self, slug: &str) -> String {
        format!(
            "{OUTPUT_DIR}/{}_base_{slug}.wav",
            self.quality.file_prefix()
        )
    }

    fn scenario_path(&self, patch_slug: &str, scenario: GfmLiveScenario) -> String {
        format!(
            "{OUTPUT_DIR}/{}_gfm_{}_{}.wav",
            self.quality.file_prefix(),
            scenario.slug,
            patch_slug
        )
    }
}

#[derive(Debug, Clone, Copy)]
enum ScenarioFilter {
    All,
    Base,
    Low,
    Mid,
    High,
}

impl ScenarioFilter {
    fn parse(value: &str) -> std::io::Result<Self> {
        match value {
            "all" => Ok(Self::All),
            "base" | "baseline" => Ok(Self::Base),
            "low" => Ok(Self::Low),
            "mid" => Ok(Self::Mid),
            "high" => Ok(Self::High),
            _ => Err(std::io::Error::other(format!("unknown scenario: {value}"))),
        }
    }

    const fn includes_baseline(self) -> bool {
        matches!(self, Self::All | Self::Base)
    }

    fn includes_live(self, slug: &str) -> bool {
        match self {
            Self::All => true,
            Self::Low => slug == "low",
            Self::Mid => slug == "mid",
            Self::High => slug == "high",
            Self::Base => false,
        }
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
            Self::Smoke => "ab",
            Self::Listen => "ab_listen",
        }
    }
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
        if options.scenario.includes_baseline() {
            render_wav(
                options.baseline_path(slug),
                patch.clone(),
                GfmLayerMode::Disabled,
                "baseline",
                None,
                options.quality.sample_rate_hz(),
                options.quality.render_seconds(),
            )?;
        }
        for scenario in GFM_LIVE_SCENARIOS {
            if !options.scenario.includes_live(scenario.slug) {
                continue;
            }
            render_wav(
                options.scenario_path(slug, scenario),
                patch.clone(),
                GfmLayerMode::Enabled {
                    seed: GFM_PERFORMANCE_BASELINE_SEED,
                },
                scenario.slug,
                Some(scenario),
                options.quality.sample_rate_hz(),
                options.quality.render_seconds(),
            )?;
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
        "usage: cargo run --locked -p mamut-engine --example gfm_engine_layer_ab_render -- [--patch cathedral-bloom|ember-vault|razor-thaw] [--scenario base|low|mid|high|all] [--quality smoke|listen]"
    );
}

fn render_wav(
    path: impl AsRef<Path>,
    patch: PatchFileV1,
    mode: GfmLayerMode,
    mode_label: &str,
    live_scenario: Option<GfmLiveScenario>,
    sample_rate_hz: u32,
    render_seconds: usize,
) -> std::io::Result<()> {
    let path = path.as_ref();
    let patch_name = patch.meta.patch_name.clone();
    let patch_selection = select_gfm_program_for_patch(&patch);
    let Some(program_id) = patch_selection.program_id else {
        return Err(std::io::Error::other(format!(
            "patch did not select a GFM program: {patch_name}"
        )));
    };
    let mut engine = Engine::new(
        EngineConfig {
            sample_rate_hz: sample_rate_hz as f32,
            max_block_frames: BLOCK_FRAMES,
            voice_count: 6,
        },
        patch,
    )
    .map_err(std::io::Error::other)?;
    let layer_selection = engine.set_gfm_layer_mode(mode);
    if matches!(mode, GfmLayerMode::Enabled { .. })
        && layer_selection.program_id != Some(program_id)
    {
        return Err(std::io::Error::other(format!(
            "GFM layer selected {:?}, expected {:?} for {patch_name}",
            layer_selection.program_id,
            Some(program_id)
        )));
    }

    let note_on_events = layer_note_on_events(program_id);
    let controller_events = live_scenario.map(live_controller_events);
    let frames = sample_rate_hz as usize * render_seconds;
    let release_frame = sample_rate_hz as usize * (render_seconds - 1);
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    write_wav_header(&mut writer, sample_rate_hz, frames)?;

    let mut left = [0.0_f32; BLOCK_FRAMES];
    let mut right = [0.0_f32; BLOCK_FRAMES];
    let mut rendered = 0;
    let mut sum_squares = 0.0_f32;
    let mut peak_abs = 0.0_f32;
    let mut finite = true;

    while rendered < frames {
        let frame_count = (frames - rendered).min(BLOCK_FRAMES);

        if rendered == 0 {
            process_block(
                &mut engine,
                frame_count,
                &note_on_events,
                controller_events
                    .as_ref()
                    .map(|events| &events[..])
                    .unwrap_or(&[]),
                &mut left,
                &mut right,
            );
        } else if rendered <= release_frame && release_frame < rendered + frame_count {
            let frame_offset = release_frame - rendered;
            let note_events = layer_note_off_events(program_id, frame_offset);
            process_block(
                &mut engine,
                frame_count,
                &note_events,
                &[],
                &mut left,
                &mut right,
            );
        } else {
            process_block(&mut engine, frame_count, &[], &[], &mut left, &mut right);
        }

        let block = StereoBlockMut::new(&mut left[..frame_count], &mut right[..frame_count]);
        finite &= block.is_finite();
        peak_abs = peak_abs.max(block.peak_abs());
        sum_squares += block.rms().powi(2) * (block.frames() * 2) as f32;
        for index in 0..block.frames() {
            writer.write_all(&sample_to_pcm16(block.left[index]).to_le_bytes())?;
            writer.write_all(&sample_to_pcm16(block.right[index]).to_le_bytes())?;
        }

        rendered += frame_count;
    }
    writer.flush()?;

    if !finite {
        return Err(std::io::Error::other(
            "GFM engine layer A/B render produced non-finite output",
        ));
    }

    let rms = (sum_squares / (frames * 2) as f32).sqrt();
    let snapshot = engine.snapshot().gfm_layer;
    if let Some(diagnostics) = snapshot.diagnostics {
        let controls = snapshot
            .controls
            .map(|controls| {
                format!(
                    "depth={:.2} heat={:.2} spread={:.2} rupture={:.2} recovery={:.2} motion={:.2} body={:.2} brightness={:.2}",
                    controls.depth,
                    controls.heat,
                    controls.spread,
                    controls.rupture,
                    controls.recovery,
                    controls.motion,
                    controls.body,
                    controls.brightness
                )
            })
            .unwrap_or_else(|| "controls=none".to_string());
        println!(
            "{} mode={} patch=\"{}\" program={:?} active={:?} live={:?} scores=({:.4},{:.4},{:.4}) {} rms={:.4} peak={:.4} max_ruptures={} health={:?}",
            path.display(),
            mode_label,
            patch_name,
            program_id,
            snapshot.active_program_id,
            live_scenario.map(|scenario| (scenario.amount, scenario.pressure)),
            snapshot.selection.horizont_score,
            snapshot.selection.pec_score,
            snapshot.selection.baklja_score,
            controls,
            rms,
            peak_abs,
            diagnostics.max_rupture_count,
            diagnostics.health
        );
    } else {
        println!(
            "{} mode={} patch=\"{}\" program={:?} active={:?} live={:?} scores=({:.4},{:.4},{:.4}) rms={:.4} peak={:.4}",
            path.display(),
            mode_label,
            patch_name,
            program_id,
            snapshot.active_program_id,
            live_scenario.map(|scenario| (scenario.amount, scenario.pressure)),
            patch_selection.horizont_score,
            patch_selection.pec_score,
            patch_selection.baklja_score,
            rms,
            peak_abs
        );
    }

    Ok(())
}

fn live_controller_events(scenario: GfmLiveScenario) -> [ScheduledControllerEvent; 2] {
    [
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::GfmLayerAmount {
                amount: scenario.amount,
            },
        },
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::ChannelAftertouch {
                pressure: scenario.pressure,
            },
        },
    ]
}

fn layer_note_on_events(program_id: GfmProgramId) -> [ScheduledNoteEvent; 3] {
    let [low, middle, high] = layer_notes(program_id);
    [
        Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: low,
                velocity: 0.78,
            },
        },
        Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: middle,
                velocity: 0.70,
            },
        },
        Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: high,
                velocity: 0.64,
            },
        },
    ]
}

fn layer_note_off_events(program_id: GfmProgramId, frame_offset: usize) -> [ScheduledNoteEvent; 3] {
    let [low, middle, high] = layer_notes(program_id);
    [
        Scheduled {
            frame_offset,
            event: NoteEvent::NoteOff { note: low },
        },
        Scheduled {
            frame_offset,
            event: NoteEvent::NoteOff { note: middle },
        },
        Scheduled {
            frame_offset,
            event: NoteEvent::NoteOff { note: high },
        },
    ]
}

fn layer_notes(program_id: GfmProgramId) -> [u8; 3] {
    match program_id {
        GfmProgramId::HorizontPerformance => [48, 55, 60],
        GfmProgramId::PecPerformance => [60, 67, 72],
        GfmProgramId::BakljaPerformance => [48, 55, 60],
    }
}

fn process_block(
    engine: &mut Engine,
    frame_count: usize,
    note_events: &[ScheduledNoteEvent],
    controller_events: &[ScheduledControllerEvent],
    left: &mut [f32; BLOCK_FRAMES],
    right: &mut [f32; BLOCK_FRAMES],
) {
    engine.process_block(ProcessBlock {
        frame_count,
        note_events,
        controller_events,
        macro_state: None,
        output: Some(StereoBlockMut::new(
            &mut left[..frame_count],
            &mut right[..frame_count],
        )),
    });
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
