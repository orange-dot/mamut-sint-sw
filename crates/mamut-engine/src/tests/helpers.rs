use super::*;

#[derive(Clone, Copy)]
pub(super) struct GfmDryRunStats {
    pub(super) finite: bool,
    pub(super) peak_abs: f32,
    pub(super) max_rupture_count: usize,
    pub(super) health: GfmHealthHistogram,
    pub(super) probe_position: (usize, usize),
    pub(super) frames: usize,
    pub(super) final_frame_index: usize,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct EngineLayerRenderStats {
    pub(super) finite: bool,
    pub(super) rms: f32,
    pub(super) peak_abs: f32,
    pub(super) gfm_selection: GfmVoiceProgramSelection,
    pub(super) gfm_program_id: Option<GfmProgramId>,
    pub(super) gfm_diagnostics: Option<GfmDiagnostics>,
}

pub(super) fn fixture_engine() -> Engine {
    let patch = load_patch_toml(MOLTEN_HORIZON).expect("fixture must parse");
    Engine::new(EngineConfig::default(), patch).expect("fixture must validate")
}

#[derive(Debug, Clone, Copy)]
pub(super) struct OutputStats {
    pub(super) left_mean: f64,
    pub(super) right_mean: f64,
    pub(super) peak_abs: f32,
}

pub(super) fn render_post_warmup_stats(patch_source: &str, note: Option<u8>) -> OutputStats {
    const SAMPLE_RATE_HZ: usize = 48_000;
    const BLOCK_FRAMES: usize = 240;
    const TOTAL_FRAMES: usize = SAMPLE_RATE_HZ * 3;
    const WARMUP_FRAMES: usize = SAMPLE_RATE_HZ;

    let patch = load_patch_toml(patch_source).expect("factory patch must parse");
    let mut engine = Engine::new(
        EngineConfig {
            sample_rate_hz: SAMPLE_RATE_HZ as f32,
            max_block_frames: BLOCK_FRAMES,
            voice_count: 6,
        },
        patch,
    )
    .expect("engine must validate");
    let note_on = [Scheduled {
        frame_offset: 0,
        event: NoteEvent::NoteOn {
            note: note.unwrap_or(60),
            velocity: 0.9,
        },
    }];
    let mut left = [0.0_f32; BLOCK_FRAMES];
    let mut right = [0.0_f32; BLOCK_FRAMES];
    let mut frame_cursor = 0_usize;
    let mut left_sum = 0.0_f64;
    let mut right_sum = 0.0_f64;
    let mut peak_abs = 0.0_f32;
    let mut measured_frames = 0_usize;

    while frame_cursor < TOTAL_FRAMES {
        let frame_count = (TOTAL_FRAMES - frame_cursor).min(BLOCK_FRAMES);
        let note_events = if frame_cursor == 0 && note.is_some() {
            &note_on[..]
        } else {
            &[]
        };
        engine.process_block(ProcessBlock {
            frame_count,
            note_events,
            controller_events: &[],
            macro_state: None,
            output: Some(StereoBlockMut::new(&mut left, &mut right)),
        });

        for frame in 0..frame_count {
            if frame_cursor + frame >= WARMUP_FRAMES {
                let left_sample = left[frame];
                let right_sample = right[frame];
                assert!(left_sample.is_finite());
                assert!(right_sample.is_finite());
                left_sum += left_sample as f64;
                right_sum += right_sample as f64;
                peak_abs = peak_abs.max(left_sample.abs().max(right_sample.abs()));
                measured_frames += 1;
            }
        }

        frame_cursor += frame_count;
    }

    OutputStats {
        left_mean: left_sum / measured_frames as f64,
        right_mean: right_sum / measured_frames as f64,
        peak_abs,
    }
}
pub(super) fn render_engine_layer_signature(
    patch: PatchFileV1,
    layer_seed: Option<u64>,
    frames: usize,
) -> (u64, EngineLayerRenderStats) {
    render_engine_layer_signature_at_rate(patch, layer_seed, ENGINE_LAYER_TEST_RATE_HZ, frames)
}

pub(super) fn render_engine_layer_signature_at_rate(
    patch: PatchFileV1,
    layer_seed: Option<u64>,
    sample_rate_hz: u32,
    frames: usize,
) -> (u64, EngineLayerRenderStats) {
    let note_events = engine_layer_note_on_events(select_gfm_program_for_patch(&patch).program_id);
    let mut engine = Engine::new(
        EngineConfig {
            sample_rate_hz: sample_rate_hz as f32,
            max_block_frames: ENGINE_LAYER_BLOCK_FRAMES,
            voice_count: 6,
        },
        patch,
    )
    .expect("engine must validate");

    if let Some(seed) = layer_seed {
        engine.set_gfm_layer_mode(GfmLayerMode::Enabled { seed });
    }

    render_engine_layer_signature_with_notes(&mut engine, frames, note_events)
}

pub(super) fn render_engine_layer_signature_for_engine(
    engine: &mut Engine,
    frames: usize,
) -> (u64, EngineLayerRenderStats) {
    let note_events =
        engine_layer_note_on_events(select_gfm_program_for_patch(&engine.patch).program_id);
    render_engine_layer_signature_with_notes(engine, frames, note_events)
}

pub(super) fn render_engine_layer_signature_with_notes(
    engine: &mut Engine,
    frames: usize,
    note_events: [ScheduledNoteEvent; 3],
) -> (u64, EngineLayerRenderStats) {
    render_engine_layer_signature_with_notes_and_controllers(engine, frames, note_events, &[])
}

pub(super) fn bcs_layer_playable_controller_events(amount: f32) -> [ScheduledControllerEvent; 2] {
    [
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::BcsLayerEnabled { enabled: true },
        },
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::BcsLayerAmount { amount },
        },
    ]
}

pub(super) fn render_engine_layer_mono_samples(
    engine: &mut Engine,
    frames: usize,
    note_events: &[ScheduledNoteEvent],
    initial_controller_events: &[ScheduledControllerEvent],
) -> (Vec<f32>, EngineLayerRenderStats) {
    let mut rendered = 0;
    let mut finite = true;
    let mut peak_abs = 0.0_f32;
    let mut sum_squares = 0.0_f32;
    let mut mono = Vec::with_capacity(frames);
    let mut left = [0.0_f32; ENGINE_LAYER_BLOCK_FRAMES];
    let mut right = [0.0_f32; ENGINE_LAYER_BLOCK_FRAMES];

    while rendered < frames {
        let frame_count = (frames - rendered).min(ENGINE_LAYER_BLOCK_FRAMES);

        let (block_note_events, block_controller_events) = if rendered == 0 {
            (note_events, initial_controller_events)
        } else {
            (&[][..], &[][..])
        };
        engine.process_block(ProcessBlock {
            frame_count,
            note_events: block_note_events,
            controller_events: block_controller_events,
            macro_state: None,
            output: Some(StereoBlockMut::new(
                &mut left[..frame_count],
                &mut right[..frame_count],
            )),
        });

        for index in 0..frame_count {
            let left_sample = left[index];
            let right_sample = right[index];
            finite &= left_sample.is_finite() && right_sample.is_finite();
            peak_abs = peak_abs.max(left_sample.abs().max(right_sample.abs()));
            sum_squares += left_sample * left_sample + right_sample * right_sample;
            mono.push((left_sample + right_sample) * 0.5);
        }

        rendered += frame_count;
    }

    let rms = (sum_squares / (frames.max(1) * 2) as f32).sqrt();
    let gfm_layer = engine.snapshot().gfm_layer;
    (
        mono,
        EngineLayerRenderStats {
            finite,
            rms,
            peak_abs,
            gfm_selection: gfm_layer.selection,
            gfm_program_id: gfm_layer.active_program_id,
            gfm_diagnostics: gfm_layer.diagnostics,
        },
    )
}

pub(super) fn rms_window(samples: &[f32]) -> f32 {
    let sum_squares = samples.iter().map(|sample| sample * sample).sum::<f32>();
    (sum_squares / samples.len().max(1) as f32).sqrt()
}

pub(super) fn rms_delta_window(left: &[f32], right: &[f32]) -> f32 {
    let sum_squares = left
        .iter()
        .zip(right)
        .map(|(left, right)| {
            let delta = left - right;
            delta * delta
        })
        .sum::<f32>();
    (sum_squares / left.len().min(right.len()).max(1) as f32).sqrt()
}

pub(super) fn amplitude_ratio_db(numerator: f32, denominator: f32) -> f32 {
    20.0 * (numerator.max(f32::MIN_POSITIVE) / denominator.max(f32::MIN_POSITIVE)).log10()
}

pub(super) fn render_engine_layer_signature_with_notes_and_controllers(
    engine: &mut Engine,
    frames: usize,
    note_events: [ScheduledNoteEvent; 3],
    initial_controller_events: &[ScheduledControllerEvent],
) -> (u64, EngineLayerRenderStats) {
    let mut rendered = 0;
    let mut signature = 0xcbf2_9ce4_8422_2325_u64;
    let mut finite = true;
    let mut peak_abs = 0.0_f32;
    let mut sum_squares = 0.0_f32;
    let mut left = [0.0_f32; ENGINE_LAYER_BLOCK_FRAMES];
    let mut right = [0.0_f32; ENGINE_LAYER_BLOCK_FRAMES];

    while rendered < frames {
        let frame_count = (frames - rendered).min(ENGINE_LAYER_BLOCK_FRAMES);

        if rendered == 0 {
            engine.process_block(ProcessBlock {
                frame_count,
                note_events: &note_events,
                controller_events: initial_controller_events,
                macro_state: None,
                output: Some(StereoBlockMut::new(
                    &mut left[..frame_count],
                    &mut right[..frame_count],
                )),
            });
        } else {
            engine.process_block(ProcessBlock {
                frame_count,
                note_events: &[],
                controller_events: &[],
                macro_state: None,
                output: Some(StereoBlockMut::new(
                    &mut left[..frame_count],
                    &mut right[..frame_count],
                )),
            });
        }

        for index in 0..frame_count {
            let left_sample = left[index];
            let right_sample = right[index];
            finite &= left_sample.is_finite() && right_sample.is_finite();
            peak_abs = peak_abs.max(left_sample.abs().max(right_sample.abs()));
            sum_squares += left_sample * left_sample + right_sample * right_sample;
            for pcm in [sample_to_pcm16(left_sample), sample_to_pcm16(right_sample)] {
                signature ^= pcm as u16 as u64;
                signature = signature.wrapping_mul(0x100_0000_01b3);
            }
        }

        rendered += frame_count;
    }

    let rms = (sum_squares / (frames.max(1) * 2) as f32).sqrt();
    let gfm_layer = engine.snapshot().gfm_layer;
    (
        signature,
        EngineLayerRenderStats {
            finite,
            rms,
            peak_abs,
            gfm_selection: gfm_layer.selection,
            gfm_program_id: gfm_layer.active_program_id,
            gfm_diagnostics: gfm_layer.diagnostics,
        },
    )
}

pub(super) fn engine_layer_note_on_events(
    program_id: Option<GfmProgramId>,
) -> [ScheduledNoteEvent; 3] {
    let [low, middle, high] = engine_layer_notes(program_id);
    [
        Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: low,
                velocity: 0.78,
            },
        },
        Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: middle,
                velocity: 0.70,
            },
        },
        Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: high,
                velocity: 0.64,
            },
        },
    ]
}

pub(super) fn engine_layer_notes(program_id: Option<GfmProgramId>) -> [u8; 3] {
    match program_id {
        Some(GfmProgramId::PecPerformance) => [60, 67, 72],
        Some(GfmProgramId::HorizontPerformance | GfmProgramId::BakljaPerformance) | None => {
            [48, 55, 60]
        }
    }
}

pub(super) fn render_gfm_adapter_signature(program_id: GfmProgramId) -> (u64, GfmDryRunStats) {
    let mut voice = GfmFieldVoice::new(program_id, GFM_PERFORMANCE_BASELINE_SEED, GFM_TEST_RATE_HZ);
    let frames = voice.frames();
    let probe_position = voice.probe_position();
    let mut signature = 0xcbf2_9ce4_8422_2325_u64;
    let mut finite = true;
    let mut peak_abs = 0.0_f32;

    for _ in 0..frames {
        let sample = voice.next_sample();
        finite &= sample.is_finite();
        peak_abs = peak_abs.max(sample.abs());
        let pcm = sample_to_pcm16(sample);
        signature ^= pcm as u16 as u64;
        signature = signature.wrapping_mul(0x100_0000_01b3);
    }

    let diagnostics = voice.diagnostics();
    (
        signature,
        GfmDryRunStats {
            finite,
            peak_abs,
            max_rupture_count: diagnostics.max_rupture_count,
            health: diagnostics.health,
            probe_position,
            frames,
            final_frame_index: voice.frame_index(),
        },
    )
}

pub(super) fn render_gfm_adapter_signature_with_live_pressure(
    program_id: GfmProgramId,
    pressure: f32,
    frames: usize,
) -> (u64, GfmDryRunStats) {
    let mut voice = GfmFieldVoice::new(program_id, GFM_PERFORMANCE_BASELINE_SEED, GFM_TEST_RATE_HZ);
    let probe_position = voice.probe_position();
    let mut signature = 0xcbf2_9ce4_8422_2325_u64;
    let mut finite = true;
    let mut peak_abs = 0.0_f32;

    for _ in 0..frames {
        let sample = voice.next_sample_with_live_pressure(pressure);
        finite &= sample.is_finite();
        peak_abs = peak_abs.max(sample.abs());
        let pcm = sample_to_pcm16(sample);
        signature ^= pcm as u16 as u64;
        signature = signature.wrapping_mul(0x100_0000_01b3);
    }

    let diagnostics = voice.diagnostics();
    (
        signature,
        GfmDryRunStats {
            finite,
            peak_abs,
            max_rupture_count: diagnostics.max_rupture_count,
            health: diagnostics.health,
            probe_position,
            frames,
            final_frame_index: voice.frame_index(),
        },
    )
}

pub(super) fn render_gfm_block_signature(program_id: GfmProgramId) -> (u64, GfmDryRunStats) {
    let voice = GfmFieldVoice::new(program_id, GFM_PERFORMANCE_BASELINE_SEED, GFM_TEST_RATE_HZ);
    render_gfm_voice_block_signature(voice)
}

pub(super) fn render_gfm_voice_block_signature(mut voice: GfmFieldVoice) -> (u64, GfmDryRunStats) {
    let frames = voice.frames();
    let probe_position = voice.probe_position();
    let mut signature = 0xcbf2_9ce4_8422_2325_u64;
    let mut finite = true;
    let mut peak_abs = 0.0_f32;
    let mut rendered = 0;
    let mut block = [0.0_f32; 512];

    while rendered < frames {
        let frame_count = (frames - rendered).min(block.len());
        voice.render_mono_block(&mut block[..frame_count]);
        for sample in &block[..frame_count] {
            finite &= sample.is_finite();
            peak_abs = peak_abs.max(sample.abs());
            let pcm = sample_to_pcm16(*sample);
            signature ^= pcm as u16 as u64;
            signature = signature.wrapping_mul(0x100_0000_01b3);
        }
        rendered += frame_count;
    }

    let diagnostics = voice.diagnostics();
    (
        signature,
        GfmDryRunStats {
            finite,
            peak_abs,
            max_rupture_count: diagnostics.max_rupture_count,
            health: diagnostics.health,
            probe_position,
            frames,
            final_frame_index: voice.frame_index(),
        },
    )
}

pub(super) fn render_gfm_chunked_signature(
    program_id: GfmProgramId,
    chunk_size: usize,
) -> (u64, GfmDryRunStats) {
    let mut voice = GfmFieldVoice::new(program_id, GFM_PERFORMANCE_BASELINE_SEED, GFM_TEST_RATE_HZ);
    let frames = voice.frames();
    let probe_position = voice.probe_position();
    let mut signature = 0xcbf2_9ce4_8422_2325_u64;
    let mut finite = true;
    let mut peak_abs = 0.0_f32;
    let mut rendered = 0;
    let mut block = [0.0_f32; 251];
    let chunk_size = chunk_size.clamp(1, block.len());

    while rendered < frames {
        let frame_count = (frames - rendered).min(chunk_size);
        voice.render_mono_block(&mut block[..frame_count]);
        for sample in &block[..frame_count] {
            finite &= sample.is_finite();
            peak_abs = peak_abs.max(sample.abs());
            let pcm = sample_to_pcm16(*sample);
            signature ^= pcm as u16 as u64;
            signature = signature.wrapping_mul(0x100_0000_01b3);
        }
        rendered += frame_count;
    }

    let diagnostics = voice.diagnostics();
    (
        signature,
        GfmDryRunStats {
            finite,
            peak_abs,
            max_rupture_count: diagnostics.max_rupture_count,
            health: diagnostics.health,
            probe_position,
            frames,
            final_frame_index: voice.frame_index(),
        },
    )
}

pub(super) fn render_gfm_selected_patch_signature(
    program_id: GfmProgramId,
) -> (u64, GfmDryRunStats) {
    render_gfm_block_signature(program_id)
}

pub(super) fn gfm_patch_voice_test_config() -> GfmPatchVoiceConfig {
    GfmPatchVoiceConfig::new(GFM_PERFORMANCE_BASELINE_SEED, GFM_TEST_RATE_HZ)
}

pub(super) fn low_score_gfm_patch() -> PatchFileV1 {
    let mut patch = load_patch_toml(MOLTEN_HORIZON).expect("fixture must parse");
    patch.meta.patch_name = "GFM Low Score Disabled".to_string();
    patch.macros.gravitacija = 0.0;
    patch.macros.bloom = 0.0;
    patch.macros.heat = 0.0;
    patch.macros.ruin = 0.0;
    patch.macros.swarm = 0.0;
    patch.identity_bias.horizont_bias = -1.0;
    patch.identity_bias.pec_bias = -1.0;
    patch.identity_bias.baklja_bias = -1.0;
    patch.identity_bias.gravitacija_pressure_bias = -1.0;
    patch.identity_bias.rupture_threshold_bias = 1.0;
    patch.identity_bias.body_focus_bias = -1.0;
    patch
}

pub(super) fn render_direct_field_signature(program_id: GfmProgramId) -> (u64, GfmDryRunStats) {
    let program = GfmPerformanceProgram::new(program_id, GFM_TEST_RATE_HZ as f32);
    let gesture = GfmPerformanceGesture::v0_4();
    let mut lattice = GfmLattice16::new(GFM_PERFORMANCE_BASELINE_SEED, program.params());
    let frames = gesture.frames(GFM_TEST_RATE_HZ);
    let probe_position = lattice.probe_position();
    let mut signature = 0xcbf2_9ce4_8422_2325_u64;
    let mut finite = true;
    let mut peak_abs = 0.0_f32;

    for frame in 0..frames {
        let sample = lattice
            .next_sample_with_excitation(gesture.excitation_at_frame(frame, GFM_TEST_RATE_HZ));
        finite &= sample.is_finite();
        peak_abs = peak_abs.max(sample.abs());
        let pcm = sample_to_pcm16(sample);
        signature ^= pcm as u16 as u64;
        signature = signature.wrapping_mul(0x100_0000_01b3);
    }

    let diagnostics = lattice.diagnostics();
    (
        signature,
        GfmDryRunStats {
            finite,
            peak_abs,
            max_rupture_count: diagnostics.max_rupture_count,
            health: diagnostics.health,
            probe_position,
            frames,
            final_frame_index: frames,
        },
    )
}
