use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::{self, BufRead, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    process::Command,
};

use mamut_dsp::StereoBlockMut;
use mamut_engine::{
    ControllerEvent, Engine, EngineConfig, NoteEvent, OutputSafetySnapshot, ProcessBlock,
    Scheduled, ScheduledControllerEvent, ScheduledNoteEvent,
};
use mamut_params::ParamId;
use mamut_patch::{PatchFileV1, load_patch_toml};

const SAMPLE_RATE_HZ: u32 = 48_000;
const BLOCK_FRAMES: usize = 256;
const DURATION_SECONDS: usize = 3;
const ANALYSIS_WINDOW_FRAMES: usize = 1024;
const ANALYSIS_BINS: usize = 256;
const BASELINE_ROOT: &str = "runs/audio-baseline";

const FACTORY_PATCHES: [PatchFixture; 7] = [
    PatchFixture {
        slug: "molten-horizon",
        source: include_str!("../../../patches/factory/molten-horizon.toml"),
    },
    PatchFixture {
        slug: "cathedral-bloom",
        source: include_str!("../../../patches/factory/cathedral-bloom.toml"),
    },
    PatchFixture {
        slug: "granite-plain",
        source: include_str!("../../../patches/factory/granite-plain.toml"),
    },
    PatchFixture {
        slug: "gravity-wake",
        source: include_str!("../../../patches/factory/gravity-wake.toml"),
    },
    PatchFixture {
        slug: "ember-vault",
        source: include_str!("../../../patches/factory/ember-vault.toml"),
    },
    PatchFixture {
        slug: "furnace-choir",
        source: include_str!("../../../patches/factory/furnace-choir.toml"),
    },
    PatchFixture {
        slug: "razor-thaw",
        source: include_str!("../../../patches/factory/razor-thaw.toml"),
    },
];

const SCENARIOS: [BaselineScenario; 7] = [
    BaselineScenario::SingleLow,
    BaselineScenario::SingleMid,
    BaselineScenario::SingleHigh,
    BaselineScenario::DenseChord,
    BaselineScenario::PitchBend,
    BaselineScenario::Expression,
    BaselineScenario::HighResFilter,
];

#[derive(Debug, Clone, Copy)]
struct PatchFixture {
    slug: &'static str,
    source: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BaselineScenario {
    SingleLow,
    SingleMid,
    SingleHigh,
    DenseChord,
    PitchBend,
    Expression,
    HighResFilter,
}

impl BaselineScenario {
    const fn label(self) -> &'static str {
        match self {
            Self::SingleLow => "single-low",
            Self::SingleMid => "single-mid",
            Self::SingleHigh => "single-high",
            Self::DenseChord => "dense-chord",
            Self::PitchBend => "pitch-bend",
            Self::Expression => "expression",
            Self::HighResFilter => "high-res-filter",
        }
    }
}

#[derive(Debug, Clone)]
struct RenderArtifact {
    frames: Vec<[f32; 2]>,
    metrics: RenderMetrics,
}

#[derive(Debug, Clone, Copy, Default)]
struct RenderMetrics {
    frames: usize,
    rms_left: f32,
    rms_right: f32,
    rms_combined: f32,
    peak_left: f32,
    peak_right: f32,
    peak_combined: f32,
    dc_left: f32,
    dc_right: f32,
    zero_crossing_rate: f32,
    spectral: SpectralSummary,
    finite: bool,
    safety: OutputSafetySnapshot,
}

#[derive(Debug, Clone, Copy, Default)]
struct SpectralSummary {
    centroid_hz: f32,
    rolloff_hz: f32,
    low_energy: f32,
    mid_energy: f32,
    high_energy: f32,
}

#[derive(Debug, Clone)]
struct SummaryRow {
    patch: String,
    scenario: String,
    metrics: RenderMetrics,
    f32le: String,
}

fn main() -> io::Result<()> {
    run(&env::args().collect::<Vec<_>>())
}

fn run(args: &[String]) -> io::Result<()> {
    match args.get(1).map(String::as_str) {
        Some("capture") => {
            let Some(label) = args.get(2) else {
                print_usage();
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "capture requires a label",
                ));
            };
            capture(label)
        }
        Some("compare") => {
            let (Some(baseline), Some(candidate)) = (args.get(2), args.get(3)) else {
                print_usage();
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "compare requires baseline and candidate labels",
                ));
            };
            compare(baseline, candidate)
        }
        _ => {
            print_usage();
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "expected capture or compare",
            ))
        }
    }
}

fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  cargo run --locked -p mamut-engine --example audio_baseline -- capture <label>");
    eprintln!(
        "  cargo run --locked -p mamut-engine --example audio_baseline -- compare <baseline-label> <candidate-label>"
    );
}

fn capture(label: &str) -> io::Result<()> {
    validate_label(label)?;
    let root = baseline_dir(label);
    let audio_dir = root.join("audio");
    fs::create_dir_all(&audio_dir)?;

    let mut summary = BufWriter::new(File::create(root.join("summary.csv"))?);
    write_summary_header(&mut summary)?;

    for patch_fixture in FACTORY_PATCHES {
        let patch = load_patch_toml(patch_fixture.source).map_err(io::Error::other)?;
        for scenario in SCENARIOS {
            let artifact = render_patch_scenario(patch.clone(), scenario)?;
            let stem = artifact_stem(patch_fixture.slug, scenario);
            let f32_name = format!("audio/{stem}.f32le");
            let wav_name = format!("audio/{stem}.wav");
            write_stereo_f32le_file(&root.join(&f32_name), &artifact.frames)?;
            write_stereo_f32_wav_file(&root.join(&wav_name), &artifact.frames)?;
            write_summary_row(
                &mut summary,
                patch_fixture.slug,
                scenario.label(),
                &artifact.metrics,
                &wav_name,
                &f32_name,
            )?;
        }
    }

    summary.flush()?;
    write_manifest(&root, label)?;
    println!("captured audio baseline: {}", root.display());
    Ok(())
}

fn compare(baseline_label: &str, candidate_label: &str) -> io::Result<()> {
    validate_label(baseline_label)?;
    validate_label(candidate_label)?;

    let baseline_root = baseline_dir(baseline_label);
    let candidate_root = baseline_dir(candidate_label);
    let baseline_summary = read_summary_rows(&baseline_root.join("summary.csv"))?;
    let candidate_summary = read_summary_rows(&candidate_root.join("summary.csv"))?;
    let compare_path = candidate_root.join(format!("compare-vs-{baseline_label}.csv"));
    let mut writer = BufWriter::new(File::create(&compare_path)?);
    write_compare_header(&mut writer)?;

    for patch_fixture in FACTORY_PATCHES {
        for scenario in SCENARIOS {
            let key = summary_key(patch_fixture.slug, scenario.label());
            let Some(baseline) = baseline_summary.get(&key) else {
                return Err(io::Error::other(format!(
                    "baseline summary row missing: {key}"
                )));
            };
            let Some(candidate) = candidate_summary.get(&key) else {
                return Err(io::Error::other(format!(
                    "candidate summary row missing: {key}"
                )));
            };
            let baseline_audio = read_stereo_f32le_file(&baseline_root.join(&baseline.f32le))?;
            let candidate_audio = read_stereo_f32le_file(&candidate_root.join(&candidate.f32le))?;
            let diff = compare_audio(&baseline_audio, &candidate_audio, baseline.metrics);
            write_compare_row(&mut writer, baseline, candidate, diff)?;
        }
    }

    writer.flush()?;
    println!("wrote report-only comparison: {}", compare_path.display());
    Ok(())
}

fn validate_label(label: &str) -> io::Result<()> {
    let valid = !label.is_empty()
        && label
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
        && !label.contains("..");
    if valid {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "label may contain only ASCII letters, digits, dot, dash, or underscore",
        ))
    }
}

fn baseline_dir(label: &str) -> PathBuf {
    PathBuf::from(BASELINE_ROOT).join(label)
}

fn artifact_stem(patch_slug: &str, scenario: BaselineScenario) -> String {
    format!("{}__{}", patch_slug, scenario.label())
}

fn render_patch_scenario(
    patch: PatchFileV1,
    scenario: BaselineScenario,
) -> io::Result<RenderArtifact> {
    let total_frames = SAMPLE_RATE_HZ as usize * DURATION_SECONDS;
    let notes = note_events_for_scenario(scenario);
    let controllers = controller_events_for_scenario(scenario);
    let mut engine = Engine::new(
        EngineConfig {
            sample_rate_hz: SAMPLE_RATE_HZ as f32,
            max_block_frames: BLOCK_FRAMES,
            voice_count: 6,
        },
        patch,
    )
    .map_err(io::Error::other)?;

    let mut frames = Vec::with_capacity(total_frames);
    let mut left = [0.0_f32; BLOCK_FRAMES];
    let mut right = [0.0_f32; BLOCK_FRAMES];
    let mut rendered = 0_usize;
    let mut safety = OutputSafetySnapshot::default();

    while rendered < total_frames {
        let frame_count = (total_frames - rendered).min(BLOCK_FRAMES);
        let block_note_events = block_events(&notes, rendered, frame_count);
        let block_controller_events = block_events(&controllers, rendered, frame_count);

        engine.process_block(ProcessBlock {
            frame_count,
            note_events: &block_note_events,
            controller_events: &block_controller_events,
            macro_state: None,
            output: Some(StereoBlockMut::new(
                &mut left[..frame_count],
                &mut right[..frame_count],
            )),
        });

        accumulate_safety(&mut safety, engine.snapshot().output_safety);
        for index in 0..frame_count {
            frames.push([left[index], right[index]]);
        }
        rendered += frame_count;
    }

    let metrics = analyze_frames(&frames, safety);
    Ok(RenderArtifact { frames, metrics })
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

fn note_events_for_scenario(scenario: BaselineScenario) -> Vec<ScheduledNoteEvent> {
    let notes: &[u8] = match scenario {
        BaselineScenario::SingleLow => &[36],
        BaselineScenario::SingleMid
        | BaselineScenario::PitchBend
        | BaselineScenario::Expression
        | BaselineScenario::HighResFilter => &[60],
        BaselineScenario::SingleHigh => &[84],
        BaselineScenario::DenseChord => &[36, 43, 48, 55, 60, 67],
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

fn controller_events_for_scenario(scenario: BaselineScenario) -> Vec<ScheduledControllerEvent> {
    match scenario {
        BaselineScenario::PitchBend => vec![
            controller_at(0.0, ControllerEvent::PitchBend { semitones: -2.0 }),
            controller_at(0.75, ControllerEvent::PitchBend { semitones: 0.0 }),
            controller_at(1.50, ControllerEvent::PitchBend { semitones: 2.0 }),
            controller_at(2.25, ControllerEvent::PitchBend { semitones: 0.0 }),
        ],
        BaselineScenario::Expression => vec![
            controller_at(0.0, ControllerEvent::ModWheel { amount: 0.0 }),
            controller_at(0.0, ControllerEvent::ChannelAftertouch { pressure: 0.0 }),
            controller_at(0.5, ControllerEvent::ModWheel { amount: 0.35 }),
            controller_at(0.5, ControllerEvent::ChannelAftertouch { pressure: 0.25 }),
            controller_at(1.0, ControllerEvent::ModWheel { amount: 0.75 }),
            controller_at(1.0, ControllerEvent::ChannelAftertouch { pressure: 0.70 }),
            controller_at(1.5, ControllerEvent::ModWheel { amount: 1.0 }),
            controller_at(1.5, ControllerEvent::ChannelAftertouch { pressure: 1.0 }),
            controller_at(2.3, ControllerEvent::ModWheel { amount: 0.0 }),
            controller_at(2.3, ControllerEvent::ChannelAftertouch { pressure: 0.0 }),
        ],
        BaselineScenario::HighResFilter => vec![
            direct_param_at(0.0, ParamId::FilterCutoffHz, 180.0),
            direct_param_at(0.0, ParamId::FilterResonance, 0.95),
            direct_param_at(0.0, ParamId::FilterDrive, 0.85),
            direct_param_at(1.0, ParamId::FilterCutoffHz, 1_600.0),
            direct_param_at(2.0, ParamId::FilterCutoffHz, 8_800.0),
        ],
        BaselineScenario::SingleLow
        | BaselineScenario::SingleMid
        | BaselineScenario::SingleHigh
        | BaselineScenario::DenseChord => Vec::new(),
    }
}

fn controller_at(seconds: f32, event: ControllerEvent) -> ScheduledControllerEvent {
    Scheduled {
        frame_offset: frame_at_seconds(seconds),
        event,
    }
}

fn direct_param_at(seconds: f32, id: ParamId, value: f32) -> ScheduledControllerEvent {
    controller_at(seconds, ControllerEvent::DirectParam { id, value })
}

fn frame_at_seconds(seconds: f32) -> usize {
    (seconds.max(0.0) * SAMPLE_RATE_HZ as f32).round() as usize
}

fn analyze_frames(frames: &[[f32; 2]], safety: OutputSafetySnapshot) -> RenderMetrics {
    let frame_count = frames.len();
    if frame_count == 0 {
        return RenderMetrics {
            finite: true,
            safety,
            ..RenderMetrics::default()
        };
    }

    let mut sum_left = 0.0_f32;
    let mut sum_right = 0.0_f32;
    let mut sum_squares_left = 0.0_f32;
    let mut sum_squares_right = 0.0_f32;
    let mut peak_left = 0.0_f32;
    let mut peak_right = 0.0_f32;
    let mut finite = true;
    let mut zero_crossings = 0_usize;
    let mut last_mono = (frames[0][0] + frames[0][1]) * 0.5;

    for frame in frames {
        let left = frame[0];
        let right = frame[1];
        finite &= left.is_finite() && right.is_finite();
        sum_left += left;
        sum_right += right;
        sum_squares_left += left * left;
        sum_squares_right += right * right;
        peak_left = peak_left.max(left.abs());
        peak_right = peak_right.max(right.abs());

        let mono = (left + right) * 0.5;
        if (last_mono < 0.0 && mono >= 0.0) || (last_mono >= 0.0 && mono < 0.0) {
            zero_crossings += 1;
        }
        last_mono = mono;
    }

    let frames_f32 = frame_count as f32;
    let rms_left = (sum_squares_left / frames_f32).sqrt();
    let rms_right = (sum_squares_right / frames_f32).sqrt();
    let rms_combined = ((sum_squares_left + sum_squares_right) / (frames_f32 * 2.0)).sqrt();
    let zero_crossing_rate = zero_crossings as f32 / frames_f32;

    RenderMetrics {
        frames: frame_count,
        rms_left,
        rms_right,
        rms_combined,
        peak_left,
        peak_right,
        peak_combined: peak_left.max(peak_right),
        dc_left: sum_left / frames_f32,
        dc_right: sum_right / frames_f32,
        zero_crossing_rate,
        spectral: analyze_spectrum(frames),
        finite,
        safety,
    }
}

fn analyze_spectrum(frames: &[[f32; 2]]) -> SpectralSummary {
    if frames.len() < 2 {
        return SpectralSummary::default();
    }

    let window_len = frames.len().min(ANALYSIS_WINDOW_FRAMES);
    let start = (frames.len() - window_len) / 2;
    let bins = ANALYSIS_BINS.min((window_len / 2).max(1));
    let mut total_energy = 0.0_f32;
    let mut weighted_frequency = 0.0_f32;
    let mut low_energy = 0.0_f32;
    let mut mid_energy = 0.0_f32;
    let mut high_energy = 0.0_f32;
    let mut bin_energies = Vec::with_capacity(bins);

    for bin in 1..=bins {
        let frequency_hz = bin as f32 * SAMPLE_RATE_HZ as f32 / window_len as f32;
        let mut real = 0.0_f32;
        let mut imag = 0.0_f32;
        for index in 0..window_len {
            let frame = frames[start + index];
            let mono = (frame[0] + frame[1]) * 0.5;
            let window = hann_window(index, window_len);
            let angle = std::f32::consts::TAU * bin as f32 * index as f32 / window_len as f32;
            real += mono * window * angle.cos();
            imag -= mono * window * angle.sin();
        }
        let energy = real * real + imag * imag;
        total_energy += energy;
        weighted_frequency += frequency_hz * energy;
        if frequency_hz <= 250.0 {
            low_energy += energy;
        } else if frequency_hz <= 2_000.0 {
            mid_energy += energy;
        } else {
            high_energy += energy;
        }
        bin_energies.push((frequency_hz, energy));
    }

    if total_energy <= f32::EPSILON {
        return SpectralSummary::default();
    }

    let rolloff_target = total_energy * 0.85;
    let mut cumulative = 0.0_f32;
    let mut rolloff_hz = 0.0_f32;
    for (frequency_hz, energy) in bin_energies {
        cumulative += energy;
        if cumulative >= rolloff_target {
            rolloff_hz = frequency_hz;
            break;
        }
    }

    SpectralSummary {
        centroid_hz: weighted_frequency / total_energy,
        rolloff_hz,
        low_energy,
        mid_energy,
        high_energy,
    }
}

fn hann_window(index: usize, len: usize) -> f32 {
    if len <= 1 {
        1.0
    } else {
        let phase = index as f32 / (len - 1) as f32;
        0.5 - 0.5 * (std::f32::consts::TAU * phase).cos()
    }
}

fn accumulate_safety(total: &mut OutputSafetySnapshot, block: OutputSafetySnapshot) {
    total.pre_safety_peak = total.pre_safety_peak.max(block.pre_safety_peak);
    total.post_safety_peak = total.post_safety_peak.max(block.post_safety_peak);
    total.safety_limiter_hits += block.safety_limiter_hits;
    total.max_safety_reduction = total.max_safety_reduction.max(block.max_safety_reduction);
    total.tiny_flush_events += block.tiny_flush_events;
}

fn write_stereo_f32le_file(path: &Path, frames: &[[f32; 2]]) -> io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    write_stereo_f32le(&mut writer, frames)
}

fn write_stereo_f32le<W: Write>(writer: &mut W, frames: &[[f32; 2]]) -> io::Result<()> {
    for frame in frames {
        writer.write_all(&frame[0].to_le_bytes())?;
        writer.write_all(&frame[1].to_le_bytes())?;
    }
    Ok(())
}

fn read_stereo_f32le_file(path: &Path) -> io::Result<Vec<[f32; 2]>> {
    let mut bytes = Vec::new();
    File::open(path)?.read_to_end(&mut bytes)?;
    if bytes.len() % 8 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("f32le stereo file has partial frame: {}", path.display()),
        ));
    }

    let mut frames = Vec::with_capacity(bytes.len() / 8);
    for chunk in bytes.chunks_exact(8) {
        frames.push([
            f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]),
            f32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]),
        ]);
    }
    Ok(frames)
}

fn write_stereo_f32_wav_file(path: &Path, frames: &[[f32; 2]]) -> io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    write_stereo_f32_wav(&mut writer, SAMPLE_RATE_HZ, frames)
}

fn write_stereo_f32_wav<W: Write>(
    writer: &mut W,
    sample_rate_hz: u32,
    frames: &[[f32; 2]],
) -> io::Result<()> {
    let data_len = frames
        .len()
        .checked_mul(8)
        .and_then(|len| u32::try_from(len).ok())
        .ok_or_else(|| io::Error::other("WAV data is too large"))?;
    let byte_rate = sample_rate_hz
        .checked_mul(8)
        .ok_or_else(|| io::Error::other("WAV byte rate overflow"))?;

    writer.write_all(b"RIFF")?;
    writer.write_all(&(36 + data_len).to_le_bytes())?;
    writer.write_all(b"WAVE")?;
    writer.write_all(b"fmt ")?;
    writer.write_all(&16_u32.to_le_bytes())?;
    writer.write_all(&3_u16.to_le_bytes())?;
    writer.write_all(&2_u16.to_le_bytes())?;
    writer.write_all(&sample_rate_hz.to_le_bytes())?;
    writer.write_all(&byte_rate.to_le_bytes())?;
    writer.write_all(&8_u16.to_le_bytes())?;
    writer.write_all(&32_u16.to_le_bytes())?;
    writer.write_all(b"data")?;
    writer.write_all(&data_len.to_le_bytes())?;
    write_stereo_f32le(writer, frames)
}

fn write_summary_header<W: Write>(writer: &mut W) -> io::Result<()> {
    writeln!(
        writer,
        "patch,scenario,frames,rms_left,rms_right,rms_combined,peak_left,peak_right,peak_combined,dc_left,dc_right,zero_crossing_rate,spectral_centroid_hz,spectral_rolloff_hz,low_energy,mid_energy,high_energy,pre_peak,post_peak,limiter_hits,max_reduction,tiny_flush,finite,wav,f32le"
    )
}

fn write_summary_row<W: Write>(
    writer: &mut W,
    patch: &str,
    scenario: &str,
    metrics: &RenderMetrics,
    wav: &str,
    f32le: &str,
) -> io::Result<()> {
    writeln!(
        writer,
        "{patch},{scenario},{},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{:.3},{:.3},{:.9},{:.9},{:.9},{:.9},{:.9},{},{:.9},{},{},{wav},{f32le}",
        metrics.frames,
        metrics.rms_left,
        metrics.rms_right,
        metrics.rms_combined,
        metrics.peak_left,
        metrics.peak_right,
        metrics.peak_combined,
        metrics.dc_left,
        metrics.dc_right,
        metrics.zero_crossing_rate,
        metrics.spectral.centroid_hz,
        metrics.spectral.rolloff_hz,
        metrics.spectral.low_energy,
        metrics.spectral.mid_energy,
        metrics.spectral.high_energy,
        metrics.safety.pre_safety_peak,
        metrics.safety.post_safety_peak,
        metrics.safety.safety_limiter_hits,
        metrics.safety.max_safety_reduction,
        metrics.safety.tiny_flush_events,
        metrics.finite
    )
}

fn read_summary_rows(path: &Path) -> io::Result<HashMap<String, SummaryRow>> {
    let file = File::open(path)?;
    let mut rows = HashMap::new();
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line = line?;
        if index == 0 || line.trim().is_empty() {
            continue;
        }
        let row = parse_summary_row(&line)?;
        rows.insert(summary_key(&row.patch, &row.scenario), row);
    }
    Ok(rows)
}

fn parse_summary_row(line: &str) -> io::Result<SummaryRow> {
    let fields = line.split(',').collect::<Vec<_>>();
    if fields.len() != 25 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("summary row has {} fields, expected 25", fields.len()),
        ));
    }

    Ok(SummaryRow {
        patch: fields[0].to_string(),
        scenario: fields[1].to_string(),
        metrics: RenderMetrics {
            frames: parse_usize(fields[2], "frames")?,
            rms_left: parse_f32(fields[3], "rms_left")?,
            rms_right: parse_f32(fields[4], "rms_right")?,
            rms_combined: parse_f32(fields[5], "rms_combined")?,
            peak_left: parse_f32(fields[6], "peak_left")?,
            peak_right: parse_f32(fields[7], "peak_right")?,
            peak_combined: parse_f32(fields[8], "peak_combined")?,
            dc_left: parse_f32(fields[9], "dc_left")?,
            dc_right: parse_f32(fields[10], "dc_right")?,
            zero_crossing_rate: parse_f32(fields[11], "zero_crossing_rate")?,
            spectral: SpectralSummary {
                centroid_hz: parse_f32(fields[12], "spectral_centroid_hz")?,
                rolloff_hz: parse_f32(fields[13], "spectral_rolloff_hz")?,
                low_energy: parse_f32(fields[14], "low_energy")?,
                mid_energy: parse_f32(fields[15], "mid_energy")?,
                high_energy: parse_f32(fields[16], "high_energy")?,
            },
            safety: OutputSafetySnapshot {
                pre_safety_peak: parse_f32(fields[17], "pre_peak")?,
                post_safety_peak: parse_f32(fields[18], "post_peak")?,
                safety_limiter_hits: parse_u64(fields[19], "limiter_hits")?,
                max_safety_reduction: parse_f32(fields[20], "max_reduction")?,
                tiny_flush_events: parse_u64(fields[21], "tiny_flush")?,
            },
            finite: parse_bool(fields[22], "finite")?,
        },
        f32le: fields[24].to_string(),
    })
}

fn parse_f32(value: &str, field: &str) -> io::Result<f32> {
    value.parse::<f32>().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid {field} value {value:?}: {error}"),
        )
    })
}

fn parse_u64(value: &str, field: &str) -> io::Result<u64> {
    value.parse::<u64>().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid {field} value {value:?}: {error}"),
        )
    })
}

fn parse_usize(value: &str, field: &str) -> io::Result<usize> {
    value.parse::<usize>().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid {field} value {value:?}: {error}"),
        )
    })
}

fn parse_bool(value: &str, field: &str) -> io::Result<bool> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid {field} value {value:?}"),
        )),
    }
}

fn summary_key(patch: &str, scenario: &str) -> String {
    format!("{patch}|{scenario}")
}

#[derive(Debug, Clone, Copy)]
struct AudioDiff {
    frames_compared: usize,
    null_rms: f32,
    null_db_relative: f32,
}

fn compare_audio(
    baseline: &[[f32; 2]],
    candidate: &[[f32; 2]],
    baseline_metrics: RenderMetrics,
) -> AudioDiff {
    let frames_compared = baseline.len().min(candidate.len());
    if frames_compared == 0 {
        return AudioDiff {
            frames_compared: 0,
            null_rms: 0.0,
            null_db_relative: 0.0,
        };
    }

    let mut sum_squares = 0.0_f32;
    for index in 0..frames_compared {
        let left = candidate[index][0] - baseline[index][0];
        let right = candidate[index][1] - baseline[index][1];
        sum_squares += left * left + right * right;
    }
    let null_rms = (sum_squares / (frames_compared * 2) as f32).sqrt();
    let reference = baseline_metrics.rms_combined.max(f32::MIN_POSITIVE);
    let null_db_relative = if null_rms <= f32::MIN_POSITIVE {
        -240.0
    } else {
        20.0 * (null_rms / reference).log10()
    };

    AudioDiff {
        frames_compared,
        null_rms,
        null_db_relative,
    }
}

fn write_compare_header<W: Write>(writer: &mut W) -> io::Result<()> {
    writeln!(
        writer,
        "patch,scenario,baseline_frames,candidate_frames,frames_compared,baseline_rms,candidate_rms,delta_rms,baseline_peak,candidate_peak,delta_peak,null_rms,null_db_relative,delta_dc_left,delta_dc_right,delta_centroid_hz,delta_rolloff_hz,delta_low_energy,delta_mid_energy,delta_high_energy,limiter_hit_delta,baseline_finite,candidate_finite"
    )
}

fn write_compare_row<W: Write>(
    writer: &mut W,
    baseline: &SummaryRow,
    candidate: &SummaryRow,
    diff: AudioDiff,
) -> io::Result<()> {
    let baseline_metrics = baseline.metrics;
    let candidate_metrics = candidate.metrics;
    let limiter_hit_delta = candidate_metrics.safety.safety_limiter_hits as i128
        - baseline_metrics.safety.safety_limiter_hits as i128;
    writeln!(
        writer,
        "{},{},{},{},{},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{:.3},{:.9},{:.9},{:.3},{:.3},{:.9},{:.9},{:.9},{},{},{}",
        baseline.patch,
        baseline.scenario,
        baseline_metrics.frames,
        candidate_metrics.frames,
        diff.frames_compared,
        baseline_metrics.rms_combined,
        candidate_metrics.rms_combined,
        candidate_metrics.rms_combined - baseline_metrics.rms_combined,
        baseline_metrics.peak_combined,
        candidate_metrics.peak_combined,
        candidate_metrics.peak_combined - baseline_metrics.peak_combined,
        diff.null_rms,
        diff.null_db_relative,
        candidate_metrics.dc_left - baseline_metrics.dc_left,
        candidate_metrics.dc_right - baseline_metrics.dc_right,
        candidate_metrics.spectral.centroid_hz - baseline_metrics.spectral.centroid_hz,
        candidate_metrics.spectral.rolloff_hz - baseline_metrics.spectral.rolloff_hz,
        candidate_metrics.spectral.low_energy - baseline_metrics.spectral.low_energy,
        candidate_metrics.spectral.mid_energy - baseline_metrics.spectral.mid_energy,
        candidate_metrics.spectral.high_energy - baseline_metrics.spectral.high_energy,
        limiter_hit_delta,
        baseline_metrics.finite,
        candidate_metrics.finite
    )
}

fn write_manifest(root: &Path, label: &str) -> io::Result<()> {
    let mut writer = BufWriter::new(File::create(root.join("manifest.txt"))?);
    writeln!(writer, "label: {label}")?;
    writeln!(writer, "git: {}", git_revision())?;
    writeln!(writer, "sample_rate_hz: {SAMPLE_RATE_HZ}")?;
    writeln!(writer, "block_frames: {BLOCK_FRAMES}")?;
    writeln!(writer, "duration_seconds: {DURATION_SECONDS}")?;
    writeln!(writer, "patches:")?;
    for patch in FACTORY_PATCHES {
        writeln!(writer, "- {}", patch.slug)?;
    }
    writeln!(writer, "scenarios:")?;
    for scenario in SCENARIOS {
        writeln!(writer, "- {}", scenario.label())?;
    }
    Ok(())
}

fn git_revision() -> String {
    match Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
    {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        _ => "unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stereo_f32le_writer_uses_interleaved_little_endian_frames() -> io::Result<()> {
        let frames = [[1.0, -1.0], [0.5, -0.5]];
        let mut bytes = Vec::new();
        write_stereo_f32le(&mut bytes, &frames)?;

        assert_eq!(bytes.len(), 16);
        assert_eq!(&bytes[0..4], &1.0_f32.to_le_bytes());
        assert_eq!(&bytes[4..8], &(-1.0_f32).to_le_bytes());
        assert_eq!(&bytes[8..12], &0.5_f32.to_le_bytes());
        assert_eq!(&bytes[12..16], &(-0.5_f32).to_le_bytes());
        Ok(())
    }

    #[test]
    fn stereo_f32_wav_writer_emits_ieee_float_header() -> io::Result<()> {
        let frames = [[0.0, 0.0], [0.25, -0.25]];
        let mut bytes = Vec::new();
        write_stereo_f32_wav(&mut bytes, 48_000, &frames)?;

        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(&bytes[20..22], &3_u16.to_le_bytes());
        assert_eq!(&bytes[22..24], &2_u16.to_le_bytes());
        assert_eq!(&bytes[34..36], &32_u16.to_le_bytes());
        assert_eq!(&bytes[36..40], b"data");
        assert_eq!(bytes.len(), 44 + frames.len() * 8);
        Ok(())
    }

    #[test]
    fn render_metrics_report_rms_peak_dc_and_null() {
        let frames = [[1.0, -1.0], [0.5, -0.5], [0.0, 0.0], [-0.5, 0.5]];
        let metrics = analyze_frames(&frames, OutputSafetySnapshot::default());

        assert_eq!(metrics.frames, 4);
        assert_eq!(metrics.peak_combined, 1.0);
        assert!((metrics.rms_combined - 0.612_372_46).abs() < 0.000_001);
        assert_eq!(metrics.dc_left, 0.25);
        assert_eq!(metrics.dc_right, -0.25);
        assert!(metrics.finite);

        let diff = compare_audio(&frames, &frames, metrics);
        assert_eq!(diff.frames_compared, 4);
        assert_eq!(diff.null_rms, 0.0);
        assert_eq!(diff.null_db_relative, -240.0);
    }

    #[test]
    fn spectral_summary_is_finite_for_sine_like_input() {
        let mut frames = Vec::new();
        for index in 0..ANALYSIS_WINDOW_FRAMES {
            let sample =
                (std::f32::consts::TAU * 4.0 * index as f32 / ANALYSIS_WINDOW_FRAMES as f32).sin();
            frames.push([sample, sample]);
        }

        let spectral = analyze_spectrum(&frames);
        assert!(spectral.centroid_hz.is_finite());
        assert!(spectral.rolloff_hz.is_finite());
        assert!(spectral.low_energy + spectral.mid_energy + spectral.high_energy > 0.0);
    }
}
