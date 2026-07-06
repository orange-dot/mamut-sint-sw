use thiserror::Error;

/// Parse-phase failure (TOML → `ScenarioFile`).
#[derive(Debug, Error)]
pub enum ScenarioLoadError {
    #[error("failed to parse scenario TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("unknown field `{field}` on `{kind}` step")]
    UnknownStepField { kind: String, field: String },
}

/// Semantic failure during validation or expansion. Derives `PartialEq` so
/// tests can assert exact values (matching the `mamut-patch` convention).
#[derive(Debug, Error, PartialEq)]
pub enum ScenarioError {
    #[error("schema_version must be 1, got {found}")]
    UnsupportedSchemaVersion { found: u32 },
    #[error("scenario name must not be empty")]
    EmptyName,
    #[error("timeline must not be empty")]
    EmptyTimeline,
    #[error("unknown control `{control}` in profile `{profile}`")]
    UnknownControl { control: String, profile: String },
    #[error("MIDI channel must be in range 1..=16, got {value}")]
    InvalidChannel { value: u8 },
    #[error("note {value} must be in range 0..=127")]
    NoteOutOfRange { value: u8 },
    #[error("velocity {value} must be in range 0..=127")]
    VelocityOutOfRange { value: u8 },
    #[error("program_change program must be in range 0..=7, got {value}")]
    ProgramOutOfRange { value: u8 },
    #[error("loop count must be greater than zero")]
    ZeroLoopCount,
    #[error("ramp steps must be greater than zero")]
    ZeroRampSteps,
    #[error("ramp steps {value} exceeds the limit of {limit}")]
    RampStepsTooLarge { value: u32, limit: u32 },
    #[error("chord must contain at least one note")]
    EmptyChord,
    #[error("{field} must be a finite number")]
    NonFiniteValue { field: &'static str },
    #[error("scenario expansion exceeded the step budget of {limit}")]
    ExpansionBudgetExceeded { limit: u64 },
}
