use super::*;

pub(super) fn phase_mode_from_index(value: f32) -> OscPhaseMode {
    match value.round() as i32 {
        1 => OscPhaseMode::Fixed,
        2 => OscPhaseMode::FreeRun,
        _ => OscPhaseMode::Deterministic,
    }
}

pub(super) fn osc2_pitch_mode_from_index(value: f32) -> Osc2PitchMode {
    if value.round() as i32 >= 1 {
        Osc2PitchMode::Ratio
    } else {
        Osc2PitchMode::Semitone
    }
}

pub(super) fn noise_color_from_index(value: f32) -> NoiseColor {
    match value.round() as i32 {
        1 => NoiseColor::Pinkish,
        2 => NoiseColor::Dark,
        3 => NoiseColor::Bright,
        _ => NoiseColor::White,
    }
}

pub(super) fn filter_model_from_index(value: f32) -> FilterModel {
    match value.round() as i32 {
        1 => FilterModel::TptClean,
        2 => FilterModel::MatterDriven,
        _ => FilterModel::Legacy,
    }
}

pub(super) fn spectral_table_from_index(value: f32) -> SpectralTable {
    match value.round() as i32 {
        1 => SpectralTable::Vocalish,
        2 => SpectralTable::Metallic,
        3 => SpectralTable::Hollow,
        4 => SpectralTable::Formant,
        _ => SpectralTable::Sineish,
    }
}

pub(super) fn mod_direction_from_index(value: f32) -> ModDirection {
    if value.round() as i32 >= 1 {
        ModDirection::Osc2ToOsc1
    } else {
        ModDirection::Osc1ToOsc2
    }
}

pub(super) fn cross_mix_mode_from_index(value: f32) -> CrossMixMode {
    match value.round() as i32 {
        1 => CrossMixMode::Multiply,
        2 => CrossMixMode::Fold,
        3 => CrossMixMode::Max,
        4 => CrossMixMode::Difference,
        _ => CrossMixMode::Sum,
    }
}
