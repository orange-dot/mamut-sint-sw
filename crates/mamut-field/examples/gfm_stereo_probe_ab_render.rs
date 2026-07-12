use std::fs::{File, create_dir_all};
use std::io::{BufWriter, Write};
use std::path::Path;

use mamut_field::{
    GFM_PERFORMANCE_BASELINE_SEED, GfmLattice16, GfmPerformanceGesture, GfmPerformanceProgram,
    GfmProgramId, sample_to_pcm16,
};

const SAMPLE_RATE_HZ: u32 = 48_000;
const SECONDS: usize = 6;
const FRAMES: usize = SAMPLE_RATE_HZ as usize * SECONDS;
const OUTPUT_DIR: &str = "target/gfm-render";

fn main() -> std::io::Result<()> {
    create_dir_all(OUTPUT_DIR)?;

    for program_id in GfmProgramId::ALL_PERFORMANCE {
        let program = GfmPerformanceProgram::new(program_id, SAMPLE_RATE_HZ as f32);
        let slug = match program_id {
            GfmProgramId::HorizontPerformance => "horizont",
            GfmProgramId::PecPerformance => "pec",
            GfmProgramId::BakljaPerformance => "baklja",
        };

        let mono = render_mono(format!("{OUTPUT_DIR}/stereo_ab_mono_{slug}.wav"), program)?;
        let stereo = render_stereo(format!("{OUTPUT_DIR}/stereo_ab_pair_{slug}.wav"), program)?;
        let repeat = render_stereo_signature(program);

        if stereo.signature != repeat {
            return Err(std::io::Error::other(format!(
                "stereo probe render is not deterministic across runs: {slug}"
            )));
        }
        if !(mono.finite && stereo.finite) {
            return Err(std::io::Error::other(format!(
                "stereo probe A/B render produced non-finite output: {slug}"
            )));
        }

        let left_right_correlation =
            correlation(stereo.cross_power, stereo.left_power, stereo.right_power);
        let left_mono_correlation =
            correlation(stereo.left_mono_cross, stereo.left_power, mono.power);
        let right_mono_correlation =
            correlation(stereo.right_mono_cross, stereo.right_power, mono.power);
        let diff_rms = (stereo.diff_power / FRAMES as f32).sqrt();

        println!(
            "pair program={slug} corr(L,R)={left_right_correlation:.6} corr(L,mono)={left_mono_correlation:.6} corr(R,mono)={right_mono_correlation:.6} diff_rms={diff_rms:.6} fold_rms={:.4} fold_peak={:.4} mono_rms={:.4} mono_peak={:.4} signature={:016x}",
            (stereo.fold_power / FRAMES as f32).sqrt(),
            stereo.fold_peak,
            (mono.power / FRAMES as f32).sqrt(),
            mono.peak,
            stereo.signature
        );

        if left_right_correlation >= 0.999 {
            return Err(std::io::Error::other(format!(
                "stereo probe channels stayed correlated for {slug}: {left_right_correlation}"
            )));
        }
        if stereo.fold_peak > 1.0 || mono.peak > 1.0 {
            return Err(std::io::Error::other(format!(
                "stereo probe A/B render exceeded the unit ceiling: {slug}"
            )));
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct MonoStats {
    power: f32,
    peak: f32,
    finite: bool,
}

#[derive(Debug, Clone, Copy)]
struct StereoStats {
    left_power: f32,
    right_power: f32,
    cross_power: f32,
    diff_power: f32,
    fold_power: f32,
    fold_peak: f32,
    left_mono_cross: f32,
    right_mono_cross: f32,
    finite: bool,
    signature: u64,
}

fn render_mono(
    path: impl AsRef<Path>,
    program: GfmPerformanceProgram,
) -> std::io::Result<MonoStats> {
    let gesture = GfmPerformanceGesture::v0_4();
    let mut lattice = GfmLattice16::new(GFM_PERFORMANCE_BASELINE_SEED, program.params());
    let file = File::create(path.as_ref())?;
    let mut writer = BufWriter::new(file);
    write_wav_header(&mut writer, SAMPLE_RATE_HZ, FRAMES, 1)?;

    let mut power = 0.0_f32;
    let mut peak = 0.0_f32;
    let mut finite = true;
    for frame in 0..FRAMES {
        let sample =
            lattice.next_sample_with_excitation(gesture.excitation_at_frame(frame, SAMPLE_RATE_HZ));
        finite &= sample.is_finite();
        peak = peak.max(sample.abs());
        power += sample * sample;
        writer.write_all(&sample_to_pcm16(sample).to_le_bytes())?;
    }
    writer.flush()?;

    Ok(MonoStats {
        power,
        peak,
        finite,
    })
}

fn render_stereo(
    path: impl AsRef<Path>,
    program: GfmPerformanceProgram,
) -> std::io::Result<StereoStats> {
    let gesture = GfmPerformanceGesture::v0_4();
    // The mono reference lattice steps in lockstep so per-sample
    // cross-correlation against the center probe stays meaningful.
    let mut stereo_lattice = GfmLattice16::new(GFM_PERFORMANCE_BASELINE_SEED, program.params());
    let mut mono_lattice = GfmLattice16::new(GFM_PERFORMANCE_BASELINE_SEED, program.params());
    let file = File::create(path.as_ref())?;
    let mut writer = BufWriter::new(file);
    write_wav_header(&mut writer, SAMPLE_RATE_HZ, FRAMES, 2)?;

    let mut stats = StereoStats {
        left_power: 0.0,
        right_power: 0.0,
        cross_power: 0.0,
        diff_power: 0.0,
        fold_power: 0.0,
        fold_peak: 0.0,
        left_mono_cross: 0.0,
        right_mono_cross: 0.0,
        finite: true,
        signature: 0xcbf2_9ce4_8422_2325,
    };
    for frame in 0..FRAMES {
        let excitation = gesture.excitation_at_frame(frame, SAMPLE_RATE_HZ);
        let (left, right) = stereo_lattice.next_sample_stereo_with_excitation(excitation);
        let mono = mono_lattice.next_sample_with_excitation(excitation);
        stats.finite &= left.is_finite() && right.is_finite();
        stats.left_power += left * left;
        stats.right_power += right * right;
        stats.cross_power += left * right;
        let diff = left - right;
        stats.diff_power += diff * diff;
        let fold = (left + right) * 0.5;
        stats.fold_power += fold * fold;
        stats.fold_peak = stats.fold_peak.max(fold.abs());
        stats.left_mono_cross += left * mono;
        stats.right_mono_cross += right * mono;
        for pcm in [sample_to_pcm16(left), sample_to_pcm16(right)] {
            stats.signature ^= pcm as u16 as u64;
            stats.signature = stats.signature.wrapping_mul(0x100_0000_01b3);
            writer.write_all(&pcm.to_le_bytes())?;
        }
    }
    writer.flush()?;

    Ok(stats)
}

fn render_stereo_signature(program: GfmPerformanceProgram) -> u64 {
    let gesture = GfmPerformanceGesture::v0_4();
    let mut lattice = GfmLattice16::new(GFM_PERFORMANCE_BASELINE_SEED, program.params());
    let mut signature = 0xcbf2_9ce4_8422_2325_u64;
    for frame in 0..FRAMES {
        let (left, right) = lattice
            .next_sample_stereo_with_excitation(gesture.excitation_at_frame(frame, SAMPLE_RATE_HZ));
        for pcm in [sample_to_pcm16(left), sample_to_pcm16(right)] {
            signature ^= pcm as u16 as u64;
            signature = signature.wrapping_mul(0x100_0000_01b3);
        }
    }
    signature
}

fn correlation(cross_power: f32, left_power: f32, right_power: f32) -> f32 {
    cross_power / (left_power * right_power).sqrt().max(0.000_001)
}

fn write_wav_header(
    writer: &mut impl Write,
    sample_rate_hz: u32,
    frames: usize,
    channels: u16,
) -> std::io::Result<()> {
    let bytes_per_frame = channels as u32 * 2;
    let data_len = (frames as u32) * bytes_per_frame;
    let byte_rate = sample_rate_hz * bytes_per_frame;

    writer.write_all(b"RIFF")?;
    writer.write_all(&(36 + data_len).to_le_bytes())?;
    writer.write_all(b"WAVE")?;
    writer.write_all(b"fmt ")?;
    writer.write_all(&16_u32.to_le_bytes())?;
    writer.write_all(&1_u16.to_le_bytes())?;
    writer.write_all(&channels.to_le_bytes())?;
    writer.write_all(&sample_rate_hz.to_le_bytes())?;
    writer.write_all(&byte_rate.to_le_bytes())?;
    writer.write_all(&(bytes_per_frame as u16).to_le_bytes())?;
    writer.write_all(&16_u16.to_le_bytes())?;
    writer.write_all(b"data")?;
    writer.write_all(&data_len.to_le_bytes())?;

    Ok(())
}
