use super::*;

#[derive(Debug, Clone)]
pub struct OutputRecordingRequest {
    pub path: PathBuf,
    pub sample_rate_hz: u32,
    pub max_frames: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct MidiTraceLogStart {
    pub wav_path: PathBuf,
    pub log_path: PathBuf,
    pub patch_path: PathBuf,
    pub patch_name: String,
    pub sample_rate_hz: u32,
    pub max_frames: Option<usize>,
    pub midi_channel: Option<u8>,
    pub controller_profile: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MidiTraceLogStarted {
    pub path: PathBuf,
    pub generation: u64,
}

pub struct MidiTraceLog {
    pub active: Mutex<Option<ActiveMidiTraceLog>>,
    pub active_flag: AtomicBool,
    pub generation: AtomicU64,
}

pub struct ActiveMidiTraceLog {
    pub path: PathBuf,
    pub generation: u64,
    pub writer: BufWriter<File>,
    pub target_end_at: Option<Instant>,
    pub lines_written: u64,
    pub write_failed: bool,
}

impl Default for MidiTraceLog {
    fn default() -> Self {
        Self {
            active: Mutex::new(None),
            active_flag: AtomicBool::new(false),
            generation: AtomicU64::new(0),
        }
    }
}

impl MidiTraceLog {
    pub fn start(&self, request: MidiTraceLogStart) -> io::Result<MidiTraceLogStarted> {
        let mut active = self
            .active
            .lock()
            .map_err(|_| io::Error::other("MIDI trace log lock poisoned"))?;
        if active.is_some() {
            return Err(io::Error::other("MIDI trace log is already active"));
        }

        let started_at = Instant::now();
        let target_end_at = request.max_frames.and_then(|frames| {
            let seconds = frames as f64 / f64::from(request.sample_rate_hz.max(1));
            started_at.checked_add(Duration::from_secs_f64(seconds))
        });
        let mut writer = BufWriter::new(File::create(&request.log_path)?);
        write_midi_trace_log_header(&mut writer, &request)?;
        let path = request.log_path.clone();
        let generation = self.generation.fetch_add(1, Ordering::Relaxed) + 1;

        *active = Some(ActiveMidiTraceLog {
            path: path.clone(),
            generation,
            writer,
            target_end_at,
            lines_written: 0,
            write_failed: false,
        });
        self.active_flag.store(true, Ordering::Relaxed);
        Ok(MidiTraceLogStarted { path, generation })
    }

    pub fn is_active(&self) -> bool {
        self.active_flag.load(Ordering::Relaxed)
    }

    pub fn write_line(&self, received_at: Instant, line: &str) {
        if !self.is_active() {
            return;
        }

        let Ok(mut active) = self.active.lock() else {
            return;
        };
        let Some(active) = active.as_mut() else {
            self.active_flag.store(false, Ordering::Relaxed);
            return;
        };
        if active
            .target_end_at
            .is_some_and(|target_end_at| received_at > target_end_at)
        {
            return;
        }
        if active.write_failed {
            return;
        }

        if let Err(error) = writeln!(active.writer, "{line}") {
            active.write_failed = true;
            eprintln!(
                "midi trace sidecar write failed for {}: {error}",
                active.path.display()
            );
            return;
        }
        active.lines_written += 1;
    }

    pub fn finish(&self, reason: &str) -> Option<PathBuf> {
        self.finish_with_trace_drops(reason, 0)
    }

    pub fn finish_with_trace_drops(
        &self,
        reason: &str,
        trace_records_dropped: u64,
    ) -> Option<PathBuf> {
        self.active_flag.store(false, Ordering::Relaxed);
        let Ok(mut active) = self.active.lock() else {
            return None;
        };
        let mut active = active.take()?;

        let _ = writeln!(active.writer, "#");
        let _ = writeln!(active.writer, "# finish: {reason}");
        let _ = writeln!(active.writer, "# midi_lines: {}", active.lines_written);
        if trace_records_dropped > 0 {
            let _ = writeln!(
                active.writer,
                "# trace_records_dropped: {trace_records_dropped}"
            );
        }
        let _ = active.writer.flush();
        Some(active.path)
    }

    pub fn finish_generation_with_trace_drops(
        &self,
        generation: u64,
        reason: &str,
        trace_records_dropped: u64,
    ) -> Option<PathBuf> {
        let Ok(mut active) = self.active.lock() else {
            return None;
        };
        if active
            .as_ref()
            .is_none_or(|active| active.generation != generation)
        {
            return None;
        }
        self.active_flag.store(false, Ordering::Relaxed);
        let mut active = active.take()?;

        let _ = writeln!(active.writer, "#");
        let _ = writeln!(active.writer, "# finish: {reason}");
        let _ = writeln!(active.writer, "# midi_lines: {}", active.lines_written);
        if trace_records_dropped > 0 {
            let _ = writeln!(
                active.writer,
                "# trace_records_dropped: {trace_records_dropped}"
            );
        }
        let _ = active.writer.flush();
        Some(active.path)
    }

    pub fn abort_and_remove(&self) -> Option<PathBuf> {
        self.active_flag.store(false, Ordering::Relaxed);
        let Ok(mut active) = self.active.lock() else {
            return None;
        };
        let active = active.take()?;
        let path = active.path.clone();
        drop(active);
        let _ = fs::remove_file(&path);
        Some(path)
    }
}

pub fn output_recording_midi_log_path(wav_path: &Path) -> PathBuf {
    wav_path.with_extension("midi.log")
}

pub fn write_midi_trace_log_header<W: Write>(
    writer: &mut W,
    request: &MidiTraceLogStart,
) -> io::Result<()> {
    let started_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    writeln!(writer, "# Mamut MIDI trace sidecar v1")?;
    writeln!(writer, "# started_unix_ms: {started_unix_ms}")?;
    writeln!(writer, "# wav_path: {}", request.wav_path.display())?;
    writeln!(writer, "# midi_log_path: {}", request.log_path.display())?;
    writeln!(writer, "# patch_name: {}", request.patch_name)?;
    writeln!(writer, "# patch_path: {}", request.patch_path.display())?;
    writeln!(writer, "# sample_rate_hz: {}", request.sample_rate_hz)?;
    writeln!(
        writer,
        "# max_frames: {}",
        request
            .max_frames
            .map(|frames| frames.to_string())
            .unwrap_or_else(|| "manual-stop".to_string())
    )?;
    writeln!(
        writer,
        "# midi_channel: {}",
        request
            .midi_channel
            .map(|channel| channel.to_string())
            .unwrap_or_else(|| "all".to_string())
    )?;
    writeln!(
        writer,
        "# controller_profile: {}",
        request
            .controller_profile
            .as_deref()
            .unwrap_or("pc4-legacy")
    )?;
    writeln!(
        writer,
        "# timing: t=seconds from MIDI input open; dt=milliseconds from previous traced MIDI event"
    )?;
    writeln!(writer, "#")?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlsaPlaybackSampleFormat {
    Float32,
    Signed32,
}

impl AlsaPlaybackSampleFormat {
    pub fn alsa_format(self) -> Format {
        match self {
            Self::Float32 => Format::float(),
            Self::Signed32 => Format::s32(),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Float32 => "F32",
            Self::Signed32 => "S32_LE",
        }
    }
}
