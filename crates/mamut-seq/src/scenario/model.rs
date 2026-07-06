//! Scenario schema v1. Template = `mamut-patch/src/model.rs`: `deny_unknown_fields`,
//! `schema_version` checked first, snake_case tags, small `default_*` helpers.
//!
//! Timeline model: a `wait`, `cc_ramp`, or `aftertouch_envelope` advances the
//! timeline cursor; every other step is an instantaneous trigger at the current
//! cursor. `note`/`chord` schedule their own note-off `duration_ms` later
//! without moving the cursor (so a held chord rings while later steps fire).

use serde::{Deserialize, Serialize};

use super::errors::ScenarioLoadError;

/// `#[serde(deny_unknown_fields)]` is not honored on internally-tagged enums, so
/// a mistyped `Step` field (e.g. `velocty`) would parse and silently default.
/// This post-parse pass checks each timeline step's raw TOML keys against the
/// allowed set for its `kind`, recursing into `loop`/`section` bodies.
pub(super) fn check_unknown_step_fields(input: &str) -> Result<(), ScenarioLoadError> {
    let table: toml::Table = toml::from_str(input)?;
    if let Some(toml::Value::Array(timeline)) = table.get("timeline") {
        for entry in timeline {
            check_step_value(entry)?;
        }
    }
    Ok(())
}

fn check_step_value(value: &toml::Value) -> Result<(), ScenarioLoadError> {
    let toml::Value::Table(step) = value else {
        return Ok(());
    };
    let Some(toml::Value::String(kind)) = step.get("kind") else {
        return Ok(());
    };
    if let Some(allowed) = allowed_step_fields(kind) {
        for key in step.keys() {
            if !allowed.contains(&key.as_str()) {
                return Err(ScenarioLoadError::UnknownStepField {
                    kind: kind.clone(),
                    field: key.clone(),
                });
            }
        }
    }
    if let Some(toml::Value::Array(inner)) = step.get("steps") {
        for entry in inner {
            check_step_value(entry)?;
        }
    }
    Ok(())
}

fn allowed_step_fields(kind: &str) -> Option<&'static [&'static str]> {
    Some(match kind {
        "wait" => &["kind", "ms"],
        "note" => &["kind", "note", "velocity", "duration_ms", "channel"],
        "note_on" => &["kind", "note", "velocity", "channel"],
        "note_off" => &["kind", "note", "channel"],
        "chord" => &["kind", "notes", "velocity", "duration_ms", "channel"],
        "cc" => &["kind", "control", "value", "channel"],
        "cc_ramp" => &[
            "kind",
            "control",
            "from",
            "to",
            "duration_ms",
            "steps",
            "channel",
        ],
        "aftertouch" => &["kind", "pressure", "channel"],
        "aftertouch_envelope" => &[
            "kind",
            "peak",
            "attack_ms",
            "hold_ms",
            "release_ms",
            "steps",
            "channel",
        ],
        "pitch_bend" => &["kind", "value", "channel"],
        "mod_wheel" => &["kind", "value", "channel"],
        "sustain" => &["kind", "down", "channel"],
        "loop" => &["kind", "count", "steps"],
        "section" => &["kind", "name", "steps"],
        _ => return None,
    })
}

/// Default note velocity when a step omits it.
pub fn default_velocity() -> u8 {
    100
}

/// Default sounding length for `note`/`chord` steps.
pub fn default_duration_ms() -> u64 {
    500
}

/// Default number of interpolation points for ramps/envelopes.
pub fn default_ramp_steps() -> u32 {
    16
}

/// A full scenario file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioFile {
    pub schema_version: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Channel override; falls back to the CLI `--channel` when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<u8>,
    /// Reserved for future stochastic gestures; v1 expansion is deterministic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    #[serde(default)]
    pub timeline: Vec<Step>,
}

/// A timeline step. Internally tagged on `kind`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Step {
    /// Advance the cursor by `ms`.
    Wait { ms: u64 },
    /// Trigger a note now and auto-release it `duration_ms` later.
    Note {
        note: u8,
        #[serde(default = "default_velocity")]
        velocity: u8,
        #[serde(default = "default_duration_ms")]
        duration_ms: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        channel: Option<u8>,
    },
    /// Trigger a note-on now (no auto-release).
    NoteOn {
        note: u8,
        #[serde(default = "default_velocity")]
        velocity: u8,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        channel: Option<u8>,
    },
    /// Trigger a note-off now.
    NoteOff {
        note: u8,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        channel: Option<u8>,
    },
    /// Trigger several notes now, all auto-released `duration_ms` later.
    Chord {
        notes: Vec<u8>,
        #[serde(default = "default_velocity")]
        velocity: u8,
        #[serde(default = "default_duration_ms")]
        duration_ms: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        channel: Option<u8>,
    },
    /// A single CC value (normalized `0.0..=1.0`) addressed by profile name.
    Cc {
        control: String,
        value: f32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        channel: Option<u8>,
    },
    /// A CC ramp from `from` to `to` over `duration_ms`; advances the cursor.
    CcRamp {
        control: String,
        from: f32,
        to: f32,
        duration_ms: u64,
        #[serde(default = "default_ramp_steps")]
        steps: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        channel: Option<u8>,
    },
    /// A single channel-aftertouch value (normalized `0.0..=1.0`).
    Aftertouch {
        pressure: f32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        channel: Option<u8>,
    },
    /// An attack/hold/release channel-aftertouch envelope; advances the cursor
    /// by `attack_ms + hold_ms + release_ms`.
    AftertouchEnvelope {
        peak: f32,
        attack_ms: u64,
        hold_ms: u64,
        release_ms: u64,
        #[serde(default = "default_ramp_steps")]
        steps: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        channel: Option<u8>,
    },
    /// A pitch-bend value (normalized `-1.0..=1.0`, `0.0` = centre).
    PitchBend {
        value: f32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        channel: Option<u8>,
    },
    /// Mod wheel (CC1, normalized `0.0..=1.0`). A standard performance
    /// controller honoured by the receiver's fallback map, not a profile knob.
    ModWheel {
        value: f32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        channel: Option<u8>,
    },
    /// Sustain pedal (CC64). `down = true` sends `127`, `false` sends `0`.
    Sustain {
        down: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        channel: Option<u8>,
    },
    /// A program change (`0..=7`, the locked 8-slot live set).
    ProgramChange {
        program: u8,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        channel: Option<u8>,
    },
    /// Repeat the inner steps `count` times.
    Loop { count: u32, steps: Vec<Step> },
    /// Group inner steps under a name (purely for readability).
    Section {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        steps: Vec<Step>,
    },
}
