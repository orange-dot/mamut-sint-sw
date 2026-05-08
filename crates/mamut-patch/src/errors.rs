use super::*;

#[derive(Debug, Error)]
pub enum PatchLoadError {
    #[error("failed to parse patch TOML: {0}")]
    Parse(#[from] toml::de::Error),
}

#[derive(Debug, Error)]
pub enum PatchSaveError {
    #[error("failed to serialize patch TOML: {0}")]
    Serialize(#[from] toml::ser::Error),
}

#[derive(Debug, Error, PartialEq)]
pub enum PatchValidationError {
    #[error("schema_version must be 1, got {found}")]
    UnsupportedSchemaVersion { found: u32 },
    #[error("patch_name must not be empty")]
    EmptyPatchName,
    #[error("tag at index {index} must not be empty")]
    EmptyTag { index: usize },
    #[error("{field} must be in range {min}..={max}, got {value}")]
    RangeViolation {
        field: &'static str,
        min: f32,
        max: f32,
        value: f32,
    },
    #[error("{field} must be in integer range {min}..={max}, got {value}")]
    IntegerRangeViolation {
        field: &'static str,
        min: i32,
        max: i32,
        value: i32,
    },
    #[error("{field} must be positive, got {value}")]
    PositiveRequired { field: &'static str, value: f32 },
    #[error("{0}")]
    PolicyViolation(&'static str),
}
