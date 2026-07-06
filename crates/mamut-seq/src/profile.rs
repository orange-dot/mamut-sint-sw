//! Minimal read-only mirror of a PC4 controller profile (`profiles/pc4-full.toml`).
//!
//! `mamut-seq` deliberately does not depend on `mamut-runtime`, so it does not
//! reuse `ControllerProfile`; that loader pulls `alsa` + the engine crates and
//! couples the sender to the runtime. Instead this reads only the fields the
//! tool needs — the control `name`, its `cc`, and (for panic hygiene) the
//! `action` — and lets serde ignore every other binding field. The profile TOML
//! file stays the single source of truth for CC numbers.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Result, anyhow};
use serde::Deserialize;

/// Raw profile file shape. Not `deny_unknown_fields`: the top-level `name`/
/// `version` and every non-`cc`/`action` binding field (`section`/`index`/
/// `kind`/`target`/`scale`/`slot`) are silently ignored.
#[derive(Debug, Deserialize)]
struct ProfileFile {
    #[serde(default)]
    binding: Vec<BindingFile>,
}

#[derive(Debug, Deserialize)]
struct BindingFile {
    control: String,
    cc: u8,
    #[serde(default)]
    action: Option<String>,
}

/// Resolved profile: a name→CC index plus the panic/reset control CCs.
#[derive(Debug, Clone)]
pub struct Profile {
    source: String,
    by_name: HashMap<String, u8>,
    /// Full control names paired with their CC, in profile file order (for the
    /// live CC lane).
    controls: Vec<(String, u8)>,
    panic_cc: Option<u8>,
    reset_controllers_cc: Option<u8>,
}

impl Profile {
    /// Load and index a profile from disk.
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .map_err(|error| anyhow!("failed to read profile `{}`: {error}", path.display()))?;
        Self::from_toml(&text, &path.display().to_string())
    }

    /// Parse and index a profile from a TOML string. `source` names the origin
    /// for error messages.
    pub fn from_toml(input: &str, source: &str) -> Result<Self> {
        let file: ProfileFile = toml::from_str(input)
            .map_err(|error| anyhow!("failed to parse profile `{source}`: {error}"))?;

        let mut by_name = HashMap::new();
        let mut controls = Vec::new();
        let mut panic_cc = None;
        let mut reset_controllers_cc = None;

        for binding in &file.binding {
            // Index by both the full control string ("S9 BCS Gain") and its
            // leading token ("S9") so a scenario may address either form.
            by_name.insert(binding.control.clone(), binding.cc);
            if let Some(token) = binding.control.split_whitespace().next() {
                by_name.entry(token.to_string()).or_insert(binding.cc);
            }
            controls.push((binding.control.clone(), binding.cc));
            match binding.action.as_deref() {
                Some("panic") => panic_cc = Some(binding.cc),
                Some("reset_controllers" | "reset-controllers" | "reset") => {
                    reset_controllers_cc = Some(binding.cc);
                }
                _ => {}
            }
        }

        if by_name.is_empty() {
            return Err(anyhow!("profile `{source}` declares no bindings"));
        }

        Ok(Self {
            source: source.to_string(),
            by_name,
            controls,
            panic_cc,
            reset_controllers_cc,
        })
    }

    /// The origin string (path) used in error messages that name the profile.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Full control names with their CC, in profile file order.
    pub fn controls(&self) -> &[(String, u8)] {
        &self.controls
    }

    /// Resolve a control name to its CC number, or `None` if unknown.
    pub fn cc_for(&self, control: &str) -> Option<u8> {
        self.by_name.get(control).copied()
    }

    /// CC that drives the receiver's own panic (profile `action = "panic"`).
    pub fn panic_cc(&self) -> Option<u8> {
        self.panic_cc
    }

    /// CC that drives the receiver's reset-controllers action.
    pub fn reset_controllers_cc(&self) -> Option<u8> {
        self.reset_controllers_cc
    }
}
