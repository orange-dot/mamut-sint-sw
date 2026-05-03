use super::*;

pub(crate) fn parse_play_options(args: &[String]) -> Result<PlayOptions> {
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
        gfm_layer_seed,
        bcs_layer_scenario,
    })
}

pub(crate) fn parse_dry_run_options(args: &[String]) -> Result<DryRunOptions> {
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

pub(crate) fn parse_sample_rate_hz(value: &str) -> Result<u32> {
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

pub(crate) fn parse_gfm_layer_seed(value: &str) -> Result<u64> {
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

pub(crate) fn parse_optional_bcs_layer_scenario(value: &str) -> Result<Option<BcsScenario>> {
    let normalized = value.trim().to_ascii_lowercase().replace('_', "-");
    match normalized.as_str() {
        "off" | "none" | "disabled" => Ok(None),
        _ => parse_bcs_layer_scenario(&normalized).map(Some),
    }
}

pub(crate) fn parse_bcs_layer_scenario(value: &str) -> Result<BcsScenario> {
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

pub(crate) fn format_bcs_scenario(scenario: BcsScenario) -> &'static str {
    match scenario {
        BcsScenario::StableAnchor => "stable-anchor",
        BcsScenario::EdgeSweep => "edge-sweep",
        BcsScenario::SubharmonicPressure => "subharmonic-pressure",
        BcsScenario::RecoveryReturn => "recovery-return",
    }
}

pub(crate) fn parse_frame_count(value: &str, flag: &str) -> Result<usize> {
    let frames = value
        .parse::<usize>()
        .with_context(|| format!("invalid frame count `{value}` for {flag}"))?;
    if frames == 0 {
        Err(anyhow!("{flag} expects a value greater than zero"))
    } else {
        Ok(frames)
    }
}

pub(crate) fn parse_midi_channel(value: &str) -> Result<u8> {
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

pub(crate) fn parse_runtime_ui_command(input: &str) -> Result<RuntimeUiCommand> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(RuntimeUiCommand::Noop);
    }

    let mut parts = trimmed.split_whitespace();
    let command = parts.next().unwrap_or_default();

    match command {
        "help" | "h" => Ok(RuntimeUiCommand::Help),
        "status" | "s" => Ok(RuntimeUiCommand::Status),
        "patches" | "list-patches" => Ok(RuntimeUiCommand::Patches),
        "favorites" | "favs" => Ok(RuntimeUiCommand::Favorites),
        "favorite" | "fav" => {
            let slot = parts
                .next()
                .context("favorite command requires a numeric slot")?;
            if parts.next().is_some() {
                return Err(anyhow!("favorite command accepts exactly one slot"));
            }
            let slot = slot
                .parse::<usize>()
                .with_context(|| format!("invalid favorite slot `{slot}`"))?;
            Ok(RuntimeUiCommand::Favorite(slot))
        }
        "patch" => {
            let argument = parts.collect::<Vec<_>>().join(" ");
            if argument.trim().is_empty() {
                Err(anyhow!("patch command requires a factory name or path"))
            } else {
                Ok(RuntimeUiCommand::Patch(argument))
            }
        }
        "next" | "next-favorite" => Ok(RuntimeUiCommand::NextFavorite),
        "prev" | "previous" | "prev-favorite" => Ok(RuntimeUiCommand::PrevFavorite),
        "demo-patch" => Ok(RuntimeUiCommand::DemoPatch),
        "macro" => {
            let macro_name = parts
                .next()
                .context("macro command requires a macro name")?;
            let value = parts.next().context("macro command requires a value")?;
            if parts.next().is_some() {
                return Err(anyhow!("macro command accepts exactly two arguments"));
            }
            let macro_id = parse_macro_id(macro_name)?;
            let value = value
                .parse::<f32>()
                .with_context(|| format!("invalid macro value `{value}`"))?;
            Ok(RuntimeUiCommand::Macro(macro_id, value.clamp(0.0, 1.0)))
        }
        "panic" => Ok(RuntimeUiCommand::Panic),
        "reset-controllers" | "reset" => Ok(RuntimeUiCommand::ResetControllers),
        "bcs" | "bcs-layer" => {
            let value = parts.next().context(
                "bcs command requires off, stable-anchor, edge-sweep, subharmonic-pressure, or recovery-return",
            )?;
            if parts.next().is_some() {
                return Err(anyhow!("bcs command accepts exactly one argument"));
            }
            Ok(RuntimeUiCommand::BcsLayer(
                parse_optional_bcs_layer_scenario(value)?,
            ))
        }
        "record" | "rec" => {
            let seconds = parts
                .next()
                .context("record command requires a duration in seconds")?;
            let seconds = seconds
                .parse::<u64>()
                .with_context(|| format!("invalid record duration `{seconds}`"))?;
            if seconds == 0 {
                return Err(anyhow!("record duration must be greater than zero seconds"));
            }
            let path = parts.collect::<Vec<_>>().join(" ");
            let path = (!path.trim().is_empty()).then(|| PathBuf::from(path));
            Ok(RuntimeUiCommand::Record { seconds, path })
        }
        "record-stop" | "rec-stop" | "stop-recording" => {
            if parts.next().is_some() {
                return Err(anyhow!("record-stop command does not accept arguments"));
            }
            Ok(RuntimeUiCommand::RecordStop)
        }
        "audio" => {
            let selector = parts.collect::<Vec<_>>().join(" ");
            if selector.trim().is_empty() {
                Ok(RuntimeUiCommand::AudioList)
            } else {
                Ok(RuntimeUiCommand::AudioSelect(selector))
            }
        }
        "midi" => {
            let selector = parts.collect::<Vec<_>>().join(" ");
            if selector.trim().is_empty() {
                Ok(RuntimeUiCommand::MidiList)
            } else {
                Ok(RuntimeUiCommand::MidiSelect(selector))
            }
        }
        "demo" => Ok(RuntimeUiCommand::Demo),
        "quit" | "exit" => Ok(RuntimeUiCommand::Quit),
        other => Err(anyhow!("unknown runtime command `{other}`")),
    }
}

pub(crate) fn parse_macro_id(value: &str) -> Result<MacroId> {
    match value.trim().to_ascii_lowercase().as_str() {
        "gravitacija" => Ok(MacroId::Gravitacija),
        "bloom" => Ok(MacroId::Bloom),
        "heat" => Ok(MacroId::Heat),
        "ruin" => Ok(MacroId::Ruin),
        "swarm" => Ok(MacroId::Swarm),
        _ => Err(anyhow!(
            "unknown macro `{value}`; expected gravitacija, bloom, heat, ruin, or swarm"
        )),
    }
}

pub(crate) fn macro_display_name(id: MacroId) -> &'static str {
    match id {
        MacroId::Gravitacija => "Gravitacija",
        MacroId::Bloom => "Bloom",
        MacroId::Heat => "Heat",
        MacroId::Ruin => "Ruin",
        MacroId::Swarm => "Swarm",
    }
}

pub(crate) fn load_controller_profile(path: &Path) -> Result<ControllerProfile> {
    let input = fs::read_to_string(path)
        .with_context(|| format!("failed to read controller profile {}", path.display()))?;
    controller_profile_from_toml(&input, path)
}

pub(crate) fn controller_profile_from_toml(input: &str, path: &Path) -> Result<ControllerProfile> {
    let file: ControllerProfileFile = toml::from_str(input)
        .with_context(|| format!("failed to parse controller profile {}", path.display()))?;
    if file.binding.is_empty() {
        return Err(anyhow!(
            "controller profile {} has no bindings",
            path.display()
        ));
    }

    let mut bindings_by_cc = HashMap::new();
    for binding in file.binding {
        let cc = binding.cc;
        let action = controller_binding_action(&binding)
            .with_context(|| format!("invalid binding `{}` on cc {cc}", binding.control))?;
        let (section, index) = controller_binding_section(&binding)?;
        let previous = bindings_by_cc.insert(
            cc,
            ControllerBinding {
                cc,
                control: binding.control,
                section,
                index,
                action,
            },
        );
        if let Some(previous) = previous {
            return Err(anyhow!(
                "controller profile {} maps cc {cc} more than once (`{}` and another binding)",
                path.display(),
                previous.control
            ));
        }
    }

    Ok(ControllerProfile {
        name: file.name.unwrap_or_else(|| {
            path.file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("controller-profile")
                .to_string()
        }),
        path: path.to_path_buf(),
        bindings_by_cc,
    })
}

pub(crate) fn controller_binding_section(
    binding: &ControllerBindingFile,
) -> Result<(ControllerBindingSection, Option<u8>)> {
    let inferred = infer_controller_binding_section(&binding.control);
    let section = binding.section.unwrap_or(inferred.0);
    let index = binding.index.or(inferred.1);
    if let Some(index) = index {
        if index == 0 {
            return Err(anyhow!(
                "controller binding index must be greater than zero"
            ));
        }
        if matches!(
            section,
            ControllerBindingSection::Knob
                | ControllerBindingSection::Slider
                | ControllerBindingSection::Switch
        ) && index > 9
        {
            return Err(anyhow!(
                "controller binding index {index} is out of range for section {section:?}"
            ));
        }
    }
    Ok((section, index))
}

pub(crate) fn infer_controller_binding_section(
    control: &str,
) -> (ControllerBindingSection, Option<u8>) {
    let trimmed = control.trim();
    if let Some(rest) = trimmed.strip_prefix("SW") {
        return (
            ControllerBindingSection::Switch,
            parse_control_index_prefix(rest),
        );
    }
    if let Some(rest) = trimmed.strip_prefix('K') {
        return (
            ControllerBindingSection::Knob,
            parse_control_index_prefix(rest),
        );
    }
    if let Some(rest) = trimmed.strip_prefix('S') {
        return (
            ControllerBindingSection::Slider,
            parse_control_index_prefix(rest),
        );
    }
    (ControllerBindingSection::Other, None)
}

pub(crate) fn parse_control_index_prefix(value: &str) -> Option<u8> {
    let digits = value
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    digits.parse::<u8>().ok()
}

pub(crate) fn controller_binding_action(
    binding: &ControllerBindingFile,
) -> Result<ControllerBindingAction> {
    match binding.kind {
        ControllerBindingKind::Macro => {
            let target = binding
                .target
                .as_deref()
                .ok_or_else(|| anyhow!("macro binding requires target"))?;
            Ok(ControllerBindingAction::Macro(parse_macro_id(target)?))
        }
        ControllerBindingKind::DirectParam => {
            let target = binding
                .target
                .as_deref()
                .ok_or_else(|| anyhow!("direct_param binding requires target"))?;
            let id = param_by_key(target)
                .ok_or_else(|| anyhow!("unknown direct_param target `{target}`"))?;
            let spec = param_spec(id);
            if spec.unit == ParamUnit::Boolean {
                return Err(anyhow!(
                    "direct_param target `{target}` is boolean; use toggle_param"
                ));
            }
            Ok(ControllerBindingAction::DirectParam {
                id,
                scale: binding.scale.unwrap_or_else(|| default_scale_for_param(id)),
            })
        }
        ControllerBindingKind::GfmLayerAmount => Ok(ControllerBindingAction::GfmLayerAmount),
        ControllerBindingKind::BcsLayerAmount | ControllerBindingKind::BcsLayerGain => {
            Ok(ControllerBindingAction::BcsLayerAmount)
        }
        ControllerBindingKind::BcsLayerEnabled => Ok(ControllerBindingAction::BcsLayerEnabled),
        ControllerBindingKind::RuntimeAction => {
            let action = binding
                .action
                .as_deref()
                .ok_or_else(|| anyhow!("runtime_action binding requires action"))?;
            Ok(ControllerBindingAction::Runtime(
                parse_profile_runtime_action(action, binding.slot)?,
            ))
        }
        ControllerBindingKind::ToggleParam => {
            let target = binding
                .target
                .as_deref()
                .ok_or_else(|| anyhow!("toggle_param binding requires target"))?;
            let id =
                param_by_key(target).ok_or_else(|| anyhow!("unknown toggle target `{target}`"))?;
            let spec = param_spec(id);
            if spec.unit != ParamUnit::Boolean {
                return Err(anyhow!(
                    "toggle_param target `{target}` is not a boolean parameter"
                ));
            }
            Ok(ControllerBindingAction::ToggleParam(id))
        }
        ControllerBindingKind::Reserved => Ok(ControllerBindingAction::Reserved),
    }
}

pub(crate) fn parse_profile_runtime_action(
    action: &str,
    slot: Option<usize>,
) -> Result<RuntimeControlMessage> {
    match action.trim().to_ascii_lowercase().as_str() {
        "panic" => Ok(RuntimeControlMessage::Panic),
        "reset_controllers" | "reset-controllers" | "reset" => {
            Ok(RuntimeControlMessage::ResetControllers)
        }
        "next_favorite" | "next" => Ok(RuntimeControlMessage::NextFavorite),
        "prev_favorite" | "previous_favorite" | "prev" | "previous" => {
            Ok(RuntimeControlMessage::PrevFavorite)
        }
        "favorite_slot" | "favorite" => {
            let slot = slot.ok_or_else(|| anyhow!("favorite_slot action requires slot"))?;
            if slot < LIVE_SET_STEMS.len() {
                Ok(RuntimeControlMessage::FavoriteSlot(slot))
            } else {
                Err(anyhow!("favorite slot {slot} is out of range"))
            }
        }
        _ => Err(anyhow!("unknown runtime action `{action}`")),
    }
}

pub(crate) fn default_scale_for_param(id: ParamId) -> ControllerValueScale {
    match param_spec(id).unit {
        ParamUnit::Hertz | ParamUnit::Milliseconds => ControllerValueScale::Log,
        _ => ControllerValueScale::Linear,
    }
}

pub(crate) fn scale_controller_value(id: ParamId, value: f32, scale: ControllerValueScale) -> f32 {
    let spec = param_spec(id);
    let normalized = value.clamp(0.0, 1.0);
    match scale {
        ControllerValueScale::Linear => spec.min + normalized * (spec.max - spec.min),
        ControllerValueScale::Log if spec.min > 0.0 && spec.max > spec.min => {
            let min = spec.min.ln();
            let max = spec.max.ln();
            (min + normalized * (max - min)).exp()
        }
        ControllerValueScale::Log => spec.min + normalized * (spec.max - spec.min),
    }
}

pub(crate) fn load_patch_from_path(path: &Path) -> Result<PatchFileV1> {
    let input = fs::read_to_string(path)
        .with_context(|| format!("failed to read patch file {}", path.display()))?;
    load_patch_toml(&input)
        .with_context(|| format!("failed to parse patch file {}", path.display()))
}

pub(crate) fn resolve_patch_argument(argument: Option<&str>) -> Result<PathBuf> {
    let Some(argument) = argument else {
        return Ok(default_patch_path());
    };

    let direct_path = PathBuf::from(argument);
    if direct_path.exists() {
        return Ok(direct_path);
    }

    let factory_dir = workspace_root().join("patches/factory");
    let factory_candidate = if argument.ends_with(".toml") {
        factory_dir.join(argument)
    } else {
        factory_dir.join(format!("{argument}.toml"))
    };
    if factory_candidate.exists() {
        return Ok(factory_candidate);
    }

    let argument_slug = slugify(argument);
    for entry in factory_patch_entries()? {
        if entry.stem.eq_ignore_ascii_case(argument) || entry.slug == argument_slug {
            return Ok(entry.path);
        }
    }

    Err(anyhow!(
        "unknown patch `{argument}`; use a path or run `list-factory` for available factory names"
    ))
}

pub(crate) fn resolve_controller_profile_argument(argument: &str) -> Result<PathBuf> {
    let direct_path = PathBuf::from(argument);
    if direct_path.exists() {
        return Ok(direct_path);
    }

    let workspace_path = workspace_root().join(argument);
    if workspace_path.exists() {
        return Ok(workspace_path);
    }

    Err(anyhow!(
        "unknown controller profile `{argument}`; use a path relative to the current directory or repository root"
    ))
}

pub(crate) fn factory_patch_entries() -> Result<Vec<FactoryPatchEntry>> {
    let factory_dir = workspace_root().join("patches/factory");
    let mut entries = Vec::new();

    for entry in fs::read_dir(&factory_dir).with_context(|| {
        format!(
            "failed to read factory patch directory {}",
            factory_dir.display()
        )
    })? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }

        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("toml") {
            continue;
        }

        let patch = load_patch_from_path(&path)?;
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or_else(|| anyhow!("invalid factory patch filename {}", path.display()))?
            .to_string();
        let favorite = patch
            .ui
            .as_ref()
            .and_then(|ui| ui.favorite)
            .unwrap_or(false);
        entries.push(FactoryPatchEntry {
            stem: stem.clone(),
            slug: slugify(&patch.meta.patch_name),
            path,
            patch_name: patch.meta.patch_name,
            description: patch.meta.description,
            favorite,
        });
    }

    entries.sort_by(|left, right| left.stem.cmp(&right.stem));
    Ok(entries)
}

pub(crate) fn favorite_patch_entries() -> Result<Vec<FactoryPatchEntry>> {
    live_patch_entries()
}

pub(crate) fn live_patch_entries() -> Result<Vec<FactoryPatchEntry>> {
    let entries = factory_patch_entries()?;
    let mut ordered = Vec::with_capacity(LIVE_SET_STEMS.len());

    for stem in LIVE_SET_STEMS {
        let Some(entry) = entries.iter().find(|entry| entry.stem == stem) else {
            return Err(anyhow!(
                "live set references missing factory patch `{stem}`"
            ));
        };
        ordered.push(entry.clone());
    }

    Ok(ordered)
}

pub(crate) fn live_patch_path(slot: usize) -> Result<PathBuf> {
    live_patch_entries()?
        .get(slot)
        .map(|entry| entry.path.clone())
        .ok_or_else(|| anyhow!("live slot {slot} is out of range"))
}

pub(crate) fn live_slot_for_path(path: &Path) -> Option<usize> {
    let current_stem = path.file_stem().and_then(|value| value.to_str())?;
    LIVE_SET_STEMS.iter().position(|stem| *stem == current_stem)
}

pub(crate) fn adjacent_live_patch(current_path: &Path, direction: isize) -> Result<PathBuf> {
    let live_set = live_patch_entries()?;
    let current_stem = current_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let current_index = live_set.iter().position(|entry| entry.stem == current_stem);

    let target_index = match current_index {
        Some(index) => wrap_index(index, direction, live_set.len()),
        None if direction >= 0 => 0,
        None => live_set.len().saturating_sub(1),
    };

    live_set
        .get(target_index)
        .map(|entry| entry.path.clone())
        .ok_or_else(|| anyhow!("live patch selection failed"))
}

pub(crate) fn wrap_index(index: usize, direction: isize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let len = len as isize;
    let index = index as isize;
    ((index + direction).rem_euclid(len)) as usize
}

pub(crate) fn collect_midi_ports(midi_input: &MidiInput) -> Result<Vec<NamedMidiPort>> {
    let mut ports = Vec::new();
    for port in midi_input.ports() {
        let name = midi_input
            .port_name(&port)
            .unwrap_or_else(|_| "unknown-midi-port".to_string());
        ports.push(NamedMidiPort { port, name });
    }
    Ok(ports)
}

pub(crate) fn open_midi_input(
    midi_input_queue: Arc<ArrayQueue<RealtimeMidiMessage>>,
    bend_range: f32,
    selector: Option<&str>,
    midi_channel: Option<u8>,
    controller_profile: Option<Arc<ControllerProfile>>,
    trace_midi: bool,
    runtime_control_queue: Arc<ArrayQueue<RuntimeControlMessage>>,
    priority_actions: Arc<PriorityActions>,
    input_metrics: Arc<InputMetrics>,
    midi_trace_log: Arc<MidiTraceLog>,
    midi_trace_publisher: MidiTracePublisher,
    sound_lab_midi_focus: Arc<SoundLabMidiFocus>,
) -> Result<Option<OpenedMidiConnection>> {
    let mut midi_input = match MidiInput::new("mamut-standalone") {
        Ok(midi_input) => midi_input,
        Err(error) => {
            return if selector.is_some() {
                Err(anyhow!("failed to create MIDI input: {error}"))
            } else {
                Ok(None)
            };
        }
    };
    midi_input.ignore(Ignore::None);
    let ports = collect_midi_ports(&midi_input)?;

    if ports.is_empty() {
        return if selector.is_some() {
            Err(anyhow!("no MIDI input devices available"))
        } else {
            Ok(None)
        };
    }

    let port_index = if let Some(selector) = selector {
        let names: Vec<String> = ports.iter().map(|entry| entry.name.clone()).collect();
        select_named_index(selector, &names, "MIDI input device")?
    } else {
        0
    };

    let selected = ports
        .into_iter()
        .nth(port_index)
        .ok_or_else(|| anyhow!("MIDI input device index {port_index} is out of range"))?;
    let port_name = selected.name.clone();
    let trace_started_at = Instant::now();
    let mut previous_trace_at: Option<Instant> = None;
    let startup_guard_until = trace_started_at
        .checked_add(MIDI_STARTUP_GUARD)
        .unwrap_or_else(Instant::now);
    let connection = midi_input
        .connect(
            &selected.port,
            "mamut-midi-in",
            move |_stamp, message, _| {
                let received_at = Instant::now();
                let parsed = parse_midi_message(
                    message,
                    bend_range,
                    midi_channel,
                    controller_profile.as_deref(),
                );
                let startup_suppressed =
                    startup_guard_suppresses_message(parsed, received_at, startup_guard_until);
                let overlay = if startup_suppressed || parsed.is_none() {
                    None
                } else {
                    sound_lab_midi_overlay_events(
                        message,
                        controller_profile.as_deref(),
                        sound_lab_midi_focus.page(),
                    )
                };
                let routed = if startup_suppressed { None } else { parsed };
                if trace_midi
                    || midi_trace_log.is_active()
                    || midi_message_can_update_last_control(
                        message,
                        midi_channel,
                        controller_profile.as_deref(),
                    )
                {
                    let trace_timing = MidiTraceTiming::from_received_at(
                        trace_started_at,
                        &mut previous_trace_at,
                        received_at,
                    );
                    midi_trace_publisher.publish(RawMidiTraceRecord::new(
                        message,
                        received_at,
                        trace_timing,
                        parsed,
                        overlay,
                        startup_suppressed,
                    ));
                }
                if let Some(overlay) = overlay {
                    input_metrics.record_midi_message();
                    for event in overlay.iter() {
                        publish_realtime_midi(
                            &midi_input_queue,
                            &input_metrics,
                            RealtimeMidiMessage::Controller(event),
                        );
                    }
                } else if let Some(parsed) = routed {
                    input_metrics.record_midi_message();
                    match parsed {
                        ParsedMidiMessage::Realtime(message) => {
                            publish_realtime_midi(&midi_input_queue, &input_metrics, message);
                        }
                        ParsedMidiMessage::Runtime(command) => {
                            publish_runtime_control(
                                &runtime_control_queue,
                                &priority_actions,
                                &input_metrics,
                                command,
                            );
                        }
                        ParsedMidiMessage::Reserved => {}
                    }
                }
            },
            (),
        )
        .map_err(|error| anyhow!("failed to open MIDI input connection: {error}"))?;

    Ok(Some(OpenedMidiConnection {
        port_name,
        _connection: connection,
    }))
}

pub(crate) fn midi_message_can_update_last_control(
    message: &[u8],
    midi_channel: Option<u8>,
    controller_profile: Option<&ControllerProfile>,
) -> bool {
    let Some(raw_status) = message.first().copied() else {
        return false;
    };
    let status = raw_status & 0xF0;
    let channel = (raw_status & 0x0F) + 1;
    if status < 0xF0 && midi_channel.is_some_and(|expected| channel != expected) {
        return true;
    }
    match status {
        0xB0 if message.len() >= 3 => {
            let cc = message[1];
            controller_profile
                .and_then(|profile| profile.binding_for_cc(cc))
                .is_some()
                || matches!(cc, 1 | 64 | 16 | 17 | 18 | 19 | 20)
        }
        0xC0 if message.len() >= 2 => true,
        0xD0 if message.len() >= 2 => true,
        0xE0 if message.len() >= 3 => true,
        _ => false,
    }
}

pub(crate) fn publish_realtime_midi(
    midi_input_queue: &ArrayQueue<RealtimeMidiMessage>,
    input_metrics: &InputMetrics,
    message: RealtimeMidiMessage,
) {
    if midi_input_queue.push(message).is_ok() {
        input_metrics.record_midi_message_accepted();
    } else {
        input_metrics.record_midi_message_dropped();
    }
}

pub(crate) fn publish_runtime_control(
    runtime_control_queue: &ArrayQueue<RuntimeControlMessage>,
    priority_actions: &PriorityActions,
    input_metrics: &InputMetrics,
    command: RuntimeControlMessage,
) {
    match command {
        RuntimeControlMessage::Panic => {
            priority_actions.request_panic();
            input_metrics.record_midi_message_accepted();
        }
        RuntimeControlMessage::ResetControllers => {
            priority_actions.request_reset_controllers();
            input_metrics.record_midi_message_accepted();
        }
        command => {
            if runtime_control_queue.push(command).is_ok() {
                input_metrics.record_midi_message_accepted();
            } else {
                input_metrics.record_runtime_control_dropped();
            }
        }
    }
}

pub(crate) fn startup_guard_suppresses_message(
    parsed: Option<ParsedMidiMessage>,
    received_at: Instant,
    guard_until: Instant,
) -> bool {
    if received_at >= guard_until {
        return false;
    }

    matches!(
        parsed,
        Some(ParsedMidiMessage::Realtime(_)) | Some(ParsedMidiMessage::Runtime(_))
    )
}

pub(crate) fn format_midi_trace_startup_suppressed(
    message: &[u8],
    midi_channel: Option<u8>,
    controller_profile: Option<&ControllerProfile>,
    parsed: Option<ParsedMidiMessage>,
    timing: MidiTraceTiming,
) -> String {
    format_midi_trace_message_with_prefix(
        "startup suppressed ",
        message,
        midi_channel,
        controller_profile,
        parsed,
        timing,
    )
}

pub(crate) fn format_midi_trace_message(
    message: &[u8],
    midi_channel: Option<u8>,
    controller_profile: Option<&ControllerProfile>,
    parsed: Option<ParsedMidiMessage>,
    timing: MidiTraceTiming,
) -> String {
    format_midi_trace_message_with_prefix(
        "",
        message,
        midi_channel,
        controller_profile,
        parsed,
        timing,
    )
}

pub(crate) fn format_midi_trace_message_with_prefix(
    verdict_prefix: &str,
    message: &[u8],
    midi_channel: Option<u8>,
    controller_profile: Option<&ControllerProfile>,
    parsed: Option<ParsedMidiMessage>,
    timing: MidiTraceTiming,
) -> String {
    let raw_status = *message.first().unwrap_or(&0);
    let status = raw_status & 0xF0;
    let channel = (raw_status & 0x0F) + 1;
    let raw = message
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ");

    let channel_filtered =
        status < 0xF0 && midi_channel.is_some_and(|expected| channel != expected);
    let verdict = if channel_filtered {
        format!(
            "filtered channel {channel} (expected channel {})",
            midi_channel.unwrap_or(channel)
        )
    } else if let Some(binding) = profile_binding_for_message(message, controller_profile) {
        describe_profile_cc_message(binding, message[2] as f32 / 127.0, parsed)
    } else if let Some(parsed) = parsed {
        describe_parsed_midi_message(parsed)
    } else {
        describe_unparsed_midi_message(message)
    };
    let verdict = format!("{verdict_prefix}{verdict}");

    if status < 0xF0 {
        format!(
            "midi trace: t={:.3}s dt={:.1}ms ch={channel} raw=[{raw}] {verdict}",
            timing.elapsed_seconds, timing.delta_millis
        )
    } else {
        format!(
            "midi trace: t={:.3}s dt={:.1}ms system raw=[{raw}] {verdict}",
            timing.elapsed_seconds, timing.delta_millis
        )
    }
}

pub(crate) fn last_control_event(
    message: &[u8],
    midi_channel: Option<u8>,
    controller_profile: Option<&ControllerProfile>,
    parsed: Option<ParsedMidiMessage>,
    received_at: Instant,
    startup_suppressed: bool,
) -> Option<LastControlEvent> {
    let raw_status = *message.first()?;
    let status = raw_status & 0xF0;
    let channel = (raw_status & 0x0F) + 1;
    if status < 0xF0 && midi_channel.is_some_and(|expected| channel != expected) {
        return Some(LastControlEvent {
            kind: LastControlKind::Other,
            label: format!("channel {channel}"),
            action: "filtered".to_string(),
            raw_status,
            channel: Some(channel),
            raw_value: message.get(2).map(|value| *value as f32 / 127.0),
            program: None,
            verdict: LastControlVerdict::Filtered,
            received_at,
        });
    }

    match status {
        0xB0 if message.len() >= 3 => {
            let cc = message[1];
            let raw_value = message[2] as f32 / 127.0;
            if let Some(binding) = controller_profile.and_then(|profile| profile.binding_for_cc(cc))
            {
                let verdict = if startup_suppressed {
                    LastControlVerdict::StartupSuppressed
                } else {
                    match (binding.action, parsed) {
                        (ControllerBindingAction::Reserved, _)
                        | (_, Some(ParsedMidiMessage::Reserved)) => LastControlVerdict::Reserved,
                        (ControllerBindingAction::Runtime(_), None)
                        | (ControllerBindingAction::ToggleParam(_), None) => {
                            LastControlVerdict::ReleaseIgnored
                        }
                        (_, Some(_)) => LastControlVerdict::Accepted,
                        (_, None) => LastControlVerdict::Ignored,
                    }
                };
                return Some(LastControlEvent {
                    kind: LastControlKind::ProfileCc(cc),
                    label: binding.control.clone(),
                    action: describe_binding_action(binding.action),
                    raw_status,
                    channel: Some(channel),
                    raw_value: Some(raw_value),
                    program: None,
                    verdict,
                    received_at,
                });
            }

            let (label, action) = match cc {
                1 => ("Mod Wheel".to_string(), "mod wheel".to_string()),
                64 => ("Sustain".to_string(), "sustain".to_string()),
                16 => ("Legacy CC16".to_string(), "macro Gravitacija".to_string()),
                17 => ("Legacy CC17".to_string(), "macro Bloom".to_string()),
                18 => ("Legacy CC18".to_string(), "macro Heat".to_string()),
                19 => ("Legacy CC19".to_string(), "macro Ruin".to_string()),
                20 => ("Legacy CC20".to_string(), "macro Swarm".to_string()),
                _ => return None,
            };
            Some(LastControlEvent {
                kind: LastControlKind::LegacyCc(cc),
                label,
                action,
                raw_status,
                channel: Some(channel),
                raw_value: Some(raw_value),
                program: None,
                verdict: if startup_suppressed {
                    LastControlVerdict::StartupSuppressed
                } else if parsed.is_some() {
                    LastControlVerdict::Accepted
                } else {
                    LastControlVerdict::Ignored
                },
                received_at,
            })
        }
        0xC0 if message.len() >= 2 => Some(LastControlEvent {
            kind: LastControlKind::ProgramChange(message[1]),
            label: "Program Change".to_string(),
            action: format!("live slot {}", message[1]),
            raw_status,
            channel: Some(channel),
            raw_value: None,
            program: Some(message[1]),
            verdict: if startup_suppressed {
                LastControlVerdict::StartupSuppressed
            } else if parsed.is_some() {
                LastControlVerdict::Accepted
            } else {
                LastControlVerdict::Ignored
            },
            received_at,
        }),
        0xD0 if message.len() >= 2 => Some(LastControlEvent {
            kind: LastControlKind::Other,
            label: "Channel Aftertouch".to_string(),
            action: "aftertouch".to_string(),
            raw_status,
            channel: Some(channel),
            raw_value: Some(message[1] as f32 / 127.0),
            program: None,
            verdict: if startup_suppressed {
                LastControlVerdict::StartupSuppressed
            } else {
                LastControlVerdict::Accepted
            },
            received_at,
        }),
        0xE0 if message.len() >= 3 => Some(LastControlEvent {
            kind: LastControlKind::Other,
            label: "Pitch Bend".to_string(),
            action: "pitch bend".to_string(),
            raw_status,
            channel: Some(channel),
            raw_value: None,
            program: None,
            verdict: if startup_suppressed {
                LastControlVerdict::StartupSuppressed
            } else {
                LastControlVerdict::Accepted
            },
            received_at,
        }),
        _ => None,
    }
}

pub(crate) fn profile_binding_for_message<'a>(
    message: &[u8],
    controller_profile: Option<&'a ControllerProfile>,
) -> Option<&'a ControllerBinding> {
    let raw_status = *message.first()?;
    let status = raw_status & 0xF0;
    if status == 0xB0 && message.len() >= 3 {
        controller_profile.and_then(|profile| profile.binding_for_cc(message[1]))
    } else {
        None
    }
}

pub(crate) fn describe_profile_cc_message(
    binding: &ControllerBinding,
    value: f32,
    parsed: Option<ParsedMidiMessage>,
) -> String {
    match (binding.action, parsed) {
        (ControllerBindingAction::Reserved, _) | (_, Some(ParsedMidiMessage::Reserved)) => {
            format!("profile {} reserved value={value:.3}", binding.control)
        }
        (ControllerBindingAction::Runtime(_), None)
        | (ControllerBindingAction::ToggleParam(_), None) => {
            format!(
                "profile {} {} release ignored value={value:.3}",
                binding.control,
                describe_binding_action(binding.action)
            )
        }
        (_, Some(parsed)) => format!(
            "profile {} {} value={value:.3} -> {}",
            binding.control,
            describe_binding_action(binding.action),
            describe_parsed_midi_message(parsed)
        ),
        (_, None) => format!(
            "profile {} {} value={value:.3}",
            binding.control,
            describe_binding_action(binding.action)
        ),
    }
}

pub(crate) fn describe_binding_action(action: ControllerBindingAction) -> String {
    match action {
        ControllerBindingAction::Macro(id) => format!("macro {}", macro_display_name(id)),
        ControllerBindingAction::DirectParam { id, .. } => {
            format!("direct param {}", param_spec(id).name)
        }
        ControllerBindingAction::GfmLayerAmount => "gfm layer gate".to_string(),
        ControllerBindingAction::BcsLayerAmount => "bcs layer gain".to_string(),
        ControllerBindingAction::BcsLayerEnabled => "bcs layer enable".to_string(),
        ControllerBindingAction::Runtime(message) => describe_runtime_control_message(message),
        ControllerBindingAction::ToggleParam(id) => format!("toggle {}", param_spec(id).name),
        ControllerBindingAction::Reserved => "reserved".to_string(),
    }
}

pub(crate) fn describe_runtime_control_message(message: RuntimeControlMessage) -> String {
    match message {
        RuntimeControlMessage::ProgramChange(slot) => format!("program change slot={slot}"),
        RuntimeControlMessage::Panic => "panic".to_string(),
        RuntimeControlMessage::ResetControllers => "reset controllers".to_string(),
        RuntimeControlMessage::NextFavorite => "next favorite".to_string(),
        RuntimeControlMessage::PrevFavorite => "previous favorite".to_string(),
        RuntimeControlMessage::FavoriteSlot(slot) => format!("favorite slot={slot}"),
        RuntimeControlMessage::ToggleParam(id) => format!("toggle {}", param_spec(id).name),
    }
}

pub(crate) fn describe_parsed_midi_message(parsed: ParsedMidiMessage) -> String {
    match parsed {
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Note(NoteEvent::NoteOn {
            note,
            velocity,
        })) => format!("note on note={note} velocity={velocity:.3}"),
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Note(NoteEvent::NoteOff { note })) => {
            format!("note off note={note}")
        }
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
            ControllerEvent::ModWheel { amount },
        )) => format!("mod wheel amount={amount:.3}"),
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
            ControllerEvent::Sustain { down },
        )) => format!("sustain down={down}"),
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
            ControllerEvent::ChannelAftertouch { pressure },
        )) => format!("aftertouch pressure={pressure:.3}"),
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
            ControllerEvent::PitchBend { semitones },
        )) => format!("pitch bend semitones={semitones:.3}"),
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(ControllerEvent::Macro {
            id,
            value,
        })) => format!("macro {} value={value:.3}", macro_display_name(id)),
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
            ControllerEvent::GfmLayerAmount { amount },
        )) => format!("gfm layer gate={amount:.3}"),
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
            ControllerEvent::BcsLayerAmount { amount },
        )) => format!("bcs layer gain={amount:.3}"),
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
            ControllerEvent::BcsLayerEnabled { enabled },
        )) => format!("bcs layer enable={enabled}"),
        ParsedMidiMessage::Realtime(RealtimeMidiMessage::Controller(
            ControllerEvent::DirectParam { id, value },
        )) => format!("direct param {} value={value:.3}", param_spec(id).name),
        ParsedMidiMessage::Runtime(message) => describe_runtime_control_message(message),
        ParsedMidiMessage::Reserved => "profile reserved".to_string(),
    }
}

pub(crate) fn describe_unparsed_midi_message(message: &[u8]) -> String {
    let Some(raw_status) = message.first().copied() else {
        return "ignored empty message".to_string();
    };
    let status = raw_status & 0xF0;
    match status {
        0xA0 if message.len() >= 3 => {
            format!(
                "ignored poly aftertouch note={} value={}",
                message[1], message[2]
            )
        }
        0xB0 if message.len() >= 3 => format!("ignored cc={} value={}", message[1], message[2]),
        0xF0 => "ignored system/common message".to_string(),
        _ => "ignored unsupported message".to_string(),
    }
}

pub(crate) fn select_named_index(selector: &str, names: &[String], kind: &str) -> Result<usize> {
    if let Ok(index) = selector.parse::<usize>() {
        return if index < names.len() {
            Ok(index)
        } else {
            Err(anyhow!(
                "{kind} index {index} is out of range; available count is {}",
                names.len()
            ))
        };
    }

    if let Some((index, _)) = names
        .iter()
        .enumerate()
        .find(|(_, name)| name.eq_ignore_ascii_case(selector))
    {
        return Ok(index);
    }

    let selector_lower = selector.to_ascii_lowercase();
    let matches: Vec<usize> = names
        .iter()
        .enumerate()
        .filter_map(|(index, name)| {
            name.to_ascii_lowercase()
                .contains(&selector_lower)
                .then_some(index)
        })
        .collect();

    match matches.as_slice() {
        [index] => Ok(*index),
        [] => Err(anyhow!(
            "no {kind} matched `{selector}`; run the relevant list command to inspect available devices"
        )),
        _ => Err(anyhow!(
            "selector `{selector}` is ambiguous for {kind}; use a numeric index or a more specific name"
        )),
    }
}

pub(crate) fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut last_was_hyphen = false;

    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_was_hyphen = false;
        } else if !last_was_hyphen && !slug.is_empty() {
            slug.push('-');
            last_was_hyphen = true;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    slug
}

pub(crate) fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

pub(crate) fn default_patch_path() -> PathBuf {
    workspace_root().join("patches/factory/molten-horizon.toml")
}

pub(crate) fn user_patch_dir() -> PathBuf {
    workspace_root().join("patches/user")
}

pub(crate) fn generated_user_patch_filename(patch_name: &str) -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    generated_user_patch_filename_at(patch_name, seconds)
}

pub(crate) fn generated_user_patch_filename_at(patch_name: &str, unix_seconds: u64) -> String {
    let slug = sanitize_capture_component(patch_name, "sound-lab");
    let (year, month, day, hour, minute, second) = unix_seconds_to_utc_parts(unix_seconds);
    format!("{slug}-{year:04}{month:02}{day:02}-{hour:02}{minute:02}{second:02}.toml")
}

pub(crate) fn default_output_capture_path() -> Result<PathBuf> {
    let dir = default_output_capture_dir();
    fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create capture directory {}", dir.display()))?;
    Ok(dir.join(generated_output_capture_filename()))
}

pub(crate) fn tagged_output_capture_path(
    patch_path: &Path,
    tag: &str,
    seconds: u64,
) -> Result<PathBuf> {
    let dir = default_output_capture_dir();
    fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create capture directory {}", dir.display()))?;
    Ok(dir.join(generated_tagged_output_capture_filename(
        patch_path, tag, seconds,
    )))
}

pub(crate) fn tagged_output_capture_preview(patch_path: &Path, tag: &str, seconds: u64) -> PathBuf {
    default_output_capture_dir().join(format!(
        "{}-{}-{}s-<timestamp>.wav",
        patch_capture_stem(patch_path),
        sanitize_capture_component(tag, DEFAULT_LIVE_TAKE_TAG),
        seconds
    ))
}

pub(crate) fn default_output_capture_dir() -> PathBuf {
    if let Some(value) = env::var_os(CAPTURE_DIR_ENV) {
        if !value.as_os_str().is_empty() {
            return PathBuf::from(value);
        }
    }

    let repo_root = workspace_root();
    if let Some(lab_root) = lab_root_from_repo_root(&repo_root) {
        return lab_root.join("audio-captures");
    }
    repo_root.join("audio-captures")
}

pub(crate) fn lab_root_from_repo_root(repo_root: &Path) -> Option<PathBuf> {
    let systems_dir = repo_root.parent()?;
    if systems_dir.file_name()? != "systems" {
        return None;
    }
    let workspace_dir = systems_dir.parent()?;
    if workspace_dir.file_name()? != "workspace" {
        return None;
    }
    workspace_dir.parent().map(|path| path.to_path_buf())
}

pub(crate) fn generated_output_capture_filename() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (year, month, day, hour, minute, second) = unix_seconds_to_utc_parts(seconds);
    format!("mamut-output-{year:04}{month:02}{day:02}-{hour:02}{minute:02}{second:02}.wav")
}

pub(crate) fn generated_tagged_output_capture_filename(
    patch_path: &Path,
    tag: &str,
    seconds: u64,
) -> String {
    let unix_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (year, month, day, hour, minute, second) = unix_seconds_to_utc_parts(unix_seconds);
    format!(
        "{}-{}-{}s-{year:04}{month:02}{day:02}-{hour:02}{minute:02}{second:02}.wav",
        patch_capture_stem(patch_path),
        sanitize_capture_component(tag, DEFAULT_LIVE_TAKE_TAG),
        seconds
    )
}

pub(crate) fn patch_capture_stem(patch_path: &Path) -> String {
    patch_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| sanitize_capture_component(stem, "patch"))
        .unwrap_or_else(|| "patch".to_string())
}

pub(crate) fn sanitize_capture_component(value: &str, fallback: &str) -> String {
    let mut output = String::new();
    let mut previous_dash = false;
    for character in value.chars().flat_map(char::to_lowercase) {
        let mapped = if character.is_ascii_alphanumeric() {
            Some(character)
        } else if character == '-' || character == '_' || character.is_whitespace() {
            Some('-')
        } else {
            None
        };
        let Some(mapped) = mapped else {
            continue;
        };
        if mapped == '-' {
            if previous_dash || output.is_empty() {
                continue;
            }
            previous_dash = true;
        } else {
            previous_dash = false;
        }
        output.push(mapped);
    }
    while output.ends_with('-') {
        output.pop();
    }
    if output.is_empty() {
        sanitize_capture_component(fallback, "take")
    } else {
        output
    }
}

pub(crate) fn unix_seconds_to_utc_parts(seconds: u64) -> (i32, u32, u32, u32, u32, u32) {
    let days = (seconds / 86_400) as i64;
    let seconds_of_day = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = (seconds_of_day / 3_600) as u32;
    let minute = ((seconds_of_day % 3_600) / 60) as u32;
    let second = (seconds_of_day % 60) as u32;
    (year, month, day, hour, minute, second)
}

pub(crate) fn civil_from_days(days_since_unix_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_unix_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_phase = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_phase + 2) / 5 + 1;
    let month = month_phase + if month_phase < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }
    (year as i32, month as u32, day as u32)
}

pub(crate) fn round_up_to_render_block(frames: usize) -> usize {
    if frames == 0 {
        return 0;
    }

    let remainder = frames % ENGINE_RENDER_BLOCK_FRAMES;
    if remainder == 0 {
        frames
    } else {
        frames + (ENGINE_RENDER_BLOCK_FRAMES - remainder)
    }
}
