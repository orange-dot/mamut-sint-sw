use std::fs::{File, create_dir_all};
use std::io::{BufWriter, Write};
use std::path::Path;

use mamut_field::{GfmLattice16, GfmParams, sample_to_pcm16};

const SAMPLE_RATE_HZ: u32 = 48_000;
const SECONDS: usize = 4;
const FRAMES: usize = SAMPLE_RATE_HZ as usize * SECONDS;
const OUTPUT_DIR: &str = "target/gfm-render";

fn main() -> std::io::Result<()> {
    create_dir_all(OUTPUT_DIR)?;

    let mut slice_a_low = GfmParams::slice_a(SAMPLE_RATE_HZ as f32);
    slice_a_low.grav_coupling = 0.08;
    slice_a_low.heat_dispersion = 0.25;
    slice_a_low.output_gain = 0.92;

    let mut slice_a_high = slice_a_low;
    slice_a_high.grav_coupling = 1.85;
    let mut horizont = GfmParams::horizont(SAMPLE_RATE_HZ as f32);
    horizont.output_gain = 0.90;
    let mut pec = GfmParams::pec(SAMPLE_RATE_HZ as f32);
    pec.output_gain = 0.84;
    let mut baklja = GfmParams::baklja(SAMPLE_RATE_HZ as f32);
    baklja.output_gain = 0.40;

    render_wav(
        format!("{OUTPUT_DIR}/slice_a_low_grav.wav"),
        0x6A46_4D00,
        slice_a_low,
    )?;
    render_wav(
        format!("{OUTPUT_DIR}/slice_a_high_grav.wav"),
        0x6A46_4D00,
        slice_a_high,
    )?;
    render_wav(format!("{OUTPUT_DIR}/horizont.wav"), 0x6A46_4D00, horizont)?;
    render_wav(format!("{OUTPUT_DIR}/pec.wav"), 0x6A46_4D00, pec)?;
    render_wav(format!("{OUTPUT_DIR}/baklja.wav"), 0x6A46_4D00, baklja)?;

    Ok(())
}

fn render_wav(path: impl AsRef<Path>, seed: u64, params: GfmParams) -> std::io::Result<()> {
    let path = path.as_ref();
    let mut lattice = GfmLattice16::new(seed, params);
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    write_wav_header(&mut writer, SAMPLE_RATE_HZ, FRAMES)?;

    let mut sum_squares = 0.0_f32;
    let mut peak_abs = 0.0_f32;
    let mut finite = true;
    for _ in 0..FRAMES {
        let sample = lattice.next_sample();
        finite &= sample.is_finite();
        peak_abs = peak_abs.max(sample.abs());
        sum_squares += sample * sample;
        writer.write_all(&sample_to_pcm16(sample).to_le_bytes())?;
    }
    writer.flush()?;

    if !finite {
        return Err(std::io::Error::other(
            "GFM render produced non-finite output",
        ));
    }

    let rms = (sum_squares / FRAMES as f32).sqrt();
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
