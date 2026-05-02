use std::fs::{File, create_dir_all};
use std::io::{BufWriter, Write};
use std::path::Path;

use mamut_field::{
    GFM_PERFORMANCE_BASELINE_SEED, GfmLattice16, GfmPerformanceGesture, GfmPerformanceProgram,
    sample_to_pcm16,
};

const SAMPLE_RATE_HZ: u32 = 48_000;
const OUTPUT_DIR: &str = "target/gfm-render";

fn main() -> std::io::Result<()> {
    create_dir_all(OUTPUT_DIR)?;
    let gesture = GfmPerformanceGesture::v0_4();

    for program in GfmPerformanceProgram::all(SAMPLE_RATE_HZ as f32) {
        render_wav(
            format!("{OUTPUT_DIR}/{}", program.wav_file_name()),
            program,
            gesture,
        )?;
    }

    Ok(())
}

fn render_wav(
    path: impl AsRef<Path>,
    program: GfmPerformanceProgram,
    gesture: GfmPerformanceGesture,
) -> std::io::Result<()> {
    let path = path.as_ref();
    let mut lattice = GfmLattice16::new(GFM_PERFORMANCE_BASELINE_SEED, program.params());
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    let frames = gesture.frames(SAMPLE_RATE_HZ);
    write_wav_header(&mut writer, SAMPLE_RATE_HZ, frames)?;

    let mut sum_squares = 0.0_f32;
    let mut peak_abs = 0.0_f32;
    let mut finite = true;
    for frame in 0..frames {
        let sample =
            lattice.next_sample_with_excitation(gesture.excitation_at_frame(frame, SAMPLE_RATE_HZ));
        finite &= sample.is_finite();
        peak_abs = peak_abs.max(sample.abs());
        sum_squares += sample * sample;
        writer.write_all(&sample_to_pcm16(sample).to_le_bytes())?;
    }
    writer.flush()?;

    if !finite {
        return Err(std::io::Error::other(
            "GFM performance render produced non-finite output",
        ));
    }

    let rms = (sum_squares / frames as f32).sqrt();
    let diagnostics = lattice.diagnostics();
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
