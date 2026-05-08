use super::*;

pub(crate) fn is_sorted_by_frame<T>(events: &[Scheduled<T>]) -> bool {
    events
        .windows(2)
        .all(|pair| pair[0].frame_offset <= pair[1].frame_offset)
}

pub(crate) fn compare_voice_reuse(left: &VoiceState, right: &VoiceState) -> std::cmp::Ordering {
    left.amp_env
        .current_level()
        .partial_cmp(&right.amp_env.current_level())
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| left.age.cmp(&right.age))
}

pub(crate) fn expressive_amount(value: f32, exponent: f32) -> f32 {
    value.clamp(0.0, 1.0).powf(exponent.max(0.01))
}

pub(crate) fn shaped_velocity_level(value: f32) -> f32 {
    value.clamp(0.0, 1.0).powf(0.78)
}

pub(crate) fn shaped_velocity_filter(value: f32) -> f32 {
    value.clamp(0.0, 1.0).powf(1.08)
}

pub(crate) fn patch_switch_mute_frames(sample_rate_hz: f32) -> usize {
    ((sample_rate_hz * 0.004).round() as usize).clamp(32, 256)
}

pub(crate) fn is_osc2_to_osc1(direction_index: f32) -> bool {
    direction_index.round() as i32 == 1
}

pub(crate) fn osc_phase_mode_index(mode: OscPhaseMode) -> f32 {
    match mode {
        OscPhaseMode::Deterministic => 0.0,
        OscPhaseMode::Fixed => 1.0,
        OscPhaseMode::FreeRun => 2.0,
    }
}

pub(crate) fn osc2_pitch_mode_index(mode: Osc2PitchMode) -> f32 {
    match mode {
        Osc2PitchMode::Semitone => 0.0,
        Osc2PitchMode::Ratio => 1.0,
    }
}

pub(crate) fn noise_color_index(color: NoiseColor) -> f32 {
    match color {
        NoiseColor::White => 0.0,
        NoiseColor::Pinkish => 1.0,
        NoiseColor::Dark => 2.0,
        NoiseColor::Bright => 3.0,
    }
}

pub(crate) fn filter_model_index(model: FilterModel) -> f32 {
    match model {
        FilterModel::Legacy => 0.0,
        FilterModel::TptClean => 1.0,
        FilterModel::MatterDriven => 2.0,
    }
}

pub(crate) fn spectral_table_index(table: SpectralTable) -> f32 {
    match table {
        SpectralTable::Sineish => 0.0,
        SpectralTable::Vocalish => 1.0,
        SpectralTable::Metallic => 2.0,
        SpectralTable::Hollow => 3.0,
        SpectralTable::Formant => 4.0,
    }
}

pub(crate) fn mod_direction_index(direction: ModDirection) -> f32 {
    match direction {
        ModDirection::Osc1ToOsc2 => 0.0,
        ModDirection::Osc2ToOsc1 => 1.0,
    }
}

pub(crate) fn cross_mix_mode_index(mode: CrossMixMode) -> f32 {
    match mode {
        CrossMixMode::Sum => 0.0,
        CrossMixMode::Multiply => 1.0,
        CrossMixMode::Fold => 2.0,
        CrossMixMode::Max => 3.0,
        CrossMixMode::Difference => 4.0,
    }
}

pub(crate) fn control_smoothing_samples(sample_rate_hz: f32) -> usize {
    let smoothing_window = (sample_rate_hz * 0.006).round() as usize;
    smoothing_window.clamp(8, 512)
}

pub(crate) fn smoothing_sample_count(sample_rate_hz: f32, milliseconds: f32) -> usize {
    let sample_count = (sample_rate_hz * milliseconds.max(0.0) * 0.001).round() as usize;
    sample_count.max(1)
}

pub(crate) fn engine_gfm_sample_rate_hz(sample_rate_hz: f32) -> u32 {
    if sample_rate_hz.is_finite() {
        sample_rate_hz.round().clamp(1.0, u32::MAX as f32) as u32
    } else {
        48_000
    }
}

pub(crate) fn gfm_engine_layer_gain(program_id: GfmProgramId) -> f32 {
    match program_id {
        GfmProgramId::HorizontPerformance => 0.14,
        GfmProgramId::PecPerformance => 0.20,
        GfmProgramId::BakljaPerformance => 0.075,
    }
}

pub(crate) fn resolve_direct_parameters(
    patch: &PatchFileV1,
    resolved_frame: ResolvedIdentityFrame,
    control: ControlState,
) -> DirectParameters {
    let engine = &patch.engine;
    let identity = resolved_frame.identity;
    let derived = resolved_frame.derived;
    let shaped = resolved_frame.shaped_macros;

    let cutoff_scale =
        0.90 + shaped.bloom * 0.62 + identity.horizont_air * 0.18 - shaped.gravitacija * 0.40;
    let cutoff_hz = (engine.filter.cutoff_hz * cutoff_scale).clamp(20.0, 20_000.0);
    let stereo_width = (engine.voice.stereo_width + identity.horizont_span * 0.18).clamp(0.0, 1.0);
    let stereo_crossfeed = (0.10 + derived.body_focus * 0.28 + identity.grav_pull * 0.10
        - derived.spatial_dispersion * 0.10)
        .clamp(0.0, 1.0);
    let noise_filter_level = engine
        .noise_filter_level
        .unwrap_or(engine.osc1.noise_level)
        .clamp(0.0, 1.0);

    DirectParameters {
        osc1_wave_mix: [
            engine.osc1.saw_level,
            engine.osc1.pulse_level,
            engine.osc1.triangle_level,
            noise_filter_level,
        ],
        osc1_fine_tune_cents: engine.osc1.fine_tune_cents.unwrap_or(0.0),
        osc1_pulse_width: engine.osc1.pulse_width,
        osc1_pwm_depth: engine.osc1.pwm_depth,
        osc1_phase_mode: osc_phase_mode_index(engine.osc1.phase_mode),
        osc1_start_phase: engine.osc1.start_phase,
        osc1_saw_bend: engine.osc1.saw_bend,
        osc1_triangle_fold: engine.osc1.triangle_fold,
        osc1_pulse_edge: engine.osc1.pulse_edge,
        osc1_bandlimit: engine.osc1.bandlimit,
        osc2_wave_mix: [
            engine.osc2.saw_level,
            engine.osc2.pulse_level,
            engine.osc2.triangle_level,
        ],
        osc2_fine_tune_cents: engine.osc2.fine_tune_cents,
        osc2_pulse_width: engine.osc2.pulse_width,
        osc2_pwm_depth: engine.osc2.pwm_depth,
        osc2_phase_mode: osc_phase_mode_index(engine.osc2.phase_mode),
        osc2_start_phase: engine.osc2.start_phase,
        osc2_level: engine.osc2.level,
        osc2_pitch_mode: osc2_pitch_mode_index(engine.osc2.pitch_mode),
        osc2_ratio: engine.osc2.ratio,
        osc2_saw_bend: engine.osc2.saw_bend,
        osc2_triangle_fold: engine.osc2.triangle_fold,
        osc2_pulse_edge: engine.osc2.pulse_edge,
        osc2_bandlimit: engine.osc2.bandlimit,
        spectral_level: engine.spectral.level,
        spectral_table: spectral_table_index(engine.spectral.table),
        spectral_position: engine.spectral.position,
        spectral_morph: engine.spectral.morph,
        spectral_ratio: engine.spectral.ratio,
        spectral_fine_tune_cents: engine.spectral.fine_tune_cents,
        additive_level: engine.additive.level,
        additive_partial_count: engine.additive.partial_count as f32,
        additive_harmonic_spread: engine.additive.harmonic_spread,
        additive_odd_even_balance: engine.additive.odd_even_balance,
        additive_inharmonicity: engine.additive.inharmonicity,
        additive_spectral_tilt: engine.additive.spectral_tilt,
        additive_random_detune_cents: engine.additive.random_detune_cents,
        source_pwm_rate_hz: engine.source_pwm_rate_hz,
        noise_color: noise_color_index(engine.noise_color),
        noise_filter_level,
        noise_body_level: engine.noise_body_level,
        analog_drift: engine.analog_drift,
        micro_jitter: engine.micro_jitter,
        fm_amount: engine.fm_amount,
        fm_direction: mod_direction_index(engine.fm_direction),
        phase_mod_amount: engine.phase_mod_amount,
        phase_mod_direction: mod_direction_index(engine.phase_mod_direction),
        ring_mod_amount: engine.ring_mod_amount,
        am_amount: engine.am_amount,
        sync_direction: mod_direction_index(engine.sync_direction),
        sync_softness: engine.sync_softness,
        cross_mix_mode: cross_mix_mode_index(engine.cross_mix_mode),
        cross_mix_amount: engine.cross_mix_amount,
        sub_level: (engine.sub.level * (0.70 + derived.mass * 0.30)).clamp(0.0, 1.0),
        sub_octave_offset: engine.sub.octave_offset as f32,
        mixer_pre_filter_drive: engine.mixer.pre_filter_drive,
        mixer_body_mix: engine.mixer.body_mix,
        osc2_interval_semitones: engine.osc2.interval_semitones as f32
            + control.pitch_bend_semitones.clamp(
                -(patch.performance_response.bend_range_semitones as f32),
                patch.performance_response.bend_range_semitones as f32,
            ),
        sync_amount: (engine.osc2.sync_amount + identity.baklja_sync_bias * 0.30).clamp(0.0, 1.0),
        crossmod_amount: (engine.osc2.crossmod_amount + identity.baklja_ready * 0.24)
            .clamp(0.0, 1.0),
        detune_spread_cents: (engine.voice.detune_spread_cents * (0.72 + shaped.swarm * 0.42))
            .clamp(0.0, 50.0),
        cutoff_hz,
        resonance: (engine.filter.resonance + identity.baklja_edge * 0.14 + shaped.ruin * 0.06
            - derived.mass * 0.04)
            .clamp(0.0, 1.0),
        filter_drive: (engine.filter.drive + identity.pec_heat * 0.18 + derived.strain * 0.10)
            .clamp(0.0, 1.0),
        filter_env_depth: (engine.filter_env.depth + identity.horizont_open * 0.12).clamp(0.0, 1.0),
        filter_tracking: (engine.filter.keytrack * (0.92 - derived.mass * 0.12)).clamp(0.0, 1.0),
        filter_model: filter_model_index(engine.filter.model),
        amp_env: AdsrTiming {
            attack_ms: engine.amp_env.attack_ms,
            decay_ms: engine.amp_env.decay_ms,
            sustain: engine.amp_env.sustain,
            release_ms: engine.amp_env.release_ms,
        }
        .clamp(),
        filter_env: AdsrTiming {
            attack_ms: engine.filter_env.adsr.attack_ms,
            decay_ms: engine.filter_env.adsr.decay_ms,
            sustain: engine.filter_env.adsr.sustain,
            release_ms: engine.filter_env.adsr.release_ms,
        }
        .clamp(),
        voice_level: (0.75
            + derived.mass * 0.20
            + patch.performance_response.velocity_to_level * 0.05)
            .clamp(0.0, 1.0),
        voice_velocity_to_level: engine
            .voice
            .velocity_to_level
            .unwrap_or(patch.performance_response.velocity_to_level),
        voice_velocity_to_filter: engine
            .voice
            .velocity_to_filter
            .unwrap_or(patch.performance_response.velocity_to_filter),
        body_drive: (engine.final_stage.body_drive + derived.body_focus * 0.25).clamp(0.0, 1.0),
        output_trim_db: engine.final_stage.output_trim_db,
        stereo_width,
        stereo_crossfeed,
        final_saturation: (engine.final_stage.body_drive
            + derived.mass * 0.14
            + derived.strain * 0.08)
            .clamp(0.0, 1.0),
        final_asymmetry: (engine.final_stage.asymmetry + identity.baklja_edge * 0.22)
            .clamp(0.0, 1.0),
        low_mid_emphasis: (engine.final_stage.low_mid_emphasis + derived.mass * 0.15)
            .clamp(0.0, 1.0),
        chorus_enabled: engine.fx.chorus.enabled,
        chorus_mix: engine.fx.chorus.mix,
        chorus_depth: engine.fx.chorus.depth,
        chorus_rate_hz: engine.fx.chorus.rate_hz,
        reverb_enabled: engine.fx.reverb.enabled,
        reverb_mix: engine.fx.reverb.mix,
        reverb_size: engine.fx.reverb.size,
        reverb_damping: engine.fx.reverb.damping,
    }
}

pub(crate) fn macro_defaults_with_override(
    defaults: &mamut_patch::MacroDefaults,
    macro_id: MacroId,
    value: f32,
) -> mamut_patch::MacroDefaults {
    let mut macros = MacroState::from_defaults(defaults);
    macros.set(macro_id, value);
    mamut_patch::MacroDefaults {
        gravitacija: macros.gravitacija,
        bloom: macros.bloom,
        heat: macros.heat,
        ruin: macros.ruin,
        swarm: macros.swarm,
    }
}
