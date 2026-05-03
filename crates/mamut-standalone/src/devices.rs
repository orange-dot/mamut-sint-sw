use super::*;

pub(crate) fn open_driver_for_selector(
    tx: mpsc::Sender<EngineCommand>,
    midi_input_queue: Arc<ArrayQueue<RealtimeMidiMessage>>,
    bend_range: f32,
    midi_selector: Option<&str>,
    midi_channel: Option<u8>,
    controller_profile: Option<Arc<ControllerProfile>>,
    trace_midi: bool,
    runtime_control_queue: Arc<ArrayQueue<RuntimeControlMessage>>,
    input_metrics: Arc<InputMetrics>,
    midi_trace_log: Arc<MidiTraceLog>,
    sound_lab_midi_focus: Arc<SoundLabMidiFocus>,
    force_demo: bool,
) -> Result<PerformanceDriver> {
    if force_demo {
        return Ok(PerformanceDriver::Demo(DemoPerformer::spawn(tx)));
    }

    if let Some(connection) = open_midi_input(
        midi_input_queue,
        bend_range,
        midi_selector,
        midi_channel,
        controller_profile,
        trace_midi,
        runtime_control_queue,
        input_metrics,
        midi_trace_log,
        sound_lab_midi_focus,
    )? {
        Ok(PerformanceDriver::Midi(connection))
    } else {
        Ok(PerformanceDriver::Demo(DemoPerformer::spawn(tx)))
    }
}

pub(crate) fn restore_driver_state(
    tx: mpsc::Sender<EngineCommand>,
    midi_input_queue: Arc<ArrayQueue<RealtimeMidiMessage>>,
    bend_range: f32,
    midi_selector: Option<&str>,
    midi_channel: Option<u8>,
    controller_profile: Option<Arc<ControllerProfile>>,
    trace_midi: bool,
    runtime_control_queue: Arc<ArrayQueue<RuntimeControlMessage>>,
    input_metrics: Arc<InputMetrics>,
    midi_trace_log: Arc<MidiTraceLog>,
    sound_lab_midi_focus: Arc<SoundLabMidiFocus>,
    old_mode_was_demo: bool,
) -> PerformanceDriver {
    if old_mode_was_demo {
        return PerformanceDriver::Demo(DemoPerformer::spawn(tx));
    }

    match open_midi_input(
        midi_input_queue,
        bend_range,
        midi_selector,
        midi_channel,
        controller_profile,
        trace_midi,
        runtime_control_queue,
        input_metrics,
        midi_trace_log,
        sound_lab_midi_focus,
    ) {
        Ok(Some(connection)) => PerformanceDriver::Midi(connection),
        Ok(None) | Err(_) => PerformanceDriver::Idle,
    }
}

pub(crate) fn list_factory_patches() -> Result<()> {
    for entry in factory_patch_entries()? {
        let favorite = if entry.favorite { "*" } else { " " };
        let description = entry
            .description
            .as_deref()
            .unwrap_or("no description")
            .trim();
        println!(
            "{favorite} {:<18} {:<20} {}",
            entry.stem, entry.patch_name, description
        );
    }
    Ok(())
}

pub(crate) fn list_favorite_patches() -> Result<()> {
    for (slot, entry) in favorite_patch_entries()?.iter().enumerate() {
        let description = entry
            .description
            .as_deref()
            .unwrap_or("no description")
            .trim();
        println!(
            "{slot}: {:<18} {:<20} {}",
            entry.stem, entry.patch_name, description
        );
    }
    Ok(())
}

pub(crate) fn list_audio_devices() -> Result<()> {
    let devices = collect_alsa_output_devices()?;
    if devices.is_empty() {
        println!("no ALSA hw playback devices found");
        return Ok(());
    }

    for (index, entry) in devices.iter().enumerate() {
        println!(
            "{index}: {} - {} - {}",
            entry.selector, entry.card_name, entry.pcm_name
        );
    }
    Ok(())
}

pub(crate) fn collect_alsa_output_devices() -> Result<Vec<AlsaOutputDevice>> {
    let card_names = read_alsa_card_names()?;
    let pcm_contents = fs::read_to_string("/proc/asound/pcm")
        .context("failed to read /proc/asound/pcm for ALSA playback enumeration")?;
    let mut devices = Vec::new();

    for line in pcm_contents.lines() {
        let Some((card_index, device_index, pcm_name, has_playback)) = parse_alsa_pcm_line(line)
        else {
            continue;
        };
        if !has_playback {
            continue;
        }

        devices.push(AlsaOutputDevice {
            selector: format!("hw:{card_index},{device_index}"),
            card_index,
            device_index,
            card_name: card_names
                .get(&card_index)
                .cloned()
                .unwrap_or_else(|| format!("card {card_index}")),
            pcm_name,
        });
    }

    devices.sort_by(|left, right| {
        (
            left.card_index,
            left.device_index,
            left.card_name.as_str(),
            left.pcm_name.as_str(),
        )
            .cmp(&(
                right.card_index,
                right.device_index,
                right.card_name.as_str(),
                right.pcm_name.as_str(),
            ))
    });
    Ok(devices)
}

pub(crate) fn read_alsa_card_names() -> Result<HashMap<i32, String>> {
    let contents = fs::read_to_string("/proc/asound/cards")
        .context("failed to read /proc/asound/cards for ALSA card enumeration")?;
    let mut cards = HashMap::new();

    for line in contents.lines() {
        let Some((card_index, card_name)) = parse_alsa_card_line(line) else {
            continue;
        };
        cards.insert(card_index, card_name);
    }

    Ok(cards)
}

pub(crate) fn parse_alsa_card_line(line: &str) -> Option<(i32, String)> {
    let trimmed = line.trim_start();
    let index_end = trimmed.find(char::is_whitespace)?;
    let card_index = trimmed[..index_end].parse::<i32>().ok()?;
    let bracket_start = trimmed.find('[')?;
    let bracket_end = trimmed[bracket_start + 1..].find(']')? + bracket_start + 1;
    let short_name = trimmed[bracket_start + 1..bracket_end].trim();
    let description = trimmed[bracket_end + 1..].strip_prefix(':')?.trim();
    let card_name = description
        .rsplit_once(" - ")
        .map(|(_, display)| display.trim())
        .filter(|display| !display.is_empty())
        .unwrap_or(short_name);
    Some((card_index, card_name.to_string()))
}

pub(crate) fn parse_alsa_pcm_line(line: &str) -> Option<(i32, i32, String, bool)> {
    let (prefix, rest) = line.split_once(':')?;
    let (card, device) = prefix.split_once('-')?;
    let card_index = card.trim().parse::<i32>().ok()?;
    let device_index = device.trim().parse::<i32>().ok()?;
    let segments = rest.split(" : ").map(str::trim).collect::<Vec<_>>();
    if segments.len() < 3 {
        return None;
    }

    let pcm_name = segments[1].to_string();
    let has_playback = segments
        .iter()
        .skip(2)
        .any(|segment| segment.contains("playback"));
    Some((card_index, device_index, pcm_name, has_playback))
}

pub(crate) fn select_alsa_output_device(selector: Option<&str>) -> Result<AlsaOutputDevice> {
    let devices = collect_alsa_output_devices()?;
    if devices.is_empty() {
        return Err(anyhow!("no ALSA hw playback devices available"));
    }

    let selector = selector.context(
        "audio output device is required; run `mamut-standalone list-audio` and choose an ALSA `hw:<card>,<device>` selector",
    )?;

    if let Ok(index) = selector.parse::<usize>() {
        return devices
            .into_iter()
            .nth(index)
            .ok_or_else(|| {
                anyhow!(
                    "audio output device index {index} is out of range; run `mamut-standalone list-audio`"
                )
            });
    }

    let normalized = normalize_alsa_hw_selector(selector)?;
    devices
        .into_iter()
        .find(|device| device.selector == normalized)
        .ok_or_else(|| anyhow!("no ALSA hw playback device matched `{normalized}`"))
}

pub(crate) fn normalize_alsa_hw_selector(selector: &str) -> Result<String> {
    let trimmed = selector.trim();
    let Some(rest) = trimmed.strip_prefix("hw:") else {
        return Err(anyhow!(
            "invalid audio selector `{selector}`; expected a list index or ALSA `hw:<card>,<device>`"
        ));
    };

    let (card, device) = rest
        .split_once(',')
        .context("ALSA hw selector must look like `hw:<card>,<device>`")?;
    if device.contains(',') {
        return Err(anyhow!(
            "invalid ALSA hw selector `{selector}`; expected exactly one card/device pair"
        ));
    }

    let card_index = card
        .parse::<i32>()
        .with_context(|| format!("invalid ALSA card index `{card}`"))?;
    let device_index = device
        .parse::<i32>()
        .with_context(|| format!("invalid ALSA device index `{device}`"))?;

    if card_index < 0 || device_index < 0 {
        return Err(anyhow!(
            "invalid ALSA hw selector `{selector}`; card and device indexes must be non-negative"
        ));
    }

    Ok(format!("hw:{card_index},{device_index}"))
}

pub(crate) fn same_hw_selector(current: Option<&str>, replacement: &str) -> bool {
    let Some(current) = current else {
        return false;
    };
    normalize_alsa_hw_selector(current).ok() == normalize_alsa_hw_selector(replacement).ok()
}

pub(crate) fn open_alsa_playback_device(
    selected_device: &AlsaOutputDevice,
    sample_rate_hz: u32,
    tuning: AlsaPlaybackTuning,
) -> Result<OpenedAlsaPlayback> {
    let pcm =
        PCM::new(&selected_device.selector, Direction::Playback, false).with_context(|| {
            format!(
                "failed to open ALSA playback device {}",
                selected_device.selector
            )
        })?;

    let sample_format;
    {
        let hwp = HwParams::any(&pcm).context("failed to allocate ALSA hw params")?;
        hwp.set_rate_resample(false)
            .context("failed to disable ALSA resampling")?;
        hwp.set_channels(ALSA_PLAYBACK_CHANNELS as u32)
            .context("failed to set ALSA channel count")?;
        hwp.set_rate(sample_rate_hz, ValueOr::Nearest)
            .context("failed to set ALSA sample rate")?;
        sample_format =
            select_alsa_playback_sample_format(&hwp, selected_device.selector.as_str())?;
        hwp.set_access(Access::RWInterleaved)
            .context("failed to set ALSA access mode")?;
        hwp.set_period_size(tuning.period_frames as Frames, ValueOr::Nearest)
            .context("failed to set ALSA period size")?;
        hwp.set_buffer_size(tuning.buffer_frames as Frames)
            .context("failed to set ALSA buffer size")?;
        pcm.hw_params(&hwp)
            .context("failed to apply ALSA hw params")?;
    }

    let (
        applied_rate,
        applied_channels,
        applied_format,
        applied_period_frames,
        applied_buffer_frames,
    ) = {
        let applied_hwp = pcm
            .hw_params_current()
            .context("failed to read applied ALSA hw params")?;
        let applied_rate = applied_hwp
            .get_rate()
            .context("failed to read ALSA sample rate")?;
        let applied_channels = applied_hwp
            .get_channels()
            .context("failed to read ALSA channel count")? as usize;
        let applied_format = applied_hwp
            .get_format()
            .context("failed to read ALSA sample format")?;
        let applied_period_frames = applied_hwp
            .get_period_size()
            .context("failed to read ALSA period size")?
            as usize;
        let applied_buffer_frames = applied_hwp
            .get_buffer_size()
            .context("failed to read ALSA buffer size")?
            as usize;
        (
            applied_rate,
            applied_channels,
            applied_format,
            applied_period_frames,
            applied_buffer_frames,
        )
    };

    if applied_rate != sample_rate_hz {
        return Err(anyhow!(
            "ALSA device {} applied {} Hz instead of requested {} Hz",
            selected_device.selector,
            applied_rate,
            sample_rate_hz
        ));
    }
    if applied_channels != ALSA_PLAYBACK_CHANNELS {
        return Err(anyhow!(
            "ALSA device {} applied {} channels instead of requested {}",
            selected_device.selector,
            applied_channels,
            ALSA_PLAYBACK_CHANNELS
        ));
    }
    if applied_format != sample_format.alsa_format() {
        return Err(anyhow!(
            "ALSA device {} applied {:?} instead of requested {:?}",
            selected_device.selector,
            applied_format,
            sample_format.alsa_format()
        ));
    }
    if applied_period_frames != tuning.period_frames {
        return Err(anyhow!(
            "ALSA device {} applied period {} instead of requested {}",
            selected_device.selector,
            applied_period_frames,
            tuning.period_frames
        ));
    }
    if applied_buffer_frames != tuning.buffer_frames {
        return Err(anyhow!(
            "ALSA device {} applied buffer {} instead of requested {}",
            selected_device.selector,
            applied_buffer_frames,
            tuning.buffer_frames
        ));
    }

    {
        let swp = pcm
            .sw_params_current()
            .context("failed to read ALSA sw params")?;
        swp.set_avail_min(tuning.period_frames as Frames)
            .context("failed to set ALSA avail_min")?;
        swp.set_start_threshold(tuning.start_threshold_frames as Frames)
            .context("failed to set ALSA start threshold")?;
        pcm.sw_params(&swp)
            .context("failed to apply ALSA sw params")?;
    }
    pcm.prepare().context("failed to prepare ALSA PCM")?;

    Ok(OpenedAlsaPlayback {
        pcm,
        audio_selector: selected_device.selector.clone(),
        audio_device_name: selected_device.display_name(),
        sample_rate_hz,
        channels: ALSA_PLAYBACK_CHANNELS,
        sample_format,
        tuning,
    })
}

pub(crate) fn select_alsa_playback_sample_format(
    hwp: &HwParams<'_>,
    selector: &str,
) -> Result<AlsaPlaybackSampleFormat> {
    for candidate in [
        AlsaPlaybackSampleFormat::Float32,
        AlsaPlaybackSampleFormat::Signed32,
    ] {
        if hwp.test_format(candidate.alsa_format()).is_ok() {
            hwp.set_format(candidate.alsa_format()).with_context(|| {
                format!(
                    "failed to set ALSA sample format {} on {selector}",
                    candidate.label()
                )
            })?;
            return Ok(candidate);
        }
    }

    Err(anyhow!(
        "ALSA device {selector} supports neither F32 nor S32_LE playback sample format"
    ))
}

pub(crate) fn list_midi_devices() -> Result<()> {
    let midi_input = match MidiInput::new("mamut-standalone") {
        Ok(midi_input) => midi_input,
        Err(error) => {
            println!("MIDI support unavailable: {error}");
            return Ok(());
        }
    };
    let ports = collect_midi_ports(&midi_input)?;
    if ports.is_empty() {
        println!("no MIDI input devices found");
        return Ok(());
    }

    for (index, port) in ports.iter().enumerate() {
        println!("{index}: {}", port.name);
    }
    Ok(())
}
