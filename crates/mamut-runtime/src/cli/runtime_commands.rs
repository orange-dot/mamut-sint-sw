use super::*;

pub fn parse_runtime_ui_command(input: &str) -> Result<RuntimeUiCommand> {
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

pub fn parse_macro_id(value: &str) -> Result<MacroId> {
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

pub fn macro_display_name(id: MacroId) -> &'static str {
    match id {
        MacroId::Gravitacija => "Gravitacija",
        MacroId::Bloom => "Bloom",
        MacroId::Heat => "Heat",
        MacroId::Ruin => "Ruin",
        MacroId::Swarm => "Swarm",
    }
}
