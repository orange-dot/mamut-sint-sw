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

pub(crate) fn mixed_wave(
    oscillator: &Oscillator,
    mix_levels: [f32; 4],
    pulse_width: f32,
    noise: &mut NoiseRng,
) -> f32 {
    let saw = oscillator.saw_sample() * mix_levels[0];
    let pulse = oscillator.pulse_sample(pulse_width) * mix_levels[1];
    let triangle = oscillator.triangle_sample() * mix_levels[2];
    let noise = noise.next_bipolar() * mix_levels[3] * 0.85;
    normalize_weighted_mix(saw + pulse + triangle + noise, &mix_levels)
}

pub(crate) fn mixed_wave_osc2(
    oscillator: &Oscillator,
    mix_levels: [f32; 3],
    pulse_width: f32,
) -> f32 {
    let saw = oscillator.saw_sample() * mix_levels[0];
    let pulse = oscillator.pulse_sample(pulse_width) * mix_levels[1];
    let triangle = oscillator.triangle_sample() * mix_levels[2];
    normalize_weighted_mix(saw + pulse + triangle, &mix_levels)
}

pub(crate) fn normalize_weighted_mix<const N: usize>(
    sample_sum: f32,
    mix_levels: &[f32; N],
) -> f32 {
    let normalizer = mix_levels.iter().copied().sum::<f32>().max(1.0);
    sample_sum / normalizer
}

pub(crate) fn control_smoothing_samples(sample_rate_hz: f32) -> usize {
    let smoothing_window = (sample_rate_hz * 0.006).round() as usize;
    smoothing_window.max(8).min(512)
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

    DirectParameters {
        osc1_wave_mix: [
            engine.osc1.saw_level,
            engine.osc1.pulse_level,
            engine.osc1.triangle_level,
            engine.osc1.noise_level,
        ],
        osc2_wave_mix: [
            engine.osc2.saw_level,
            engine.osc2.pulse_level,
            engine.osc2.triangle_level,
        ],
        sub_level: (engine.sub.level * (0.70 + derived.mass * 0.30)).clamp(0.0, 1.0),
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
