use std::fs::{File, create_dir_all};
use std::io::{BufWriter, Write};
use std::path::Path;

use mamut_dsp::StereoBlockMut;
use mamut_engine::{
    Engine, EngineConfig, GfmLayerMode, NoteEvent, ProcessBlock, Scheduled, ScheduledNoteEvent,
};
use mamut_field::{
    GFM_PERFORMANCE_BASELINE_SEED, GFM_PERFORMANCE_DURATION_SECONDS, GfmProgramId, sample_to_pcm16,
};
use mamut_patch::{PatchFileV1, load_patch_toml};

const SAMPLE_RATE_HZ: u32 = 48_000;
const BLOCK_FRAMES: usize = 256;
const OUTPUT_DIR: &str = "target/gfm-render";
const LAYER_PATCHES: [(&str, &str); 3] = [
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

fn main() -> std::io::Result<()> {
    create_dir_all(OUTPUT_DIR)?;

    for (slug, patch_source) in LAYER_PATCHES {
        let patch = load_patch_toml(patch_source).map_err(std::io::Error::other)?;
        render_wav(format!("{OUTPUT_DIR}/layer_gfm_{slug}.wav"), patch)?;
    }

    Ok(())
}

fn render_wav(path: impl AsRef<Path>, patch: PatchFileV1) -> std::io::Result<()> {
    let path = path.as_ref();
    let patch_name = patch.meta.patch_name.clone();
    let mut engine = Engine::new(
        EngineConfig {
            sample_rate_hz: SAMPLE_RATE_HZ as f32,
            max_block_frames: BLOCK_FRAMES,
            voice_count: 6,
        },
        patch,
    )
    .map_err(std::io::Error::other)?;
    let selection = engine.set_gfm_layer_mode(GfmLayerMode::Enabled {
        seed: GFM_PERFORMANCE_BASELINE_SEED,
    });
    let Some(program_id) = selection.program_id else {
        return Err(std::io::Error::other(format!(
            "patch did not select a GFM layer program: {patch_name}"
        )));
    };
    let note_on_events = layer_note_on_events(program_id);

    let frames = SAMPLE_RATE_HZ as usize * GFM_PERFORMANCE_DURATION_SECONDS;
    let release_frame = SAMPLE_RATE_HZ as usize * 11;
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    write_wav_header(&mut writer, SAMPLE_RATE_HZ, frames)?;

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
                &mut left,
                &mut right,
            );
        } else {
            process_block(&mut engine, frame_count, &[], &mut left, &mut right);
        }

        for index in 0..frame_count {
            let left_sample = left[index];
            let right_sample = right[index];
            finite &= left_sample.is_finite() && right_sample.is_finite();
            peak_abs = peak_abs.max(left_sample.abs().max(right_sample.abs()));
            sum_squares += left_sample * left_sample + right_sample * right_sample;
            writer.write_all(&sample_to_pcm16(left_sample).to_le_bytes())?;
            writer.write_all(&sample_to_pcm16(right_sample).to_le_bytes())?;
        }

        rendered += frame_count;
    }
    writer.flush()?;

    if !finite {
        return Err(std::io::Error::other(
            "GFM engine layer render produced non-finite output",
        ));
    }

    let rms = (sum_squares / (frames * 2) as f32).sqrt();
    let diagnostics = engine
        .gfm_layer_diagnostics()
        .expect("enabled GFM layer should keep diagnostics");
    println!(
        "{} patch=\"{}\" program={:?} scores=({:.4},{:.4},{:.4}) rms={:.4} peak={:.4} max_ruptures={} health={:?}",
        path.display(),
        patch_name,
        program_id,
        selection.horizont_score,
        selection.pec_score,
        selection.baklja_score,
        rms,
        peak_abs,
        diagnostics.max_rupture_count,
        diagnostics.health
    );

    Ok(())
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
    left: &mut [f32; BLOCK_FRAMES],
    right: &mut [f32; BLOCK_FRAMES],
) {
    engine.process_block(ProcessBlock {
        frame_count,
        note_events,
        controller_events: &[],
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
