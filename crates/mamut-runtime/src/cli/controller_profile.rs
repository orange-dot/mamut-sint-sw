use super::*;

pub fn load_controller_profile(path: &Path) -> Result<ControllerProfile> {
    let input = fs::read_to_string(path)
        .with_context(|| format!("failed to read controller profile {}", path.display()))?;
    controller_profile_from_toml(&input, path)
}

pub fn controller_profile_from_toml(input: &str, path: &Path) -> Result<ControllerProfile> {
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

pub fn controller_binding_section(
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

pub fn infer_controller_binding_section(control: &str) -> (ControllerBindingSection, Option<u8>) {
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

pub fn parse_control_index_prefix(value: &str) -> Option<u8> {
    let digits = value
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    digits.parse::<u8>().ok()
}

pub fn controller_binding_action(
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

pub fn parse_profile_runtime_action(
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

pub fn default_scale_for_param(id: ParamId) -> ControllerValueScale {
    match param_spec(id).unit {
        ParamUnit::Hertz | ParamUnit::Milliseconds => ControllerValueScale::Log,
        _ => ControllerValueScale::Linear,
    }
}

pub fn scale_controller_value(id: ParamId, value: f32, scale: ControllerValueScale) -> f32 {
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
