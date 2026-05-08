use super::*;

pub fn parse_play_options(args: &[String]) -> Result<PlayOptions> {
    let mut patch_arg: Option<String> = None;
    let mut force_demo = false;
    let mut audio_selector = None;
    let mut sample_rate_hz = ALSA_PLAYBACK_SAMPLE_RATE_HZ;
    let mut alsa_period_frames = None;
    let mut alsa_buffer_frames = None;
    let mut alsa_start_threshold_frames = None;
    let mut midi_selector = None;
    let mut midi_channel = None;
    let mut controller_profile_path = None;
    let mut trace_midi = false;
    let mut headless = false;
    let mut gui = false;
    let mut gfm_layer_seed = None;
    let mut bcs_layer_scenario = None;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--demo" => {
                force_demo = true;
                index += 1;
            }
            "--headless" => {
                headless = true;
                index += 1;
            }
            "--gui" => {
                gui = true;
                index += 1;
            }
            "--audio-device" => {
                let value = args
                    .get(index + 1)
                    .context("missing value after --audio-device")?;
                audio_selector = Some(value.clone());
                index += 2;
            }
            "--sample-rate" => {
                let value = args
                    .get(index + 1)
                    .context("missing value after --sample-rate")?;
                sample_rate_hz = parse_sample_rate_hz(value)?;
                index += 2;
            }
            "--alsa-period-frames" => {
                let value = args
                    .get(index + 1)
                    .context("missing value after --alsa-period-frames")?;
                alsa_period_frames = Some(parse_frame_count(value, "--alsa-period-frames")?);
                index += 2;
            }
            "--alsa-buffer-frames" => {
                let value = args
                    .get(index + 1)
                    .context("missing value after --alsa-buffer-frames")?;
                alsa_buffer_frames = Some(parse_frame_count(value, "--alsa-buffer-frames")?);
                index += 2;
            }
            "--alsa-start-threshold-frames" => {
                let value = args
                    .get(index + 1)
                    .context("missing value after --alsa-start-threshold-frames")?;
                alsa_start_threshold_frames =
                    Some(parse_frame_count(value, "--alsa-start-threshold-frames")?);
                index += 2;
            }
            "--midi-device" => {
                let value = args
                    .get(index + 1)
                    .context("missing value after --midi-device")?;
                midi_selector = Some(value.clone());
                index += 2;
            }
            "--midi-channel" => {
                let value = args
                    .get(index + 1)
                    .context("missing value after --midi-channel")?;
                midi_channel = Some(parse_midi_channel(value)?);
                index += 2;
            }
            "--controller-profile" => {
                let value = args
                    .get(index + 1)
                    .context("missing value after --controller-profile")?;
                controller_profile_path = Some(resolve_controller_profile_argument(value)?);
                index += 2;
            }
            "--trace-midi" => {
                trace_midi = true;
                index += 1;
            }
            "--gfm-layer-seed" => {
                let value = args
                    .get(index + 1)
                    .context("missing value after --gfm-layer-seed")?;
                gfm_layer_seed = Some(parse_gfm_layer_seed(value)?);
                index += 2;
            }
            "--bcs-layer-scenario" => {
                let value = args
                    .get(index + 1)
                    .context("missing value after --bcs-layer-scenario")?;
                bcs_layer_scenario = parse_optional_bcs_layer_scenario(value)?;
                index += 2;
            }
            option if option.starts_with("--") => {
                return Err(anyhow!("unknown play option `{option}`"));
            }
            patch if patch_arg.is_none() => {
                patch_arg = Some(patch.to_string());
                index += 1;
            }
            extra => {
                return Err(anyhow!(
                    "unexpected extra argument `{extra}`; pass at most one patch name or path"
                ));
            }
        }
    }

    Ok(PlayOptions {
        patch_path: resolve_patch_argument(patch_arg.as_deref())?,
        force_demo,
        audio_selector,
        sample_rate_hz,
        alsa_period_frames,
        alsa_buffer_frames,
        alsa_start_threshold_frames,
        midi_selector,
        midi_channel,
        controller_profile_path,
        trace_midi,
        headless,
        gui,
        gfm_layer_seed,
        bcs_layer_scenario,
    })
}

pub fn parse_dry_run_options(args: &[String]) -> Result<DryRunOptions> {
    let mut patch_arg: Option<String> = None;
    let mut gfm_layer_seed = None;
    let mut bcs_layer_scenario = None;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--gfm-layer-seed" => {
                let value = args
                    .get(index + 1)
                    .context("missing value after --gfm-layer-seed")?;
                gfm_layer_seed = Some(parse_gfm_layer_seed(value)?);
                index += 2;
            }
            "--bcs-layer-scenario" => {
                let value = args
                    .get(index + 1)
                    .context("missing value after --bcs-layer-scenario")?;
                bcs_layer_scenario = parse_optional_bcs_layer_scenario(value)?;
                index += 2;
            }
            option if option.starts_with("--") => {
                return Err(anyhow!("unknown dry-run option `{option}`"));
            }
            patch if patch_arg.is_none() => {
                patch_arg = Some(patch.to_string());
                index += 1;
            }
            extra => {
                return Err(anyhow!(
                    "unexpected extra argument `{extra}`; pass at most one patch name or path"
                ));
            }
        }
    }

    Ok(DryRunOptions {
        patch_path: resolve_patch_argument(patch_arg.as_deref())?,
        gfm_layer_seed,
        bcs_layer_scenario,
    })
}

pub fn parse_sample_rate_hz(value: &str) -> Result<u32> {
    let sample_rate_hz = value
        .parse::<u32>()
        .with_context(|| format!("invalid sample rate `{value}`"))?;
    if ALSA_PLAYBACK_SAMPLE_RATE_HZ_ALLOWED.contains(&sample_rate_hz) {
        Ok(sample_rate_hz)
    } else {
        Err(anyhow!(
            "unsupported sample rate `{sample_rate_hz}`; expected one of 44100, 48000, 88200, 96000, 176400, or 192000"
        ))
    }
}

pub fn parse_gfm_layer_seed(value: &str) -> Result<u64> {
    let normalized = value.replace('_', "");
    if normalized.is_empty() {
        return Err(anyhow!("invalid GFM layer seed `{value}`"));
    }

    if let Some(hex) = normalized
        .strip_prefix("0x")
        .or_else(|| normalized.strip_prefix("0X"))
    {
        if hex.is_empty() {
            return Err(anyhow!("invalid GFM layer seed `{value}`"));
        }
        u64::from_str_radix(hex, 16).with_context(|| format!("invalid GFM layer seed `{value}`"))
    } else {
        normalized
            .parse::<u64>()
            .with_context(|| format!("invalid GFM layer seed `{value}`"))
    }
}

pub fn parse_optional_bcs_layer_scenario(value: &str) -> Result<Option<BcsScenario>> {
    let normalized = value.trim().to_ascii_lowercase().replace('_', "-");
    match normalized.as_str() {
        "off" | "none" | "disabled" => Ok(None),
        _ => parse_bcs_layer_scenario(&normalized).map(Some),
    }
}

pub fn parse_bcs_layer_scenario(value: &str) -> Result<BcsScenario> {
    match value.trim().to_ascii_lowercase().replace('_', "-").as_str() {
        "stable-anchor" | "stable" | "anchor" => Ok(BcsScenario::StableAnchor),
        "edge-sweep" | "edge" => Ok(BcsScenario::EdgeSweep),
        "subharmonic-pressure" | "subharmonic" | "pressure" => Ok(BcsScenario::SubharmonicPressure),
        "recovery-return" | "recovery" | "return" => Ok(BcsScenario::RecoveryReturn),
        _ => Err(anyhow!(
            "invalid BCS layer scenario `{value}`; expected stable-anchor, edge-sweep, subharmonic-pressure, recovery-return, or off"
        )),
    }
}

pub fn format_bcs_scenario(scenario: BcsScenario) -> &'static str {
    match scenario {
        BcsScenario::StableAnchor => "stable-anchor",
        BcsScenario::EdgeSweep => "edge-sweep",
        BcsScenario::SubharmonicPressure => "subharmonic-pressure",
        BcsScenario::RecoveryReturn => "recovery-return",
    }
}

pub fn parse_frame_count(value: &str, flag: &str) -> Result<usize> {
    let frames = value
        .parse::<usize>()
        .with_context(|| format!("invalid frame count `{value}` for {flag}"))?;
    if frames == 0 {
        Err(anyhow!("{flag} expects a value greater than zero"))
    } else {
        Ok(frames)
    }
}

pub fn parse_midi_channel(value: &str) -> Result<u8> {
    let channel = value
        .parse::<u8>()
        .with_context(|| format!("invalid MIDI channel `{value}`"))?;
    if (1..=16).contains(&channel) {
        Ok(channel)
    } else {
        Err(anyhow!(
            "invalid MIDI channel `{value}`; expected a value in the range 1..16"
        ))
    }
}
