//! Scenario schema, deterministic expansion, and playback (SET3-2).

pub(crate) mod errors;
mod expand;
mod model;
mod player;

pub use errors::ScenarioLoadError;
pub use expand::{AbsoluteEvent, expand, validate_scenario};
pub use model::ScenarioFile;
pub use player::play;

use std::path::Path;

use anyhow::{Context, Result};

/// Parse a scenario from a TOML string (structural key check only; call
/// `validate_scenario` for semantic validation).
pub fn load_from_str(input: &str) -> Result<ScenarioFile, ScenarioLoadError> {
    let scenario: ScenarioFile = toml::from_str(input).map_err(ScenarioLoadError::from)?;
    model::check_unknown_step_fields(input)?;
    Ok(scenario)
}

/// Load and parse a scenario from disk (no validation).
pub fn load_from_path(path: &Path) -> Result<ScenarioFile> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read scenario `{}`", path.display()))?;
    load_from_str(&text).with_context(|| format!("failed to parse scenario `{}`", path.display()))
}
