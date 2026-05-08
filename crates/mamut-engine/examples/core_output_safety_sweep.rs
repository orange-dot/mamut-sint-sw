use mamut_dsp::{MASTER_SAFETY_CEILING, StereoBlockMut};
use mamut_engine::{
    BcsLayerMode, BcsScenario, ControllerEvent, Engine, EngineConfig, GfmLayerMode, NoteEvent,
    OutputSafetySnapshot, ProcessBlock, Scheduled, ScheduledControllerEvent, ScheduledNoteEvent,
    select_gfm_program_for_patch,
};
use mamut_field::{GFM_PERFORMANCE_BASELINE_SEED, GfmProgramId};
use mamut_identity::MacroState;
use mamut_patch::{PatchFileV1, load_patch_toml};

const SAMPLE_RATE_HZ: u32 = 48_000;
const BLOCK_FRAMES: usize = 256;
const DURATION_SECONDS: usize = 2;
const FACTORY_PATCHES: [(&str, &str); 9] = [
    (
        "cathedral-bloom",
        include_str!("../../../patches/factory/cathedral-bloom.toml"),
    ),
    (
        "ember-vault",
        include_str!("../../../patches/factory/ember-vault.toml"),
    ),
    (
        "furnace-choir",
        include_str!("../../../patches/factory/furnace-choir.toml"),
    ),
    (
        "glass-tide",
        include_str!("../../../patches/factory/glass-tide.toml"),
    ),
    (
        "granite-plain",
        include_str!("../../../patches/factory/granite-plain.toml"),
    ),
    (
        "gravity-wake",
        include_str!("../../../patches/factory/gravity-wake.toml"),
    ),
    (
        "molten-horizon",
        include_str!("../../../patches/factory/molten-horizon.toml"),
    ),
    (
        "razor-thaw",
        include_str!("../../../patches/factory/razor-thaw.toml"),
    ),
    (
        "sawyer-rezz",
        include_str!("../../../patches/factory/sawyer-rezz.toml"),
    ),
];

const SCENARIOS: [SweepScenario; 5] = [
    SweepScenario::SingleNote,
    SweepScenario::DenseChord,
    SweepScenario::MacroPressure,
    SweepScenario::GfmLayer,
    SweepScenario::BcsLayer,
];

#[derive(Debug, Clone, Copy)]
enum SweepScenario {
    SingleNote,
    DenseChord,
    MacroPressure,
    GfmLayer,
    BcsLayer,
}

impl SweepScenario {
    const fn label(self) -> &'static str {
        match self {
            Self::SingleNote => "single-note",
            Self::DenseChord => "dense-chord",
            Self::MacroPressure => "macro-pressure",
            Self::GfmLayer => "gfm-layer",
            Self::BcsLayer => "bcs-layer",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct SweepStats {
    rms: f32,
    finite: bool,
    safety: OutputSafetySnapshot,
}

fn main() -> std::io::Result<()> {
    println!("patch,scenario,rms,pre_peak,post_peak,limiter_hits,max_reduction,tiny_flush,finite");

    let mut safe = true;
    for (patch_slug, patch_source) in FACTORY_PATCHES {
        let patch = load_patch_toml(patch_source).map_err(std::io::Error::other)?;
        for scenario in SCENARIOS {
            let stats = render_scenario(patch.clone(), scenario).map_err(std::io::Error::other)?;
            safe &= stats.finite && stats.safety.post_safety_peak <= MASTER_SAFETY_CEILING;
            println!(
                "{},{},{:.6},{:.6},{:.6},{},{:.6},{},{}",
                patch_slug,
                scenario.label(),
                stats.rms,
                stats.safety.pre_safety_peak,
                stats.safety.post_safety_peak,
                stats.safety.safety_limiter_hits,
                stats.safety.max_safety_reduction,
                stats.safety.tiny_flush_events,
                stats.finite
            );
        }
    }

    if safe {
        Ok(())
    } else {
        Err(std::io::Error::other(
            "core output safety sweep found non-finite or over-ceiling output",
        ))
    }
}

fn render_scenario(
    patch: PatchFileV1,
    scenario: SweepScenario,
) -> Result<SweepStats, mamut_patch::PatchValidationError> {
    let initial_note_events = note_events_for_scenario(&patch, scenario);
    let initial_controller_events = controller_events_for_scenario(scenario);
    let initial_macro_state = macro_state_for_scenario(scenario);
    let frames = SAMPLE_RATE_HZ as usize * DURATION_SECONDS;
    let mut engine = Engine::new(
        EngineConfig {
            sample_rate_hz: SAMPLE_RATE_HZ as f32,
            max_block_frames: BLOCK_FRAMES,
            voice_count: 6,
        },
        patch,
    )?;

    match scenario {
        SweepScenario::GfmLayer => {
            engine.set_gfm_layer_mode(GfmLayerMode::Enabled {
                seed: GFM_PERFORMANCE_BASELINE_SEED,
            });
        }
        SweepScenario::BcsLayer => {
            engine.set_bcs_layer_mode(BcsLayerMode::Enabled {
                scenario: BcsScenario::SubharmonicPressure,
            });
        }
        SweepScenario::SingleNote | SweepScenario::DenseChord | SweepScenario::MacroPressure => {}
    }

    let mut left = [0.0_f32; BLOCK_FRAMES];
    let mut right = [0.0_f32; BLOCK_FRAMES];
    let mut rendered = 0_usize;
    let mut finite = true;
    let mut sum_squares = 0.0_f32;
    let mut safety = OutputSafetySnapshot::default();

    while rendered < frames {
        let frame_count = (frames - rendered).min(BLOCK_FRAMES);
        let note_events = if rendered == 0 {
            initial_note_events.as_slice()
        } else {
            &[]
        };
        let controller_events = if rendered == 0 {
            initial_controller_events.as_slice()
        } else {
            &[]
        };
        let macro_state = if rendered == 0 {
            initial_macro_state
        } else {
            None
        };

        engine.process_block(ProcessBlock {
            frame_count,
            note_events,
            controller_events,
            macro_state,
            output: Some(StereoBlockMut::new(
                &mut left[..frame_count],
                &mut right[..frame_count],
            )),
        });

        let block = StereoBlockMut::new(&mut left[..frame_count], &mut right[..frame_count]);
        finite &= block.is_finite();
        sum_squares += block.rms().powi(2) * (block.frames() * 2) as f32;
        accumulate_safety(&mut safety, engine.snapshot().output_safety);

        rendered += frame_count;
    }

    Ok(SweepStats {
        rms: (sum_squares / (frames * 2) as f32).sqrt(),
        finite,
        safety,
    })
}

fn accumulate_safety(total: &mut OutputSafetySnapshot, block: OutputSafetySnapshot) {
    total.pre_safety_peak = total.pre_safety_peak.max(block.pre_safety_peak);
    total.post_safety_peak = total.post_safety_peak.max(block.post_safety_peak);
    total.safety_limiter_hits += block.safety_limiter_hits;
    total.max_safety_reduction = total.max_safety_reduction.max(block.max_safety_reduction);
    total.tiny_flush_events += block.tiny_flush_events;
}

fn note_events_for_scenario(
    patch: &PatchFileV1,
    scenario: SweepScenario,
) -> Vec<ScheduledNoteEvent> {
    let notes: &[u8] = match scenario {
        SweepScenario::SingleNote => &[48],
        SweepScenario::DenseChord | SweepScenario::MacroPressure => &[36, 43, 48, 55, 60, 67],
        SweepScenario::GfmLayer => {
            return layer_notes(select_gfm_program_for_patch(patch).program_id)
                .into_iter()
                .map(note_on)
                .collect();
        }
        SweepScenario::BcsLayer => &[48, 55, 60],
    };
    notes.iter().copied().map(note_on).collect()
}

fn note_on(note: u8) -> ScheduledNoteEvent {
    Scheduled {
        frame_offset: 0,
        event: NoteEvent::NoteOn {
            note,
            velocity: 0.88,
        },
    }
}

fn controller_events_for_scenario(scenario: SweepScenario) -> Vec<ScheduledControllerEvent> {
    match scenario {
        SweepScenario::GfmLayer => vec![
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::GfmLayerAmount { amount: 0.85 },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::ChannelAftertouch { pressure: 1.0 },
            },
        ],
        SweepScenario::BcsLayer => vec![
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::BcsLayerEnabled { enabled: true },
            },
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::BcsLayerAmount { amount: 0.80 },
            },
        ],
        SweepScenario::SingleNote | SweepScenario::DenseChord | SweepScenario::MacroPressure => {
            Vec::new()
        }
    }
}

fn macro_state_for_scenario(scenario: SweepScenario) -> Option<MacroState> {
    match scenario {
        SweepScenario::MacroPressure => Some(MacroState {
            gravitacija: 1.0,
            bloom: 0.45,
            heat: 1.0,
            ruin: 0.85,
            swarm: 0.70,
        }),
        SweepScenario::SingleNote
        | SweepScenario::DenseChord
        | SweepScenario::GfmLayer
        | SweepScenario::BcsLayer => None,
    }
}

fn layer_notes(program_id: Option<GfmProgramId>) -> [u8; 3] {
    match program_id {
        Some(GfmProgramId::PecPerformance) => [60, 67, 72],
        Some(GfmProgramId::HorizontPerformance | GfmProgramId::BakljaPerformance) | None => {
            [48, 55, 60]
        }
    }
}
