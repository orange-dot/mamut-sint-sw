use super::*;

pub fn sound_lab_extension_table(
    mut extensions: toml::Table,
    snapshot: &EngineSnapshot,
    source_patch_path: &Path,
) -> toml::Table {
    let mut sound_lab = toml::Table::new();
    sound_lab.insert(
        "intent_version".to_string(),
        toml::Value::String("1".to_string()),
    );
    sound_lab.insert(
        "note".to_string(),
        toml::Value::String(
            "Runtime layer intent only; GFM/BCS state is not auto-loaded from this patch."
                .to_string(),
        ),
    );
    sound_lab.insert(
        "source_patch_path".to_string(),
        toml::Value::String(source_patch_path.display().to_string()),
    );

    match snapshot.gfm_layer.mode {
        GfmLayerMode::Enabled { seed } => {
            sound_lab.insert("gfm_enabled".to_string(), toml::Value::Boolean(true));
            sound_lab.insert(
                "gfm_seed".to_string(),
                toml::Value::String(format_gfm_seed(seed)),
            );
        }
        GfmLayerMode::Disabled => {
            sound_lab.insert("gfm_enabled".to_string(), toml::Value::Boolean(false));
            sound_lab.insert(
                "gfm_seed".to_string(),
                toml::Value::String("disabled".to_string()),
            );
        }
    }

    match snapshot.bcs_layer.mode {
        BcsLayerMode::Enabled { scenario } => {
            sound_lab.insert("bcs_enabled".to_string(), toml::Value::Boolean(true));
            sound_lab.insert(
                "bcs_scenario".to_string(),
                toml::Value::String(format_bcs_scenario(scenario).to_string()),
            );
        }
        BcsLayerMode::Disabled => {
            sound_lab.insert("bcs_enabled".to_string(), toml::Value::Boolean(false));
            sound_lab.insert(
                "bcs_scenario".to_string(),
                toml::Value::String("disabled".to_string()),
            );
        }
    }
    sound_lab.insert(
        "bcs_gain".to_string(),
        toml::Value::Float(snapshot.bcs_layer.gain as f64),
    );
    sound_lab.insert(
        "bcs_effective_gain".to_string(),
        toml::Value::Float(snapshot.bcs_layer.effective_gain as f64),
    );

    extensions.insert("sound_lab".to_string(), toml::Value::Table(sound_lab));
    extensions
}
