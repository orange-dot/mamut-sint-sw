use std::fs::{File, create_dir_all};
use std::io::{BufWriter, Write};
use std::path::Path;

use mamut_engine::GfmFieldVoice;
use mamut_field::{GFM_PERFORMANCE_BASELINE_SEED, GfmProgramId, sample_to_pcm16};

const SAMPLE_RATE_HZ: u32 = 48_000;
const BLOCK_FRAMES: usize = 256;
const OUTPUT_DIR: &str = "target/gfm-render";

fn main() -> std::io::Result<()> {
    create_dir_all(OUTPUT_DIR)?;

    for program_id in GfmProgramId::ALL_PERFORMANCE {
        render_wav(
            format!("{OUTPUT_DIR}/{}", engine_block_wav_file_name(program_id)),
            program_id,
        )?;
    }

    Ok(())
}

fn render_wav(path: impl AsRef<Path>, program_id: GfmProgramId) -> std::io::Result<()> {
    let path = path.as_ref();
    let mut voice = GfmFieldVoice::new(program_id, GFM_PERFORMANCE_BASELINE_SEED, SAMPLE_RATE_HZ);
    let frames = voice.frames();
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    write_wav_header(&mut writer, SAMPLE_RATE_HZ, frames)?;

    let mut block = [0.0_f32; BLOCK_FRAMES];
    let mut rendered = 0;
    let mut sum_squares = 0.0_f32;
    let mut peak_abs = 0.0_f32;
    let mut finite = true;

    while rendered < frames {
        let frame_count = (frames - rendered).min(block.len());
        voice.render_mono_block(&mut block[..frame_count]);
        for sample in &block[..frame_count] {
            finite &= sample.is_finite();
            peak_abs = peak_abs.max(sample.abs());
            sum_squares += sample * sample;
            writer.write_all(&sample_to_pcm16(*sample).to_le_bytes())?;
        }
        rendered += frame_count;
    }
    writer.flush()?;

    if !finite {
        return Err(std::io::Error::other(
            "GFM engine block render produced non-finite output",
        ));
    }

    let rms = (sum_squares / frames as f32).sqrt();
    let diagnostics = voice.diagnostics();
    println!(
        "{} rms={:.4} peak={:.4} max_ruptures={} health={:?}",
        path.display(),
        rms,
        peak_abs,
        diagnostics.max_rupture_count,
        diagnostics.health
    );

    Ok(())
}

fn engine_block_wav_file_name(program_id: GfmProgramId) -> &'static str {
    match program_id {
        GfmProgramId::HorizontPerformance => "engine_block_gfm_horizont.wav",
        GfmProgramId::PecPerformance => "engine_block_gfm_pec.wav",
        GfmProgramId::BakljaPerformance => "engine_block_gfm_baklja.wav",
    }
}

fn write_wav_header(
    writer: &mut impl Write,
    sample_rate_hz: u32,
    frames: usize,
) -> std::io::Result<()> {
    let data_len = (frames as u32) * 2;
    let byte_rate = sample_rate_hz * 2;

    writer.write_all(b"RIFF")?;
    writer.write_all(&(36 + data_len).to_le_bytes())?;
    writer.write_all(b"WAVE")?;
    writer.write_all(b"fmt ")?;
    writer.write_all(&16_u32.to_le_bytes())?;
    writer.write_all(&1_u16.to_le_bytes())?;
    writer.write_all(&1_u16.to_le_bytes())?;
    writer.write_all(&sample_rate_hz.to_le_bytes())?;
    writer.write_all(&byte_rate.to_le_bytes())?;
    writer.write_all(&2_u16.to_le_bytes())?;
    writer.write_all(&16_u16.to_le_bytes())?;
    writer.write_all(b"data")?;
    writer.write_all(&data_len.to_le_bytes())?;

    Ok(())
}
