use std::fs::{File, create_dir_all};
use std::io::{BufWriter, Write};
use std::path::Path;

use mamut_field::bcs::{BCS_V0_1_SAMPLE_RATE_HZ, BcsScenario, render_scenario};

const OUTPUT_DIR: &str = "target/bcs-render";

fn main() -> std::io::Result<()> {
    create_dir_all(OUTPUT_DIR)?;

    for scenario in BcsScenario::ALL {
        let (samples, diagnostics) = render_scenario(scenario).map_err(|diagnostics| {
            std::io::Error::other(format!("{scenario:?} render failed: {diagnostics:?}"))
        })?;
        let path = format!("{OUTPUT_DIR}/{}", scenario.wav_file_name());
        write_float_wav(&path, &samples)?;
        println!(
            "{} frames={} rms={:.6} peak={:.6} max_state={:.6} period_hz={:?} ratio={:?} lyap={:.6} zero_crossings={} period_count={}",
            path,
            diagnostics.frames,
            diagnostics.rms,
            diagnostics.peak_abs,
            diagnostics.max_state_abs,
            diagnostics.estimated_period_hz,
            diagnostics.subharmonic_ratio,
            diagnostics.lyapunov_proxy,
            diagnostics.zero_crossings,
            diagnostics.period_estimate_count,
        );
    }

    Ok(())
}

fn write_float_wav(path: impl AsRef<Path>, samples: &[f32]) -> std::io::Result<()> {
    let path = path.as_ref();
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    write_wav_header(&mut writer, samples.len())?;

    for sample in samples {
        writer.write_all(&sample.to_le_bytes())?;
    }

    writer.flush()
}

fn write_wav_header(writer: &mut impl Write, frames: usize) -> std::io::Result<()> {
    let data_len = (frames as u32) * 4;
    let byte_rate = BCS_V0_1_SAMPLE_RATE_HZ * 4;
    let riff_len = 48 + data_len;

    writer.write_all(b"RIFF")?;
    writer.write_all(&riff_len.to_le_bytes())?;
    writer.write_all(b"WAVE")?;
    writer.write_all(b"fmt ")?;
    writer.write_all(&16_u32.to_le_bytes())?;
    writer.write_all(&3_u16.to_le_bytes())?;
    writer.write_all(&1_u16.to_le_bytes())?;
    writer.write_all(&BCS_V0_1_SAMPLE_RATE_HZ.to_le_bytes())?;
    writer.write_all(&byte_rate.to_le_bytes())?;
    writer.write_all(&4_u16.to_le_bytes())?;
    writer.write_all(&32_u16.to_le_bytes())?;
    writer.write_all(b"fact")?;
    writer.write_all(&4_u32.to_le_bytes())?;
    writer.write_all(&(frames as u32).to_le_bytes())?;
    writer.write_all(b"data")?;
    writer.write_all(&data_len.to_le_bytes())?;

    Ok(())
}
