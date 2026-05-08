use super::*;

pub fn load_patch_from_path(path: &Path) -> Result<PatchFileV1> {
    let input = fs::read_to_string(path)
        .with_context(|| format!("failed to read patch file {}", path.display()))?;
    load_patch_toml(&input)
        .with_context(|| format!("failed to parse patch file {}", path.display()))
}

pub fn resolve_patch_argument(argument: Option<&str>) -> Result<PathBuf> {
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

pub fn resolve_controller_profile_argument(argument: &str) -> Result<PathBuf> {
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

pub fn factory_patch_entries() -> Result<Vec<FactoryPatchEntry>> {
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

pub fn favorite_patch_entries() -> Result<Vec<FactoryPatchEntry>> {
    live_patch_entries()
}

pub fn live_patch_entries() -> Result<Vec<FactoryPatchEntry>> {
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

pub fn live_patch_path(slot: usize) -> Result<PathBuf> {
    live_patch_entries()?
        .get(slot)
        .map(|entry| entry.path.clone())
        .ok_or_else(|| anyhow!("live slot {slot} is out of range"))
}

pub fn live_slot_for_path(path: &Path) -> Option<usize> {
    let current_stem = path.file_stem().and_then(|value| value.to_str())?;
    LIVE_SET_STEMS.iter().position(|stem| *stem == current_stem)
}

pub fn adjacent_live_patch(current_path: &Path, direction: isize) -> Result<PathBuf> {
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

pub fn wrap_index(index: usize, direction: isize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let len = len as isize;
    let index = index as isize;
    ((index + direction).rem_euclid(len)) as usize
}
