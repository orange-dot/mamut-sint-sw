use super::*;

pub fn load_patch_toml(input: &str) -> Result<PatchFileV1, PatchLoadError> {
    toml::from_str(input).map_err(PatchLoadError::from)
}

pub fn save_patch_toml(patch: &PatchFileV1) -> Result<String, PatchSaveError> {
    toml::to_string_pretty(patch).map_err(PatchSaveError::from)
}
