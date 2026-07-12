//! Mozaik v0.1 offline render evidence: quasicrystal oscillator scenarios.
//!
//! Renders the six SET5-3 scenarios plus the pitch-anchor table to
//! `target/mozaik-render/`, printing one metrics block per scenario. All
//! spectral numbers come from the in-example radix-2 FFT (no external tools).

use std::fs::{File, create_dir_all};
use std::io::{BufWriter, Write};
use std::path::Path;

use mamut_dsp::{
    MOZAIK_DEFAULT_CONTRAST, MOZAIK_SLOPE_DETENT_FIVE_EIGHTHS_Q32, MOZAIK_SLOPE_DETENT_HALF_Q32,
    MOZAIK_SLOPE_DETENT_THREE_FIFTHS_Q32, MOZAIK_SLOPE_DETENT_TWO_THIRDS_Q32,
    MOZAIK_SLOPE_GOLDEN_Q32, QuasicrystalOsc, QuasicrystalWord, sanitize_sample,
};

const OUTPUT_DIR: &str = "target/mozaik-render";
const SAMPLE_RATE_HZ: u32 = 48_000;
const SAMPLE_RATE_F: f32 = 48_000.0;
const F0_HZ: f32 = 220.0;
const FFT_SIZE: usize = 65_536;
const LONG_SETTLE_FRAMES: usize = 24_000;
const STEP_SETTLE_FRAMES: usize = 4_096;
const STEP_FRAMES: usize = 72_000;
const MORPH_CLICK_GUARD: f32 = 0.2;

const SLOPE_STEPS: [(&str, u32); 5] = [
    ("1/2", MOZAIK_SLOPE_DETENT_HALF_Q32),
    ("3/5", MOZAIK_SLOPE_DETENT_THREE_FIFTHS_Q32),
    ("golden", MOZAIK_SLOPE_GOLDEN_Q32),
    ("5/8", MOZAIK_SLOPE_DETENT_FIVE_EIGHTHS_Q32),
    ("2/3", MOZAIK_SLOPE_DETENT_TWO_THIRDS_Q32),
];
const CONTRAST_STEPS: [f32; 5] = [1.0, 1.3, MOZAIK_DEFAULT_CONTRAST, 1.9, 2.2];
const PITCH_ANCHOR_F0S: [f32; 8] = [55.0, 110.0, 220.0, 440.0, 880.0, 1760.0, 3520.0, 8000.0];

fn main() -> std::io::Result<()> {
    create_dir_all(OUTPUT_DIR)?;

    scenario_golden()?;
    scenario_detent_walk()?;
    scenario_slope_morph()?;
    scenario_phason_drift()?;
    scenario_contrast()?;
    scenario_determinism()?;
    pitch_anchor_table()?;

    Ok(())
}

fn render_golden(frames: usize) -> Vec<f32> {
    let mut osc = QuasicrystalOsc::new();
    let mut samples = Vec::with_capacity(frames);
    for _ in 0..frames {
        samples.push(osc.next_sample(F0_HZ, SAMPLE_RATE_F));
    }
    samples
}

fn scenario_golden() -> std::io::Result<()> {
    let samples = render_golden(192_000);
    check_bounded(&samples, "golden")?;
    let path = format!("{OUTPUT_DIR}/golden.wav");
    write_float_wav(&path, &samples)?;

    let segment = &samples[LONG_SETTLE_FRAMES..LONG_SETTLE_FRAMES + FFT_SIZE];
    let spectrum = analyze_spectrum(segment);
    let (autocorr_lag, autocorr_norm) = autocorr_peak(segment, 100, 24_000);
    let (rms, peak) = stats(&samples);
    let dc_mean = mean(&samples);
    println!(
        "{path} frames={} rms={rms:.6} peak={peak:.6} dc_mean={dc_mean:.6} strongest_peak_hz={:.2} peak_over_f0={:.4} autocorr_lag={autocorr_lag} autocorr_norm={autocorr_norm:.4} floor_db={:.1} signature={:016x}",
        samples.len(),
        spectrum.strongest_hz,
        spectrum.strongest_hz / f64::from(F0_HZ),
        spectrum.floor_db,
        fnv64_pcm(&samples),
    );
    println!("  top8 {}", format_peaks(&spectrum.top_peaks));

    Ok(())
}

fn scenario_detent_walk() -> std::io::Result<()> {
    let mut osc = QuasicrystalOsc::new();
    let mut samples = Vec::with_capacity(SLOPE_STEPS.len() * STEP_FRAMES);
    for (_, slope_q32) in SLOPE_STEPS {
        osc.set_slope_q32(slope_q32);
        for _ in 0..STEP_FRAMES {
            samples.push(osc.next_sample(F0_HZ, SAMPLE_RATE_F));
        }
    }
    check_bounded(&samples, "detent_walk")?;
    let path = format!("{OUTPUT_DIR}/detent_walk.wav");
    write_float_wav(&path, &samples)?;

    let (rms, peak) = stats(&samples);
    println!(
        "{path} frames={} rms={rms:.6} peak={peak:.6} signature={:016x}",
        samples.len(),
        fnv64_pcm(&samples),
    );
    for (index, (name, slope_q32)) in SLOPE_STEPS.iter().enumerate() {
        let start = index * STEP_FRAMES + STEP_SETTLE_FRAMES;
        let segment = &samples[start..start + FFT_SIZE];
        let spectrum = analyze_spectrum(segment);
        let (_, autocorr_norm) = autocorr_peak(segment, 100, 24_000);
        let sigma = f64::from(*slope_q32) / 4_294_967_296.0;
        println!(
            "  step name={name} sigma={sigma:.6} strongest_peak_hz={:.2} peak_over_f0={:.4} autocorr_norm={autocorr_norm:.4} floor_db={:.1} top8 {}",
            spectrum.strongest_hz,
            spectrum.strongest_hz / f64::from(F0_HZ),
            spectrum.floor_db,
            format_peaks(&spectrum.top_peaks),
        );
    }

    Ok(())
}

fn scenario_slope_morph() -> std::io::Result<()> {
    const FRAMES: usize = 288_000;
    let mut osc = QuasicrystalOsc::new();
    let mut samples = Vec::with_capacity(FRAMES);
    for frame in 0..FRAMES {
        let sigma = 0.45 + 0.30 * (frame as f32 / (FRAMES - 1) as f32);
        osc.set_slope(sigma);
        samples.push(osc.next_sample(F0_HZ, SAMPLE_RATE_F));
    }
    check_bounded(&samples, "slope_morph")?;
    let path = format!("{OUTPUT_DIR}/slope_morph.wav");
    write_float_wav(&path, &samples)?;

    let morph_step = max_step(&samples);
    let golden_step = max_step(&render_golden(96_000));
    let mut extreme = QuasicrystalOsc::new();
    extreme.set_slope(0.75);
    let mut extreme_samples = Vec::with_capacity(96_000);
    for _ in 0..96_000 {
        extreme_samples.push(extreme.next_sample(F0_HZ, SAMPLE_RATE_F));
    }
    let extreme_step = max_step(&extreme_samples);
    if morph_step > MORPH_CLICK_GUARD {
        return Err(std::io::Error::other(format!(
            "slope_morph max_step {morph_step} exceeds the click guard {MORPH_CLICK_GUARD}"
        )));
    }
    let (rms, peak) = stats(&samples);
    println!(
        "{path} frames={} rms={rms:.6} peak={peak:.6} max_step={morph_step:.6} static_golden_max_step={golden_step:.6} static_sigma075_max_step={extreme_step:.6} signature={:016x}",
        samples.len(),
        fnv64_pcm(&samples),
    );

    Ok(())
}

fn scenario_phason_drift() -> std::io::Result<()> {
    const FRAMES: usize = 288_000;
    let mut osc = QuasicrystalOsc::new();
    let mut samples = Vec::with_capacity(FRAMES);
    for frame in 0..FRAMES {
        osc.set_phason(frame as f32 / FRAMES as f32);
        samples.push(osc.next_sample(F0_HZ, SAMPLE_RATE_F));
    }
    check_bounded(&samples, "phason_drift")?;
    let path = format!("{OUTPUT_DIR}/phason_drift.wav");
    write_float_wav(&path, &samples)?;

    let drift_step = max_step(&samples);
    if drift_step > MORPH_CLICK_GUARD {
        return Err(std::io::Error::other(format!(
            "phason_drift max_step {drift_step} exceeds the click guard {MORPH_CLICK_GUARD}"
        )));
    }
    let first = analyze_spectrum(&samples[LONG_SETTLE_FRAMES..LONG_SETTLE_FRAMES + FFT_SIZE]);
    let second_start = FRAMES - FFT_SIZE - 1_000;
    let second = analyze_spectrum(&samples[second_start..second_start + FFT_SIZE]);
    let pitch_shift = (second.strongest_hz - first.strongest_hz).abs() / first.strongest_hz;
    if pitch_shift > 0.02 {
        return Err(std::io::Error::other(format!(
            "phason_drift moved the strongest peak by {pitch_shift:.4} (> 2%): {} -> {} Hz",
            first.strongest_hz, second.strongest_hz
        )));
    }
    let (rms, peak) = stats(&samples);
    println!(
        "{path} frames={} rms={rms:.6} peak={peak:.6} max_step={drift_step:.6} first_half_peak_hz={:.2} second_half_peak_hz={:.2} signature={:016x}",
        samples.len(),
        first.strongest_hz,
        second.strongest_hz,
        fnv64_pcm(&samples),
    );
    let flips: Vec<String> = [0.01_f64, 0.05, 0.10, 0.25, 0.50]
        .iter()
        .map(|&delta| {
            let delta_q32 = (delta * 4_294_967_296.0) as u64 as u32;
            let fraction = word_flip_fraction(MOZAIK_SLOPE_GOLDEN_Q32, delta_q32, 20_000);
            format!("delta={delta:.2} fraction={fraction:.4}")
        })
        .collect();
    println!("  flips {}", flips.join(" "));

    Ok(())
}

fn scenario_contrast() -> std::io::Result<()> {
    let mut osc = QuasicrystalOsc::new();
    let mut samples = Vec::with_capacity(CONTRAST_STEPS.len() * STEP_FRAMES);
    for gamma in CONTRAST_STEPS {
        osc.set_contrast(gamma);
        for _ in 0..STEP_FRAMES {
            samples.push(osc.next_sample(F0_HZ, SAMPLE_RATE_F));
        }
    }
    check_bounded(&samples, "contrast")?;
    let path = format!("{OUTPUT_DIR}/contrast.wav");
    write_float_wav(&path, &samples)?;

    let (rms, peak) = stats(&samples);
    println!(
        "{path} frames={} rms={rms:.6} peak={peak:.6} signature={:016x}",
        samples.len(),
        fnv64_pcm(&samples),
    );
    for (index, gamma) in CONTRAST_STEPS.iter().enumerate() {
        let start = index * STEP_FRAMES + STEP_SETTLE_FRAMES;
        let segment = &samples[start..start + FFT_SIZE];
        let spectrum = analyze_spectrum(segment);
        let (step_rms, _) = stats(segment);
        println!(
            "  step gamma={gamma:.3} dc_mean={:.6} floor_db={:.1} rms={step_rms:.6} strongest_peak_hz={:.2}",
            mean(segment),
            spectrum.floor_db,
            spectrum.strongest_hz,
        );
    }

    Ok(())
}

fn scenario_determinism() -> std::io::Result<()> {
    let first = render_golden(192_000);
    let second = render_golden(192_000);
    let identical = first
        .iter()
        .zip(second.iter())
        .all(|(a, b)| a.to_bits() == b.to_bits());
    if !identical {
        return Err(std::io::Error::other(
            "determinism: two golden renders are not bit-identical",
        ));
    }
    let signature_first = fnv64_pcm(&first);
    let signature_second = fnv64_pcm(&second);
    if signature_first != signature_second {
        return Err(std::io::Error::other(
            "determinism: golden render signatures diverged",
        ));
    }
    println!(
        "determinism golden run1={signature_first:016x} run2={signature_second:016x} byte_identical=true"
    );

    Ok(())
}

fn pitch_anchor_table() -> std::io::Result<()> {
    for f0 in PITCH_ANCHOR_F0S {
        const FRAMES: usize = 96_000;
        let mut osc = QuasicrystalOsc::new();
        let mut samples = Vec::with_capacity(FRAMES);
        for _ in 0..FRAMES {
            samples.push(osc.next_sample(f0, SAMPLE_RATE_F));
        }
        check_bounded(&samples, "pitch_anchor")?;
        let segment = &samples[LONG_SETTLE_FRAMES..LONG_SETTLE_FRAMES + FFT_SIZE];
        let spectrum = analyze_spectrum(segment);
        let seconds = FRAMES as f64 / f64::from(SAMPLE_RATE_HZ);
        let tile_rate = f64::from(osc.tiles_emitted()) / seconds;
        let nominal_rate = 2.0 * f64::from(f0);
        println!(
            "pitch_anchor f0={f0} strongest_peak_hz={:.2} peak_over_f0={:.4} tile_rate_hz={tile_rate:.1} nominal_tile_rate_hz={nominal_rate:.1} tile_rate_bias={:.4}",
            spectrum.strongest_hz,
            spectrum.strongest_hz / f64::from(f0),
            tile_rate / nominal_rate,
        );
    }

    Ok(())
}

fn word_flip_fraction(slope_q32: u32, delta_q32: u32, tiles: u32) -> f64 {
    let mut reference = QuasicrystalWord::new(slope_q32, 0);
    let mut shifted = QuasicrystalWord::new(slope_q32, delta_q32);
    let mut flips = 0_u32;
    for _ in 0..tiles {
        if reference.next_tile_kind() != shifted.next_tile_kind() {
            flips += 1;
        }
    }
    f64::from(flips) / f64::from(tiles)
}

struct SpectrumAnalysis {
    strongest_hz: f64,
    top_peaks: Vec<(f64, f64)>,
    floor_db: f64,
}

fn analyze_spectrum(segment: &[f32]) -> SpectrumAnalysis {
    let n = segment.len();
    let mean_value = segment.iter().map(|&s| f64::from(s)).sum::<f64>() / n as f64;
    let mut re: Vec<f64> = segment
        .iter()
        .enumerate()
        .map(|(index, &sample)| (f64::from(sample) - mean_value) * hann_window(index, n))
        .collect();
    let mut im = vec![0.0_f64; n];
    fft_in_place(&mut re, &mut im);
    let magnitudes: Vec<f64> = (0..n / 2)
        .map(|bin| (re[bin] * re[bin] + im[bin] * im[bin]).sqrt())
        .collect();

    const MIN_BIN: usize = 16;
    let mut peak_bins: Vec<(usize, f64)> = Vec::new();
    for bin in MIN_BIN..n / 2 - 1 {
        if magnitudes[bin] > magnitudes[bin - 1] && magnitudes[bin] >= magnitudes[bin + 1] {
            peak_bins.push((bin, magnitudes[bin]));
        }
    }
    peak_bins.sort_by(|a, b| b.1.total_cmp(&a.1));
    peak_bins.truncate(8);

    let strongest_mag = peak_bins.first().map(|p| p.1).unwrap_or(1.0e-30);
    let refine = |bin: usize| -> f64 {
        let left = magnitudes[bin - 1].max(1.0e-30).ln();
        let center = magnitudes[bin].max(1.0e-30).ln();
        let right = magnitudes[bin + 1].max(1.0e-30).ln();
        let denom = left - 2.0 * center + right;
        let delta = if denom.abs() > 1.0e-12 {
            (0.5 * (left - right) / denom).clamp(-0.5, 0.5)
        } else {
            0.0
        };
        (bin as f64 + delta) * f64::from(SAMPLE_RATE_HZ) / n as f64
    };
    let top_peaks: Vec<(f64, f64)> = peak_bins
        .iter()
        .map(|&(bin, magnitude)| (refine(bin), 20.0 * (magnitude / strongest_mag).log10()))
        .collect();
    let strongest_hz = top_peaks.first().map(|p| p.0).unwrap_or(0.0);

    let floor_low_bin = (18_000.0 * n as f64 / f64::from(SAMPLE_RATE_HZ)) as usize;
    let mut floor_band: Vec<f64> = magnitudes[floor_low_bin..n / 2].to_vec();
    floor_band.sort_by(f64::total_cmp);
    let floor_median = floor_band[floor_band.len() / 2];
    let floor_db = 20.0 * (floor_median.max(1.0e-30) / strongest_mag).log10();

    SpectrumAnalysis {
        strongest_hz,
        top_peaks,
        floor_db,
    }
}

/// Unbiased normalized autocorrelation peak over `[min_lag, max_lag]`,
/// computed via zero-padded FFT of the power spectrum.
fn autocorr_peak(segment: &[f32], min_lag: usize, max_lag: usize) -> (usize, f64) {
    let n = segment.len();
    let padded = 2 * n;
    let mean_value = segment.iter().map(|&s| f64::from(s)).sum::<f64>() / n as f64;
    let mut re = vec![0.0_f64; padded];
    for (index, &sample) in segment.iter().enumerate() {
        re[index] = f64::from(sample) - mean_value;
    }
    let mut im = vec![0.0_f64; padded];
    fft_in_place(&mut re, &mut im);
    for bin in 0..padded {
        re[bin] = re[bin] * re[bin] + im[bin] * im[bin];
        im[bin] = 0.0;
    }
    // The power spectrum is real and even, so a second forward FFT returns
    // `padded * autocorr[k]`.
    fft_in_place(&mut re, &mut im);
    let zero_lag = re[0] / padded as f64;
    if zero_lag <= 0.0 {
        return (0, 0.0);
    }
    let mut best_lag = min_lag;
    let mut best_value = f64::MIN;
    for lag in min_lag..=max_lag.min(n - 1) {
        let unbiased = (re[lag] / padded as f64) / (n - lag) as f64;
        let normalized = unbiased / (zero_lag / n as f64);
        if normalized > best_value {
            best_value = normalized;
            best_lag = lag;
        }
    }
    (best_lag, best_value)
}

fn fft_in_place(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    let mut swap_target = 0_usize;
    for index in 1..n {
        let mut bit = n >> 1;
        while swap_target & bit != 0 {
            swap_target ^= bit;
            bit >>= 1;
        }
        swap_target |= bit;
        if index < swap_target {
            re.swap(index, swap_target);
            im.swap(index, swap_target);
        }
    }
    let mut len = 2;
    while len <= n {
        let angle = -std::f64::consts::TAU / len as f64;
        let (step_re, step_im) = (angle.cos(), angle.sin());
        for start in (0..n).step_by(len) {
            let mut twiddle_re = 1.0_f64;
            let mut twiddle_im = 0.0_f64;
            for offset in 0..len / 2 {
                let even = start + offset;
                let odd = even + len / 2;
                let product_re = re[odd] * twiddle_re - im[odd] * twiddle_im;
                let product_im = re[odd] * twiddle_im + im[odd] * twiddle_re;
                re[odd] = re[even] - product_re;
                im[odd] = im[even] - product_im;
                re[even] += product_re;
                im[even] += product_im;
                let next_re = twiddle_re * step_re - twiddle_im * step_im;
                twiddle_im = twiddle_re * step_im + twiddle_im * step_re;
                twiddle_re = next_re;
            }
        }
        len <<= 1;
    }
}

fn hann_window(index: usize, len: usize) -> f64 {
    0.5 * (1.0 - (std::f64::consts::TAU * index as f64 / len as f64).cos())
}

fn format_peaks(peaks: &[(f64, f64)]) -> String {
    let formatted: Vec<String> = peaks
        .iter()
        .map(|&(hz, db)| format!("{hz:.1}:{db:+.1}dB"))
        .collect();
    formatted.join(" ")
}

fn stats(samples: &[f32]) -> (f32, f32) {
    let mut power = 0.0_f64;
    let mut peak = 0.0_f32;
    for &sample in samples {
        power += f64::from(sample) * f64::from(sample);
        peak = peak.max(sample.abs());
    }
    (((power / samples.len().max(1) as f64).sqrt()) as f32, peak)
}

fn mean(samples: &[f32]) -> f64 {
    samples.iter().map(|&s| f64::from(s)).sum::<f64>() / samples.len().max(1) as f64
}

fn max_step(samples: &[f32]) -> f32 {
    samples
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).abs())
        .fold(0.0, f32::max)
}

fn check_bounded(samples: &[f32], scenario: &str) -> std::io::Result<()> {
    for (index, &sample) in samples.iter().enumerate() {
        if !sample.is_finite() || sample.abs() > 1.0 {
            return Err(std::io::Error::other(format!(
                "{scenario}: sample {index} out of bounds: {sample}"
            )));
        }
    }
    Ok(())
}

fn sample_to_pcm16(sample: f32) -> i16 {
    (sanitize_sample(sample) * f32::from(i16::MAX)).round() as i16
}

fn fnv64_pcm(samples: &[f32]) -> u64 {
    let mut signature = 0xcbf2_9ce4_8422_2325_u64;
    for &sample in samples {
        let pcm = sample_to_pcm16(sample);
        signature ^= pcm as u16 as u64;
        signature = signature.wrapping_mul(0x100_0000_01b3);
    }
    signature
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
    let byte_rate = SAMPLE_RATE_HZ * 4;
    let riff_len = 48 + data_len;

    writer.write_all(b"RIFF")?;
    writer.write_all(&riff_len.to_le_bytes())?;
    writer.write_all(b"WAVE")?;
    writer.write_all(b"fmt ")?;
    writer.write_all(&16_u32.to_le_bytes())?;
    writer.write_all(&3_u16.to_le_bytes())?;
    writer.write_all(&1_u16.to_le_bytes())?;
    writer.write_all(&SAMPLE_RATE_HZ.to_le_bytes())?;
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
