use super::*;

pub fn dry_run(options: &DryRunOptions) -> Result<()> {
    let path = &options.patch_path;
    let patch = load_patch_from_path(path)?;
    validate_patch_v1(&patch).context("patch validation failed")?;
    let mut engine = Engine::new(EngineConfig::default(), patch)?;
    engine.set_gfm_layer_mode(gfm_layer_mode_from_seed(options.gfm_layer_seed));
    engine.set_bcs_layer_mode(bcs_layer_mode_from_scenario(options.bcs_layer_scenario));
    let mut left = vec![0.0_f32; 256];
    let mut right = vec![0.0_f32; 256];

    let note_events = [
        Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: 60,
                velocity: 0.88,
            },
        },
        Scheduled {
            frame_offset: 200,
            event: NoteEvent::NoteOff { note: 60 },
        },
    ];
    let controller_events = [
        Scheduled {
            frame_offset: 0,
            event: ControllerEvent::ModWheel { amount: 0.45 },
        },
        Scheduled {
            frame_offset: 64,
            event: ControllerEvent::ChannelAftertouch { pressure: 0.30 },
        },
        Scheduled {
            frame_offset: 96,
            event: ControllerEvent::Macro {
                id: MacroId::Gravitacija,
                value: 0.64,
            },
        },
        Scheduled {
            frame_offset: 160,
            event: ControllerEvent::Macro {
                id: MacroId::Ruin,
                value: 0.42,
            },
        },
    ];

    engine.process_block(ProcessBlock {
        frame_count: 256,
        note_events: &note_events,
        controller_events: &controller_events,
        macro_state: None,
        output: Some(StereoBlockMut::new(&mut left, &mut right)),
    });

    let snapshot = engine.snapshot();
    println!("patch: {} ({})", snapshot.patch_name, path.display());
    println!(
        "description: {}",
        snapshot.patch_description.as_deref().unwrap_or_default()
    );
    println!("active voices: {}", snapshot.active_voice_count);
    println!("held notes: {:?}", snapshot.held_notes);
    println!(
        "macros: gravitacija={:.3} bloom={:.3} heat={:.3} ruin={:.3} swarm={:.3}",
        snapshot.effective_macros.gravitacija,
        snapshot.effective_macros.bloom,
        snapshot.effective_macros.heat,
        snapshot.effective_macros.ruin,
        snapshot.effective_macros.swarm
    );
    println!(
        "identity: horizont_open={:.3} pec_mass={:.3} baklja_ready={:.3} grav_pull={:.3}",
        snapshot.identity.horizont_open,
        snapshot.identity.pec_mass,
        snapshot.identity.baklja_ready,
        snapshot.identity.grav_pull
    );
    println!(
        "derived: mass={:.3} strain={:.3} headroom={:.3} rupture_threshold={:.3}",
        snapshot.derived.mass,
        snapshot.derived.strain,
        snapshot.derived.headroom,
        snapshot.derived.rupture_threshold
    );
    println!(
        "direct: cutoff_hz={:.2} sync={:.3} crossmod={:.3} body_drive={:.3} peak={:.3}",
        snapshot.direct.cutoff_hz,
        snapshot.direct.sync_amount,
        snapshot.direct.crossmod_amount,
        snapshot.direct.body_drive,
        snapshot.peak_output
    );
    println!("{}", gfm_layer_status_line(&snapshot));
    println!("{}", bcs_layer_status_line(&snapshot));

    Ok(())
}

pub fn gfm_layer_mode_from_seed(seed: Option<u64>) -> GfmLayerMode {
    match seed {
        Some(seed) => GfmLayerMode::Enabled { seed },
        None => GfmLayerMode::Disabled,
    }
}

pub fn gfm_layer_seed_from_ui_text(enabled: bool, seed_text: &str) -> Result<Option<u64>> {
    if enabled {
        parse_gfm_layer_seed(seed_text).map(Some)
    } else {
        Ok(None)
    }
}

pub fn gfm_layer_status_line(snapshot: &EngineSnapshot) -> String {
    gfm_layer_status_line_from_layer(snapshot.gfm_layer)
}

pub fn bcs_layer_mode_from_scenario(scenario: Option<BcsScenario>) -> BcsLayerMode {
    match scenario {
        Some(scenario) => BcsLayerMode::Enabled { scenario },
        None => BcsLayerMode::Disabled,
    }
}

pub fn bcs_layer_status_line(snapshot: &EngineSnapshot) -> String {
    bcs_layer_status_line_from_layer(snapshot.bcs_layer)
}

pub fn bcs_layer_status_line_from_layer(layer: BcsLayerSnapshot) -> String {
    match layer.mode {
        BcsLayerMode::Disabled => format!(
            "bcs: mode=disabled playable={} knob={:.3} gain={:.3} effective_gain={:.3}",
            on_off_bool(layer.enabled),
            layer.amount,
            layer.gain,
            layer.effective_gain
        ),
        BcsLayerMode::Enabled { scenario } => {
            let active = layer
                .active_scenario
                .map(format_bcs_scenario)
                .unwrap_or("-");
            let sample_rate = layer
                .sample_rate_hz
                .map(|sample_rate| sample_rate.to_string())
                .unwrap_or_else(|| "-".to_string());
            let pitch = layer
                .pitch_note
                .map(|note| {
                    format!(
                        "{}@{:.2}Hz",
                        note,
                        layer.pitch_frequency_hz.unwrap_or_default()
                    )
                })
                .unwrap_or_else(|| "-".to_string());
            format!(
                "bcs: mode=enabled scenario={} active={} playable={} knob={:.3} gain={:.3} effective_gain={:.3} pitch={} sample_rate={} max_state={:.6} unsafe_events={} unsafe={}",
                format_bcs_scenario(scenario),
                active,
                on_off_bool(layer.enabled),
                layer.amount,
                layer.gain,
                layer.effective_gain,
                pitch,
                sample_rate,
                layer.max_state_abs,
                layer.unsafe_events,
                layer.unsafe_state
            )
        }
    }
}

pub fn gfm_layer_status_line_from_layer(layer: mamut_engine::GfmLayerSnapshot) -> String {
    match layer.mode {
        GfmLayerMode::Disabled => "gfm: mode=disabled".to_string(),
        GfmLayerMode::Enabled { seed } => {
            let ruptures = layer
                .diagnostics
                .map(|diagnostics| diagnostics.max_rupture_count.to_string())
                .unwrap_or_else(|| "-".to_string());
            format!(
                "gfm: mode=enabled seed={} selected={} active={} scores=({:.4},{:.4},{:.4}) ruptures={}",
                format_gfm_seed(seed),
                format_optional_program_id(layer.selection.program_id),
                format_optional_program_id(layer.active_program_id),
                layer.selection.horizont_score,
                layer.selection.pec_score,
                layer.selection.baklja_score,
                ruptures
            )
        }
    }
}

pub fn format_gfm_seed(seed: u64) -> String {
    format!("0x{seed:X}")
}

pub fn format_optional_program_id<T: std::fmt::Debug>(program_id: Option<T>) -> String {
    program_id
        .map(|program_id| format!("{program_id:?}"))
        .unwrap_or_else(|| "none".to_string())
}

pub fn on_off_bool(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}

pub fn play(options: &PlayOptions) -> Result<()> {
    let mut session = RuntimeSession::new(options)?;
    if !options.headless && io::stdin().is_terminal() {
        session.command_loop()
    } else {
        session.print_startup_summary();
        println!("stdin is not a terminal; running without interactive controls");
        session.block_forever()
    }
}
