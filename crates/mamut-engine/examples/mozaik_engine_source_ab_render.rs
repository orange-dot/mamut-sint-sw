//! Mozaik v0.2 engine A/B render evidence (SET5-4).
//!
//! Renders a fixed molten-horizon note script with the Mozaik voice source
//! Disabled (A) and Enabled (B) at two slope settings plus drift, verifies
//! the Disabled renders bit-for-bit against the pre-slice baseline
//! signatures (rendered at the parent commit, before any engine change),
//! shows the voice filter shaping the blend via cutoff-pinned difference
//! signals, and measures per-block render cost with a steady-state chord.

use std::fs::{File, create_dir_all};
use std::io::{self, BufWriter, Write};
use std::time::Instant;

use mamut_dsp::StereoBlockMut;
use mamut_engine::{
    ControllerEvent, DEFAULT_MOZAIK_SEED, Engine, EngineConfig, MozaikMode, MozaikParam, NoteEvent,
    ProcessBlock, Scheduled, ScheduledControllerEvent, ScheduledNoteEvent,
};
use mamut_field::sample_to_pcm16;
use mamut_params::ParamId;
use mamut_patch::load_patch_toml;

const SAMPLE_RATE_HZ: u32 = 48_000;
const BLOCK_FRAMES: usize = 256;
const TOTAL_FRAMES: usize = 240_000;
const PATCH_SLUG: &str = "molten-horizon";
const PATCH_SOURCE: &str = include_str!("../../../patches/factory/molten-horizon.toml");
const OUTPUT_DIR: &str = "target/mozaik-render/engine";

/// Pre-slice baseline signatures, rendered at the parent commit (1ec7ddf)
/// with this example's baseline stage before any engine change landed. The
/// Disabled renders must stay bit-identical to these.
const BASELINE_SIGNATURE_BASE: u64 = 0xc175_bb8e_0d9d_7376;
const BASELINE_SIGNATURE_CUTOFF_LOW: u64 = 0x3189_5f0c_a101_2765;
const BASELINE_SIGNATURE_CUTOFF_HIGH: u64 = 0xb973_f3e9_f9a5_ced2;

const HOLD_WINDOW: (usize, usize) = (96_000, 144_000);
const TAIL_WINDOW: (usize, usize) = (230_000, 240_000);
const DIFF_FFT_START: usize = 110_000;
const DIFF_FFT_SIZE: usize = 16_384;

fn main() -> io::Result<()> {
    create_dir_all(OUTPUT_DIR)?;

    // A: Disabled — must equal the pre-slice baseline bit for bit.
    let base_a = render_script(&[], |_| {})?;
    assert_signature("a_disabled", base_a.signature, BASELINE_SIGNATURE_BASE)?;
    write_stereo_wav(&format!("{OUTPUT_DIR}/a_disabled.wav"), &base_a.frames)?;
    print_render("a_disabled", &base_a);

    // B: Enabled, on-enable defaults (slope at the golden detent).
    let b_golden = render_script(&[], |engine| {
        engine.set_mozaik_mode(MozaikMode::Enabled {
            seed: DEFAULT_MOZAIK_SEED,
        });
    })?;
    write_stereo_wav(&format!("{OUTPUT_DIR}/b_golden.wav"), &b_golden.frames)?;
    print_render("b_golden", &b_golden);
    if b_golden.signature == base_a.signature {
        return Err(io::Error::other(
            "enabled render is identical to the disabled render",
        ));
    }

    // B at the 1/2 detent: slope control 1/6 maps sigma to 0.5.
    let b_half = render_script(&[], |engine| {
        engine.set_mozaik_mode(MozaikMode::Enabled {
            seed: DEFAULT_MOZAIK_SEED,
        });
        engine.set_mozaik_param(MozaikParam::Slope, (0.5 - 0.45) / 0.30);
    })?;
    write_stereo_wav(&format!("{OUTPUT_DIR}/b_half.wav"), &b_half.frames)?;
    print_render("b_half_detent", &b_half);
    if b_half.signature == b_golden.signature {
        return Err(io::Error::other(
            "half-detent render is identical to the golden render",
        ));
    }

    // B with drift: constant-pitch rearrangement on the held notes.
    let b_drift = render_script(&[], |engine| {
        engine.set_mozaik_mode(MozaikMode::Enabled {
            seed: DEFAULT_MOZAIK_SEED,
        });
        engine.set_mozaik_param(MozaikParam::Drift, 0.8);
    })?;
    write_stereo_wav(&format!("{OUTPUT_DIR}/b_drift.wav"), &b_drift.frames)?;
    print_render("b_drift", &b_drift);
    if b_drift.signature == b_golden.signature {
        return Err(io::Error::other(
            "drift render is identical to the no-drift render",
        ));
    }
    let drift_delta_rms = diff_rms(&b_drift.frames, &b_golden.frames, 0, TOTAL_FRAMES);
    println!("  drift_vs_golden diff_rms={drift_delta_rms:.6}");

    // Determinism: a fresh golden render is byte-identical.
    let b_repeat = render_script(&[], |engine| {
        engine.set_mozaik_mode(MozaikMode::Enabled {
            seed: DEFAULT_MOZAIK_SEED,
        });
    })?;
    if b_repeat.frames != b_golden.frames {
        return Err(io::Error::other("two golden renders diverged"));
    }
    println!(
        "determinism run1={:016x} run2={:016x} byte_identical=true",
        b_golden.signature, b_repeat.signature
    );

    // The blend rides the voice filter: pin the cutoff low and high, take
    // the enabled-minus-disabled difference signal (the Mozaik contribution
    // plus its nonlinear interaction), and compare spectra.
    let a_low = render_script(&cutoff_events(300.0), |_| {})?;
    assert_signature(
        "a_cutoff_low",
        a_low.signature,
        BASELINE_SIGNATURE_CUTOFF_LOW,
    )?;
    let b_low = render_script(&cutoff_events(300.0), |engine| {
        engine.set_mozaik_mode(MozaikMode::Enabled {
            seed: DEFAULT_MOZAIK_SEED,
        });
    })?;
    let a_high = render_script(&cutoff_events(9_000.0), |_| {})?;
    assert_signature(
        "a_cutoff_high",
        a_high.signature,
        BASELINE_SIGNATURE_CUTOFF_HIGH,
    )?;
    let b_high = render_script(&cutoff_events(9_000.0), |engine| {
        engine.set_mozaik_mode(MozaikMode::Enabled {
            seed: DEFAULT_MOZAIK_SEED,
        });
    })?;
    let low_spec = diff_spectrum(&b_low.frames, &a_low.frames);
    let high_spec = diff_spectrum(&b_high.frames, &a_high.frames);
    println!(
        "filter_shaping cutoff=300Hz diff_centroid_hz={:.1} diff_high_band_ratio={:.6}",
        low_spec.centroid_hz, low_spec.high_band_ratio
    );
    println!(
        "filter_shaping cutoff=9000Hz diff_centroid_hz={:.1} diff_high_band_ratio={:.6}",
        high_spec.centroid_hz, high_spec.high_band_ratio
    );
    if low_spec.centroid_hz >= high_spec.centroid_hz {
        return Err(io::Error::other(
            "cutoff sweep does not shape the Mozaik blend: low-cutoff centroid >= high-cutoff centroid",
        ));
    }

    // The blend rides the voice envelope: the difference signal decays with
    // the release after both notes end.
    let hold_rms = diff_rms(
        &b_golden.frames,
        &base_a.frames,
        HOLD_WINDOW.0,
        HOLD_WINDOW.1,
    );
    let tail_rms = diff_rms(
        &b_golden.frames,
        &base_a.frames,
        TAIL_WINDOW.0,
        TAIL_WINDOW.1,
    );
    println!("envelope_gating hold_diff_rms={hold_rms:.6} tail_diff_rms={tail_rms:.6}");
    if tail_rms >= hold_rms {
        return Err(io::Error::other(
            "difference signal does not decay after note release",
        ));
    }

    // Per-block cost, steady-state six-voice chord, audio_baseline pattern.
    let disabled_cost = measure_block_cost(false)?;
    let enabled_cost = measure_block_cost(true)?;
    println!(
        "block_cost mode=disabled mean_us_per_block={:.2} ns_per_frame={:.1}",
        disabled_cost.0, disabled_cost.1
    );
    println!(
        "block_cost mode=enabled mean_us_per_block={:.2} ns_per_frame={:.1} delta_us_per_block={:.2}",
        enabled_cost.0,
        enabled_cost.1,
        enabled_cost.0 - disabled_cost.0
    );

    Ok(())
}

struct Render {
    frames: Vec<[f32; 2]>,
    rms: f32,
    peak: f32,
    dc_left: f32,
    dc_right: f32,
    signature: u64,
}

fn print_render(label: &str, render: &Render) {
    println!(
        "render script={label} patch={PATCH_SLUG} frames={} rms={:.6} peak={:.6} dc_left={:.6} dc_right={:.6} signature={:016x}",
        render.frames.len(),
        render.rms,
        render.peak,
        render.dc_left,
        render.dc_right,
        render.signature,
    );
}

fn assert_signature(label: &str, actual: u64, baseline: u64) -> io::Result<()> {
    if actual == baseline {
        println!("baseline_check script={label} signature={actual:016x} matches_pre_slice=true");
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "{label}: disabled render signature {actual:016x} != pre-slice baseline {baseline:016x}"
        )))
    }
}

fn note_script() -> Vec<ScheduledNoteEvent> {
    vec![
        note_event(
            4_800,
            NoteEvent::NoteOn {
                note: 48,
                velocity: 0.82,
            },
        ),
        note_event(
            48_000,
            NoteEvent::NoteOn {
                note: 55,
                velocity: 0.74,
            },
        ),
        note_event(144_000, NoteEvent::NoteOff { note: 48 }),
        note_event(192_000, NoteEvent::NoteOff { note: 55 }),
    ]
}

fn note_event(frame_offset: usize, event: NoteEvent) -> ScheduledNoteEvent {
    Scheduled {
        frame_offset,
        event,
    }
}

fn cutoff_events(cutoff_hz: f32) -> Vec<ScheduledControllerEvent> {
    vec![Scheduled {
        frame_offset: 0,
        event: ControllerEvent::DirectParam {
            id: ParamId::FilterCutoffHz,
            value: cutoff_hz,
        },
    }]
}

fn render_script(
    controllers: &[ScheduledControllerEvent],
    configure: impl FnOnce(&mut Engine),
) -> io::Result<Render> {
    let patch = load_patch_toml(PATCH_SOURCE).map_err(io::Error::other)?;
    let mut engine = Engine::new(
        EngineConfig {
            sample_rate_hz: SAMPLE_RATE_HZ as f32,
            max_block_frames: BLOCK_FRAMES,
            voice_count: 6,
        },
        patch,
    )
    .map_err(io::Error::other)?;
    configure(&mut engine);

    let notes = note_script();
    let mut frames = Vec::with_capacity(TOTAL_FRAMES);
    let mut left = [0.0_f32; BLOCK_FRAMES];
    let mut right = [0.0_f32; BLOCK_FRAMES];
    let mut rendered = 0_usize;

    while rendered < TOTAL_FRAMES {
        let frame_count = (TOTAL_FRAMES - rendered).min(BLOCK_FRAMES);
        let block_notes = block_events(&notes, rendered, frame_count);
        let block_controllers = block_events(controllers, rendered, frame_count);
        engine.process_block(ProcessBlock {
            frame_count,
            note_events: &block_notes,
            controller_events: &block_controllers,
            macro_state: None,
            output: Some(StereoBlockMut::new(
                &mut left[..frame_count],
                &mut right[..frame_count],
            )),
        });
        for index in 0..frame_count {
            frames.push([left[index], right[index]]);
        }
        rendered += frame_count;
    }

    analyze(frames)
}

fn block_events<T: Copy>(events: &[Scheduled<T>], start: usize, len: usize) -> Vec<Scheduled<T>> {
    let end = start.saturating_add(len);
    let mut out = Vec::new();
    for event in events {
        if event.frame_offset >= start && event.frame_offset < end {
            out.push(Scheduled {
                frame_offset: event.frame_offset - start,
                event: event.event,
            });
        }
    }
    out
}

fn analyze(frames: Vec<[f32; 2]>) -> io::Result<Render> {
    let mut power = 0.0_f64;
    let mut peak = 0.0_f32;
    let mut sum_left = 0.0_f64;
    let mut sum_right = 0.0_f64;
    let mut signature = 0xcbf2_9ce4_8422_2325_u64;
    for frame in &frames {
        let [left, right] = *frame;
        if !left.is_finite() || !right.is_finite() {
            return Err(io::Error::other("non-finite sample in render"));
        }
        power += f64::from(left) * f64::from(left) + f64::from(right) * f64::from(right);
        peak = peak.max(left.abs()).max(right.abs());
        sum_left += f64::from(left);
        sum_right += f64::from(right);
        for pcm in [sample_to_pcm16(left), sample_to_pcm16(right)] {
            signature ^= pcm as u16 as u64;
            signature = signature.wrapping_mul(0x100_0000_01b3);
        }
    }
    let count = frames.len().max(1) as f64;
    Ok(Render {
        rms: ((power / (count * 2.0)).sqrt()) as f32,
        peak,
        dc_left: (sum_left / count) as f32,
        dc_right: (sum_right / count) as f32,
        frames,
        signature,
    })
}

fn diff_rms(after: &[[f32; 2]], before: &[[f32; 2]], start: usize, end: usize) -> f32 {
    let end = end.min(after.len()).min(before.len());
    let start = start.min(end);
    let mut power = 0.0_f64;
    let mut count = 0_usize;
    for index in start..end {
        let left = f64::from(after[index][0]) - f64::from(before[index][0]);
        let right = f64::from(after[index][1]) - f64::from(before[index][1]);
        power += left * left + right * right;
        count += 2;
    }
    ((power / count.max(1) as f64).sqrt()) as f32
}

struct DiffSpectrum {
    centroid_hz: f64,
    high_band_ratio: f64,
}

/// Hann-windowed FFT of the mono fold of `after - before` over the held
/// region; reports the spectral centroid and the >2.5 kHz energy share.
fn diff_spectrum(after: &[[f32; 2]], before: &[[f32; 2]]) -> DiffSpectrum {
    let mut re = Vec::with_capacity(DIFF_FFT_SIZE);
    for index in 0..DIFF_FFT_SIZE {
        let frame = DIFF_FFT_START + index;
        let left = f64::from(after[frame][0]) - f64::from(before[frame][0]);
        let right = f64::from(after[frame][1]) - f64::from(before[frame][1]);
        let window =
            0.5 * (1.0 - (std::f64::consts::TAU * index as f64 / DIFF_FFT_SIZE as f64).cos());
        re.push((left + right) * 0.5 * window);
    }
    let mut im = vec![0.0_f64; DIFF_FFT_SIZE];
    fft_in_place(&mut re, &mut im);

    let bin_hz = f64::from(SAMPLE_RATE_HZ) / DIFF_FFT_SIZE as f64;
    let mut total_energy = 0.0_f64;
    let mut weighted = 0.0_f64;
    let mut high_energy = 0.0_f64;
    for bin in 1..DIFF_FFT_SIZE / 2 {
        let energy = re[bin] * re[bin] + im[bin] * im[bin];
        let frequency_hz = bin as f64 * bin_hz;
        total_energy += energy;
        weighted += frequency_hz * energy;
        if frequency_hz > 2_500.0 {
            high_energy += energy;
        }
    }
    if total_energy <= f64::MIN_POSITIVE {
        return DiffSpectrum {
            centroid_hz: 0.0,
            high_band_ratio: 0.0,
        };
    }
    DiffSpectrum {
        centroid_hz: weighted / total_energy,
        high_band_ratio: high_energy / total_energy,
    }
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

/// Steady-state per-block cost with a held six-voice chord, following the
/// `audio_baseline.rs` render-loop shape: warm up, then time `process_block`
/// over a fixed block count. Returns (mean microseconds per block, mean
/// nanoseconds per frame).
fn measure_block_cost(enable_mozaik: bool) -> io::Result<(f64, f64)> {
    const WARMUP_BLOCKS: usize = 400;
    const TIMED_BLOCKS: usize = 4_000;

    let patch = load_patch_toml(PATCH_SOURCE).map_err(io::Error::other)?;
    let mut engine = Engine::new(
        EngineConfig {
            sample_rate_hz: SAMPLE_RATE_HZ as f32,
            max_block_frames: BLOCK_FRAMES,
            voice_count: 6,
        },
        patch,
    )
    .map_err(io::Error::other)?;
    if enable_mozaik {
        engine.set_mozaik_mode(MozaikMode::Enabled {
            seed: DEFAULT_MOZAIK_SEED,
        });
    }

    let chord: Vec<ScheduledNoteEvent> = [36_u8, 43, 48, 55, 60, 67]
        .iter()
        .map(|&note| {
            note_event(
                0,
                NoteEvent::NoteOn {
                    note,
                    velocity: 0.88,
                },
            )
        })
        .collect();
    let mut left = [0.0_f32; BLOCK_FRAMES];
    let mut right = [0.0_f32; BLOCK_FRAMES];

    let mut render_block = |engine: &mut Engine, notes: &[ScheduledNoteEvent]| {
        engine.process_block(ProcessBlock {
            frame_count: BLOCK_FRAMES,
            note_events: notes,
            controller_events: &[],
            macro_state: None,
            output: Some(StereoBlockMut::new(&mut left, &mut right)),
        });
    };

    render_block(&mut engine, &chord);
    for _ in 1..WARMUP_BLOCKS {
        render_block(&mut engine, &[]);
    }
    let started = Instant::now();
    for _ in 0..TIMED_BLOCKS {
        render_block(&mut engine, &[]);
    }
    let elapsed = started.elapsed();

    let mean_us = elapsed.as_secs_f64() * 1.0e6 / TIMED_BLOCKS as f64;
    let per_frame_ns = elapsed.as_secs_f64() * 1.0e9 / (TIMED_BLOCKS * BLOCK_FRAMES) as f64;
    Ok((mean_us, per_frame_ns))
}

fn write_stereo_wav(path: &str, frames: &[[f32; 2]]) -> io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    let data_len = frames
        .len()
        .checked_mul(8)
        .and_then(|len| u32::try_from(len).ok())
        .ok_or_else(|| io::Error::other("WAV data is too large"))?;

    writer.write_all(b"RIFF")?;
    writer.write_all(&(36 + data_len).to_le_bytes())?;
    writer.write_all(b"WAVE")?;
    writer.write_all(b"fmt ")?;
    writer.write_all(&16_u32.to_le_bytes())?;
    writer.write_all(&3_u16.to_le_bytes())?;
    writer.write_all(&2_u16.to_le_bytes())?;
    writer.write_all(&SAMPLE_RATE_HZ.to_le_bytes())?;
    writer.write_all(&(SAMPLE_RATE_HZ * 8).to_le_bytes())?;
    writer.write_all(&8_u16.to_le_bytes())?;
    writer.write_all(&32_u16.to_le_bytes())?;
    writer.write_all(b"data")?;
    writer.write_all(&data_len.to_le_bytes())?;
    for frame in frames {
        writer.write_all(&frame[0].to_le_bytes())?;
        writer.write_all(&frame[1].to_le_bytes())?;
    }
    writer.flush()
}
