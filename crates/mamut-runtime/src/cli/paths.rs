use super::*;

pub fn slugify(value: &str) -> String {
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

pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

pub fn default_patch_path() -> PathBuf {
    workspace_root().join("patches/factory/molten-horizon.toml")
}

pub fn user_patch_dir() -> PathBuf {
    workspace_root().join("patches/user")
}

pub fn generated_user_patch_filename(patch_name: &str) -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    generated_user_patch_filename_at(patch_name, seconds)
}

pub fn generated_user_patch_filename_at(patch_name: &str, unix_seconds: u64) -> String {
    let slug = sanitize_capture_component(patch_name, "sound-lab");
    let (year, month, day, hour, minute, second) = unix_seconds_to_utc_parts(unix_seconds);
    format!("{slug}-{year:04}{month:02}{day:02}-{hour:02}{minute:02}{second:02}.toml")
}

pub fn default_output_capture_path() -> Result<PathBuf> {
    let dir = default_output_capture_dir();
    fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create capture directory {}", dir.display()))?;
    Ok(dir.join(generated_output_capture_filename()))
}

pub fn tagged_output_capture_path(patch_path: &Path, tag: &str, seconds: u64) -> Result<PathBuf> {
    let dir = default_output_capture_dir();
    fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create capture directory {}", dir.display()))?;
    Ok(dir.join(generated_tagged_output_capture_filename(
        patch_path, tag, seconds,
    )))
}

pub fn tagged_output_capture_preview(patch_path: &Path, tag: &str, seconds: u64) -> PathBuf {
    default_output_capture_dir().join(format!(
        "{}-{}-{}s-<timestamp>.wav",
        patch_capture_stem(patch_path),
        sanitize_capture_component(tag, DEFAULT_LIVE_TAKE_TAG),
        seconds
    ))
}

pub fn default_output_capture_dir() -> PathBuf {
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

pub fn lab_root_from_repo_root(repo_root: &Path) -> Option<PathBuf> {
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

pub fn generated_output_capture_filename() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (year, month, day, hour, minute, second) = unix_seconds_to_utc_parts(seconds);
    format!("mamut-output-{year:04}{month:02}{day:02}-{hour:02}{minute:02}{second:02}.wav")
}

pub fn generated_tagged_output_capture_filename(
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

pub fn patch_capture_stem(patch_path: &Path) -> String {
    patch_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| sanitize_capture_component(stem, "patch"))
        .unwrap_or_else(|| "patch".to_string())
}

pub fn sanitize_capture_component(value: &str, fallback: &str) -> String {
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

pub fn unix_seconds_to_utc_parts(seconds: u64) -> (i32, u32, u32, u32, u32, u32) {
    let days = (seconds / 86_400) as i64;
    let seconds_of_day = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = (seconds_of_day / 3_600) as u32;
    let minute = ((seconds_of_day % 3_600) / 60) as u32;
    let second = (seconds_of_day % 60) as u32;
    (year, month, day, hour, minute, second)
}

pub fn civil_from_days(days_since_unix_epoch: i64) -> (i32, u32, u32) {
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

pub fn round_up_to_render_block(frames: usize) -> usize {
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
