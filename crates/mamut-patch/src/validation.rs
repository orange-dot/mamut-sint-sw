use super::*;

pub fn validate_patch_v1(patch: &PatchFileV1) -> Result<(), PatchValidationError> {
    if patch.meta.schema_version != 1 {
        return Err(PatchValidationError::UnsupportedSchemaVersion {
            found: patch.meta.schema_version,
        });
    }

    if patch.meta.patch_name.trim().is_empty() {
        return Err(PatchValidationError::EmptyPatchName);
    }

    if let Some(tags) = &patch.meta.tags {
        for (index, tag) in tags.iter().enumerate() {
            if tag.trim().is_empty() {
                return Err(PatchValidationError::EmptyTag { index });
            }
        }
    }

    validate_osc1(&patch.engine.osc1)?;
    validate_osc2(&patch.engine.osc2)?;
    validate_spectral(&patch.engine.spectral)?;
    validate_additive(&patch.engine.additive)?;
    validate_source_controls(&patch.engine)?;
    validate_sub(&patch.engine.sub)?;
    validate_mixer(&patch.engine.mixer)?;
    validate_filter(&patch.engine.filter)?;
    validate_adsr("engine.amp_env", &patch.engine.amp_env)?;
    validate_filter_env(&patch.engine.filter_env)?;
    validate_voice(&patch.engine.voice)?;
    validate_final_stage(&patch.engine.final_stage)?;
    validate_fx(&patch.engine.fx)?;
    validate_macro_defaults(&patch.macros)?;
    validate_macro_response_set(&patch.macro_response)?;
    validate_identity_bias(&patch.identity_bias)?;
    validate_performance_response(&patch.performance_response)?;
    validate_extensions(&patch.x)?;

    Ok(())
}

fn validate_osc1(osc1: &Osc1Patch) -> Result<(), PatchValidationError> {
    check_range("engine.osc1.saw_level", osc1.saw_level, 0.0, 1.0)?;
    check_range("engine.osc1.pulse_level", osc1.pulse_level, 0.0, 1.0)?;
    check_range("engine.osc1.triangle_level", osc1.triangle_level, 0.0, 1.0)?;
    check_range("engine.osc1.noise_level", osc1.noise_level, 0.0, 1.0)?;
    if let Some(fine_tune_cents) = osc1.fine_tune_cents {
        check_range(
            "engine.osc1.fine_tune_cents",
            fine_tune_cents,
            -100.0,
            100.0,
        )?;
    }
    check_range("engine.osc1.pulse_width", osc1.pulse_width, 0.05, 0.95)?;
    check_range("engine.osc1.pwm_depth", osc1.pwm_depth, 0.0, 1.0)?;
    check_range("engine.osc1.start_phase", osc1.start_phase, 0.0, 1.0)?;
    check_range("engine.osc1.saw_bend", osc1.saw_bend, -1.0, 1.0)?;
    check_range("engine.osc1.triangle_fold", osc1.triangle_fold, 0.0, 1.0)?;
    check_range("engine.osc1.pulse_edge", osc1.pulse_edge, 0.0, 1.0)?;

    Ok(())
}

fn validate_osc2(osc2: &Osc2Patch) -> Result<(), PatchValidationError> {
    check_range("engine.osc2.saw_level", osc2.saw_level, 0.0, 1.0)?;
    check_range("engine.osc2.pulse_level", osc2.pulse_level, 0.0, 1.0)?;
    check_range("engine.osc2.triangle_level", osc2.triangle_level, 0.0, 1.0)?;
    check_integer_range(
        "engine.osc2.interval_semitones",
        osc2.interval_semitones as i32,
        -24,
        24,
    )?;
    check_range(
        "engine.osc2.fine_tune_cents",
        osc2.fine_tune_cents,
        -100.0,
        100.0,
    )?;
    check_range("engine.osc2.sync_amount", osc2.sync_amount, 0.0, 1.0)?;
    check_range(
        "engine.osc2.crossmod_amount",
        osc2.crossmod_amount,
        0.0,
        1.0,
    )?;
    check_range("engine.osc2.pulse_width", osc2.pulse_width, 0.05, 0.95)?;
    check_range("engine.osc2.pwm_depth", osc2.pwm_depth, 0.0, 1.0)?;
    check_range("engine.osc2.start_phase", osc2.start_phase, 0.0, 1.0)?;
    check_range("engine.osc2.level", osc2.level, 0.0, 2.0)?;
    check_range("engine.osc2.ratio", osc2.ratio, 0.25, 4.0)?;
    check_range("engine.osc2.saw_bend", osc2.saw_bend, -1.0, 1.0)?;
    check_range("engine.osc2.triangle_fold", osc2.triangle_fold, 0.0, 1.0)?;
    check_range("engine.osc2.pulse_edge", osc2.pulse_edge, 0.0, 1.0)?;

    Ok(())
}

fn validate_spectral(spectral: &SpectralPatch) -> Result<(), PatchValidationError> {
    check_range("engine.spectral.level", spectral.level, 0.0, 1.0)?;
    check_range("engine.spectral.position", spectral.position, 0.0, 1.0)?;
    check_range("engine.spectral.morph", spectral.morph, 0.0, 1.0)?;
    check_range("engine.spectral.ratio", spectral.ratio, 0.25, 4.0)?;
    check_range(
        "engine.spectral.fine_tune_cents",
        spectral.fine_tune_cents,
        -100.0,
        100.0,
    )?;

    Ok(())
}

fn validate_additive(additive: &AdditivePatch) -> Result<(), PatchValidationError> {
    check_range("engine.additive.level", additive.level, 0.0, 1.0)?;
    check_integer_range(
        "engine.additive.partial_count",
        additive.partial_count as i32,
        4,
        8,
    )?;
    check_range(
        "engine.additive.harmonic_spread",
        additive.harmonic_spread,
        0.0,
        1.0,
    )?;
    check_range(
        "engine.additive.odd_even_balance",
        additive.odd_even_balance,
        -1.0,
        1.0,
    )?;
    check_range(
        "engine.additive.inharmonicity",
        additive.inharmonicity,
        0.0,
        1.0,
    )?;
    check_range(
        "engine.additive.spectral_tilt",
        additive.spectral_tilt,
        -1.0,
        1.0,
    )?;
    check_range(
        "engine.additive.random_detune_cents",
        additive.random_detune_cents,
        0.0,
        35.0,
    )?;

    Ok(())
}

fn validate_source_controls(engine: &EnginePatchDefaults) -> Result<(), PatchValidationError> {
    check_range(
        "engine.source_pwm_rate_hz",
        engine.source_pwm_rate_hz,
        0.01,
        20.0,
    )?;
    if let Some(noise_filter_level) = engine.noise_filter_level {
        check_range("engine.noise_filter_level", noise_filter_level, 0.0, 1.0)?;
    }
    check_range("engine.noise_body_level", engine.noise_body_level, 0.0, 1.0)?;
    check_range("engine.analog_drift", engine.analog_drift, 0.0, 1.0)?;
    check_range("engine.micro_jitter", engine.micro_jitter, 0.0, 1.0)?;
    check_range("engine.fm_amount", engine.fm_amount, 0.0, 1.0)?;
    check_range("engine.phase_mod_amount", engine.phase_mod_amount, 0.0, 1.0)?;
    check_range("engine.ring_mod_amount", engine.ring_mod_amount, 0.0, 1.0)?;
    check_range("engine.am_amount", engine.am_amount, 0.0, 1.0)?;
    check_range("engine.sync_softness", engine.sync_softness, 0.0, 1.0)?;
    check_range("engine.cross_mix_amount", engine.cross_mix_amount, 0.0, 1.0)?;
    Ok(())
}

fn validate_sub(sub: &SubPatch) -> Result<(), PatchValidationError> {
    check_range("engine.sub.level", sub.level, 0.0, 1.0)?;
    check_integer_range("engine.sub.octave_offset", sub.octave_offset as i32, -2, 0)?;
    Ok(())
}

fn validate_mixer(mixer: &MixerPatch) -> Result<(), PatchValidationError> {
    check_range(
        "engine.mixer.pre_filter_drive",
        mixer.pre_filter_drive,
        0.0,
        1.0,
    )?;
    check_range("engine.mixer.body_mix", mixer.body_mix, 0.0, 1.0)?;
    Ok(())
}

fn validate_filter(filter: &FilterPatch) -> Result<(), PatchValidationError> {
    check_range("engine.filter.cutoff_hz", filter.cutoff_hz, 20.0, 20_000.0)?;
    check_range("engine.filter.resonance", filter.resonance, 0.0, 1.0)?;
    check_range("engine.filter.drive", filter.drive, 0.0, 1.0)?;
    check_range("engine.filter.keytrack", filter.keytrack, 0.0, 1.0)?;
    Ok(())
}

fn validate_adsr(prefix: &'static str, adsr: &AdsrPatch) -> Result<(), PatchValidationError> {
    check_positive(format_field(prefix, "attack_ms"), adsr.attack_ms)?;
    check_positive(format_field(prefix, "decay_ms"), adsr.decay_ms)?;
    check_range(format_field(prefix, "sustain"), adsr.sustain, 0.0, 1.0)?;
    check_positive(format_field(prefix, "release_ms"), adsr.release_ms)?;
    Ok(())
}

fn validate_filter_env(filter_env: &FilterEnvPatch) -> Result<(), PatchValidationError> {
    validate_adsr("engine.filter_env", &filter_env.adsr)?;
    check_range("engine.filter_env.depth", filter_env.depth, 0.0, 1.0)?;
    Ok(())
}

fn validate_voice(voice: &VoicePatch) -> Result<(), PatchValidationError> {
    check_range("engine.voice.stereo_width", voice.stereo_width, 0.0, 1.0)?;
    check_range(
        "engine.voice.detune_spread_cents",
        voice.detune_spread_cents,
        0.0,
        50.0,
    )?;
    if let Some(value) = voice.velocity_to_level {
        check_range("engine.voice.velocity_to_level", value, 0.0, 1.0)?;
    }
    if let Some(value) = voice.velocity_to_filter {
        check_range("engine.voice.velocity_to_filter", value, 0.0, 1.0)?;
    }
    Ok(())
}

fn validate_final_stage(final_stage: &FinalStagePatch) -> Result<(), PatchValidationError> {
    check_range(
        "engine.final_stage.body_drive",
        final_stage.body_drive,
        0.0,
        1.0,
    )?;
    check_range(
        "engine.final_stage.asymmetry",
        final_stage.asymmetry,
        0.0,
        1.0,
    )?;
    check_range(
        "engine.final_stage.low_mid_emphasis",
        final_stage.low_mid_emphasis,
        0.0,
        1.0,
    )?;
    check_range(
        "engine.final_stage.output_trim_db",
        final_stage.output_trim_db,
        -24.0,
        12.0,
    )?;
    Ok(())
}

fn validate_fx(fx: &FxPatch) -> Result<(), PatchValidationError> {
    check_range("engine.fx.chorus.mix", fx.chorus.mix, 0.0, 1.0)?;
    check_range("engine.fx.chorus.depth", fx.chorus.depth, 0.0, 1.0)?;
    check_positive("engine.fx.chorus.rate_hz", fx.chorus.rate_hz)?;
    check_range("engine.fx.reverb.mix", fx.reverb.mix, 0.0, 1.0)?;
    check_range("engine.fx.reverb.size", fx.reverb.size, 0.0, 1.0)?;
    check_range("engine.fx.reverb.damping", fx.reverb.damping, 0.0, 1.0)?;
    Ok(())
}

fn validate_macro_defaults(defaults: &MacroDefaults) -> Result<(), PatchValidationError> {
    for macro_id in MacroId::ALL {
        check_range(macro_id.key(), defaults.get(macro_id), 0.0, 1.0)?;
    }
    Ok(())
}

fn validate_macro_response_set(
    response_set: &MacroResponseSet,
) -> Result<(), PatchValidationError> {
    for macro_id in MacroId::ALL {
        let spec = response_set.spec(macro_id);
        check_range("macro_response.sensitivity", spec.sensitivity, 0.0, 2.0)?;
        check_range("macro_response.softness", spec.softness, 0.0, 1.0)?;
    }
    Ok(())
}

fn validate_identity_bias(identity_bias: &IdentityBiasProfile) -> Result<(), PatchValidationError> {
    check_range(
        "identity_bias.horizont_bias",
        identity_bias.horizont_bias,
        -1.0,
        1.0,
    )?;
    check_range("identity_bias.pec_bias", identity_bias.pec_bias, -1.0, 1.0)?;
    check_range(
        "identity_bias.baklja_bias",
        identity_bias.baklja_bias,
        -1.0,
        1.0,
    )?;
    check_range(
        "identity_bias.gravitacija_pressure_bias",
        identity_bias.gravitacija_pressure_bias,
        -1.0,
        1.0,
    )?;
    check_range(
        "identity_bias.rupture_threshold_bias",
        identity_bias.rupture_threshold_bias,
        -1.0,
        1.0,
    )?;
    check_range(
        "identity_bias.body_focus_bias",
        identity_bias.body_focus_bias,
        -1.0,
        1.0,
    )?;
    Ok(())
}

fn validate_performance_response(
    performance_response: &PerformanceResponseProfile,
) -> Result<(), PatchValidationError> {
    check_range(
        "performance_response.velocity_to_level",
        performance_response.velocity_to_level,
        0.0,
        1.0,
    )?;
    check_range(
        "performance_response.velocity_to_filter",
        performance_response.velocity_to_filter,
        0.0,
        1.0,
    )?;
    check_range(
        "performance_response.aftertouch_to_gravitacija",
        performance_response.aftertouch_to_gravitacija,
        0.0,
        1.0,
    )?;
    check_range(
        "performance_response.aftertouch_to_baklja",
        performance_response.aftertouch_to_baklja,
        0.0,
        1.0,
    )?;
    check_range(
        "performance_response.mod_wheel_to_bloom",
        performance_response.mod_wheel_to_bloom,
        0.0,
        1.0,
    )?;
    check_range(
        "performance_response.mod_wheel_to_swarm",
        performance_response.mod_wheel_to_swarm,
        0.0,
        1.0,
    )?;
    check_integer_range(
        "performance_response.bend_range_semitones",
        performance_response.bend_range_semitones as i32,
        0,
        24,
    )?;
    Ok(())
}

fn validate_extensions(extensions: &Option<PatchExtensions>) -> Result<(), PatchValidationError> {
    if let Some(extensions) = extensions {
        for key in extensions.keys() {
            match key.as_str() {
                "macro_remap"
                | "macro_aliases"
                | "instrument_ontology"
                | "final_stage_bypass"
                | "gravitacija_mode" => {
                    return Err(PatchValidationError::PolicyViolation(
                        "x must not redefine macro meaning or instrument ontology",
                    ));
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn check_range(
    field: &'static str,
    value: f32,
    min: f32,
    max: f32,
) -> Result<(), PatchValidationError> {
    if (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(PatchValidationError::RangeViolation {
            field,
            min,
            max,
            value,
        })
    }
}

fn check_integer_range(
    field: &'static str,
    value: i32,
    min: i32,
    max: i32,
) -> Result<(), PatchValidationError> {
    if (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(PatchValidationError::IntegerRangeViolation {
            field,
            min,
            max,
            value,
        })
    }
}

fn check_positive(field: &'static str, value: f32) -> Result<(), PatchValidationError> {
    if value > 0.0 {
        Ok(())
    } else {
        Err(PatchValidationError::PositiveRequired { field, value })
    }
}

fn format_field(prefix: &'static str, suffix: &'static str) -> &'static str {
    match (prefix, suffix) {
        ("engine.amp_env", "attack_ms") => "engine.amp_env.attack_ms",
        ("engine.amp_env", "decay_ms") => "engine.amp_env.decay_ms",
        ("engine.amp_env", "sustain") => "engine.amp_env.sustain",
        ("engine.amp_env", "release_ms") => "engine.amp_env.release_ms",
        ("engine.filter_env", "attack_ms") => "engine.filter_env.attack_ms",
        ("engine.filter_env", "decay_ms") => "engine.filter_env.decay_ms",
        ("engine.filter_env", "sustain") => "engine.filter_env.sustain",
        ("engine.filter_env", "release_ms") => "engine.filter_env.release_ms",
        _ => "unknown_field",
    }
}
