use std::fs::{File, create_dir_all};
use std::io::{BufWriter, Write};
use std::path::Path;

use mamut_engine::{GfmFieldVoice, GfmPatchVoiceConfig};
use mamut_field::{GFM_PERFORMANCE_BASELINE_SEED, sample_to_pcm16};
use mamut_patch::load_patch_toml;

const SAMPLE_RATE_HZ: u32 = 48_000;
const BLOCK_FRAMES: usize = 256;
const OUTPUT_DIR: &str = "target/gfm-render";
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

fn main() -> std::io::Result<()> {
    create_dir_all(OUTPUT_DIR)?;
    let config = GfmPatchVoiceConfig::new(GFM_PERFORMANCE_BASELINE_SEED, SAMPLE_RATE_HZ);

    for (slug, patch_source) in FACTORY_PATCHES {
        let patch = load_patch_toml(patch_source).map_err(std::io::Error::other)?;
        let decision = GfmFieldVoice::from_patch(&patch, config);
        if !decision.is_enabled() {
            return Err(std::io::Error::other(format!(
                "factory patch did not select a GFM program: {}",
                patch.meta.patch_name
            )));
        }

        render_wav(
            format!("{OUTPUT_DIR}/engine_patch_gfm_{slug}.wav"),
            &patch.meta.patch_name,
            decision,
        )?;
    }

    Ok(())
}

fn render_wav(
    path: impl AsRef<Path>,
    patch_name: &str,
    decision: mamut_engine::GfmPatchVoiceDecision,
) -> std::io::Result<()> {
    let path = path.as_ref();
    let selection = decision.selection();
    let Some(program_id) = selection.program_id else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "enabled patch voice decision did not select a program",
        ));
    };
    let Some(mut voice) = decision.into_voice() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "enabled patch voice decision did not build a GFM voice",
        ));
    };
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
            "GFM engine patch-selection render produced non-finite output",
        ));
    }

    let rms = (sum_squares / frames as f32).sqrt();
    let diagnostics = voice.diagnostics();
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
