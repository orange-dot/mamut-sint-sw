use super::*;

#[allow(clippy::too_many_arguments)]
pub fn draw_tui(
    frame: &mut Frame<'_>,
    state: &TuiState,
    snapshot: Option<&EngineSnapshot>,
    transport: &TransportMetricsSnapshot,
    input: &InputMetricsSnapshot,
    recording: &RecordingMetricsSnapshot,
    live_set: &[FactoryPatchEntry],
    factory: &[FactoryPatchEntry],
) {
    let area = frame.area();
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(area);
    draw_tabs(frame, state, vertical[0]);
    match state.screen {
        TuiScreen::Live => draw_live(
            frame,
            state,
            snapshot,
            transport,
            input,
            recording,
            live_set,
            factory,
            vertical[1],
        ),
        TuiScreen::SoundLab => draw_sound_lab(frame, state, snapshot, recording, vertical[1]),
        TuiScreen::Pc4 => draw_pc4(frame, snapshot, input, vertical[1]),
        TuiScreen::Debug => draw_debug(frame, snapshot, transport, input, recording, vertical[1]),
    }
    draw_footer(frame, state, vertical[2]);
    if state.show_help {
        draw_help(frame, centered_rect(72, 70, area));
    }
}

pub(super) fn draw_tabs(frame: &mut Frame<'_>, state: &TuiState, area: Rect) {
    let titles = TuiScreen::ALL
        .iter()
        .map(|screen| Line::from(screen.title()))
        .collect::<Vec<_>>();
    let tabs = Tabs::new(titles)
        .select(state.screen.index())
        .block(
            Block::default()
                .title("Mamut EPM1 TUI")
                .borders(Borders::ALL),
        )
        .style(Style::default().fg(Color::Gray))
        .highlight_style(
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, area);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_live(
    frame: &mut Frame<'_>,
    state: &TuiState,
    snapshot: Option<&EngineSnapshot>,
    transport: &TransportMetricsSnapshot,
    input: &InputMetricsSnapshot,
    recording: &RecordingMetricsSnapshot,
    live_set: &[FactoryPatchEntry],
    factory: &[FactoryPatchEntry],
    area: Rect,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Min(8),
        ])
        .split(area);
    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(36),
            Constraint::Percentage(32),
            Constraint::Percentage(32),
        ])
        .split(rows[0]);
    draw_patch_status(frame, snapshot, top[0]);
    draw_recording(frame, state, snapshot, recording, top[1]);
    draw_transport(frame, transport, input, top[2]);

    let middle = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(48), Constraint::Percentage(52)])
        .split(rows[1]);
    draw_live_set(frame, snapshot, live_set, middle[0]);
    draw_macros(frame, snapshot, middle[1]);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[2]);
    draw_layers(frame, snapshot, bottom[0]);
    draw_factory(frame, snapshot, factory, bottom[1]);
}

pub(super) fn draw_patch_status(
    frame: &mut Frame<'_>,
    snapshot: Option<&EngineSnapshot>,
    area: Rect,
) {
    let lines = if let Some(snapshot) = snapshot {
        vec![
            Line::from(vec![
                Span::styled("Patch ", label()),
                Span::raw(snapshot.patch_name.clone()),
            ]),
            Line::from(format!(
                "voices {}  held {:?}  sustain {}",
                snapshot.active_voice_count,
                snapshot.held_notes,
                on_off_bool(snapshot.sustain_down)
            )),
            Line::from(format!(
                "peak {:.3}  clip {}",
                snapshot.peak_output,
                on_off_bool(snapshot.clip_detected)
            )),
        ]
    } else {
        vec![Line::from("waiting for engine snapshot")]
    };
    frame.render_widget(panel("Current Patch", lines), area);
}

pub(super) fn draw_recording(
    frame: &mut Frame<'_>,
    state: &TuiState,
    snapshot: Option<&EngineSnapshot>,
    recording: &RecordingMetricsSnapshot,
    area: Rect,
) {
    let seconds = recording.frames_written as f32 / 48_000.0;
    let preview = snapshot
        .map(|snapshot| format!("{}-{}s", snapshot.patch_name, state.record_seconds))
        .unwrap_or_else(|| "pending".to_string());
    let lines = vec![
        Line::from(vec![
            Span::styled("State ", label()),
            Span::raw(recording.state.label().to_ascii_uppercase()),
        ]),
        Line::from(format!(
            "r toggles  target {}s  tag {}",
            state.record_seconds, state.take_tag
        )),
        Line::from(format!(
            "written {:.1}s  dropped {}",
            seconds, recording.frames_dropped
        )),
        Line::from(format!("next {}", preview)),
    ];
    frame.render_widget(panel("Take Recorder", lines), area);
}

pub(super) fn draw_transport(
    frame: &mut Frame<'_>,
    transport: &TransportMetricsSnapshot,
    input: &InputMetricsSnapshot,
    area: Rect,
) {
    let lines = vec![
        Line::from(format!(
            "queue {} / target {}",
            transport.queued_frames, transport.queue_target_frames
        )),
        Line::from(format!(
            "underrun {} batches / {} frames",
            transport.underrun_batches, transport.underrun_frames
        )),
        Line::from(format!(
            "overflow {} batches / {} frames",
            transport.overflow_batches, transport.overflow_frames
        )),
        Line::from(format!(
            "midi {} dropped {} trace_drop {}",
            input.midi_messages, input.midi_messages_dropped, input.trace_records_dropped
        )),
        Line::from(
            input
                .last_control
                .as_ref()
                .map(|event| format!("last {}", last_control_summary(event)))
                .unwrap_or_else(|| "last none".to_string()),
        ),
    ];
    frame.render_widget(panel("Runtime Health", lines), area);
}

pub(super) fn draw_live_set(
    frame: &mut Frame<'_>,
    snapshot: Option<&EngineSnapshot>,
    live_set: &[FactoryPatchEntry],
    area: Rect,
) {
    let rows = live_set
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let active = snapshot.is_some_and(|snapshot| snapshot.patch_name == entry.patch_name);
            Row::new(vec![
                Cell::from(format!("{index:02}")),
                Cell::from(entry.patch_name.clone()),
                Cell::from(entry.stem.clone()),
            ])
            .style(if active {
                active_style()
            } else {
                Style::default()
            })
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Length(4),
                Constraint::Percentage(45),
                Constraint::Percentage(45),
            ],
        )
        .header(Row::new(["slot", "patch", "stem"]).style(label()))
        .block(Block::default().title("Live Set").borders(Borders::ALL)),
        area,
    );
}

pub(super) fn draw_macros(frame: &mut Frame<'_>, snapshot: Option<&EngineSnapshot>, area: Rect) {
    let Some(snapshot) = snapshot else {
        frame.render_widget(
            panel("Macros", vec![Line::from("waiting for snapshot")]),
            area,
        );
        return;
    };
    let rows = [
        (
            "Gravitacija",
            snapshot.live_macros.gravitacija,
            snapshot.effective_macros.gravitacija,
        ),
        (
            "Bloom",
            snapshot.live_macros.bloom,
            snapshot.effective_macros.bloom,
        ),
        (
            "Heat",
            snapshot.live_macros.heat,
            snapshot.effective_macros.heat,
        ),
        (
            "Ruin",
            snapshot.live_macros.ruin,
            snapshot.effective_macros.ruin,
        ),
        (
            "Swarm",
            snapshot.live_macros.swarm,
            snapshot.effective_macros.swarm,
        ),
    ];
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(1); rows.len()])
        .split(area);
    frame.render_widget(Block::default().title("Macros").borders(Borders::ALL), area);
    for (index, (name, live, effective)) in rows.iter().enumerate() {
        let gauge = Gauge::default()
            .label(format!("{name} {live:.2} / eff {effective:.2}"))
            .ratio((*effective).clamp(0.0, 1.0) as f64)
            .gauge_style(Style::default().fg(Color::LightCyan));
        frame.render_widget(gauge, inner_line(chunks[index], area));
    }
}

pub(super) fn draw_layers(frame: &mut Frame<'_>, snapshot: Option<&EngineSnapshot>, area: Rect) {
    let lines = if let Some(snapshot) = snapshot {
        let gfm_mode = match snapshot.gfm_layer.mode {
            GfmLayerMode::Enabled { .. } => "enabled",
            GfmLayerMode::Disabled => "disabled",
        };
        let bcs_mode = match snapshot.bcs_layer.mode {
            BcsLayerMode::Enabled { scenario } => format!("{scenario:?}"),
            BcsLayerMode::Disabled => "disabled".to_string(),
        };
        vec![
            Line::from(format!(
                "GFM {gfm_mode} amount {:.2} pressure {:.2}",
                snapshot.gfm_layer.effective_amount, snapshot.gfm_layer.pressure
            )),
            Line::from(format!(
                "GFM program {:?}",
                snapshot.gfm_layer.active_program_id
            )),
            Line::from(format!(
                "BCS {bcs_mode} gain {:.2} enabled {}",
                snapshot.bcs_layer.effective_gain,
                on_off_bool(snapshot.bcs_layer.enabled)
            )),
            Line::from(format!(
                "BCS pitch {:?} unsafe {} / {}",
                snapshot.bcs_layer.pitch_note,
                snapshot.bcs_layer.unsafe_state,
                snapshot.bcs_layer.unsafe_events
            )),
        ]
    } else {
        vec![Line::from("waiting for snapshot")]
    };
    frame.render_widget(panel("GFM / BCS Layers", lines), area);
}

pub(super) fn draw_factory(
    frame: &mut Frame<'_>,
    snapshot: Option<&EngineSnapshot>,
    factory: &[FactoryPatchEntry],
    area: Rect,
) {
    let rows = factory
        .iter()
        .map(|entry| {
            let active = snapshot.is_some_and(|snapshot| snapshot.patch_name == entry.patch_name);
            Row::new(vec![entry.patch_name.clone(), entry.stem.clone()]).style(if active {
                active_style()
            } else {
                Style::default()
            })
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Table::new(
            rows,
            [Constraint::Percentage(55), Constraint::Percentage(45)],
        )
        .header(Row::new(["factory", "stem"]).style(label()))
        .block(Block::default().title("Factory Bank").borders(Borders::ALL)),
        area,
    );
}

pub(super) fn draw_sound_lab(
    frame: &mut Frame<'_>,
    state: &TuiState,
    snapshot: Option<&EngineSnapshot>,
    recording: &RecordingMetricsSnapshot,
    area: Rect,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(8),
        ])
        .split(area);
    let page_titles = SoundLabPage::ALL
        .iter()
        .map(|page| Line::from(page.label()))
        .collect::<Vec<_>>();
    let page_index = SoundLabPage::ALL
        .iter()
        .position(|page| *page == state.sound_lab_page)
        .unwrap_or(0);
    frame.render_widget(
        Tabs::new(page_titles)
            .select(page_index)
            .block(
                Block::default()
                    .title("Sound Lab Pages  [ ]")
                    .borders(Borders::ALL),
            )
            .highlight_style(active_style()),
        rows[0],
    );
    if state.sound_lab_page == SoundLabPage::Layers {
        draw_layers(frame, snapshot, rows[1]);
        draw_recording(frame, state, snapshot, recording, rows[2]);
        return;
    }
    let bindings = sound_lab_bindings(state.sound_lab_page);
    let table_rows = bindings
        .iter()
        .enumerate()
        .map(|(index, binding)| {
            let spec = param_spec(binding.id);
            let value = snapshot
                .and_then(|snapshot| direct_param_display_value(snapshot, binding.id))
                .unwrap_or_else(|| "-".to_string());
            Row::new(vec![
                Cell::from(binding.source.badge()),
                Cell::from(spec.name),
                Cell::from(value),
                Cell::from(format!("{}..{} {:?}", spec.min, spec.max, spec.unit)),
            ])
            .style(if index == state.focused_row {
                active_style()
            } else {
                Style::default()
            })
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Table::new(
            table_rows,
            [
                Constraint::Length(6),
                Constraint::Percentage(38),
                Constraint::Percentage(18),
                Constraint::Percentage(38),
            ],
        )
        .header(Row::new(["src", "parameter", "value", "range"]).style(label()))
        .block(
            Block::default()
                .title("Sound Lab Controls")
                .borders(Borders::ALL),
        ),
        rows[1],
    );
    let macro_params = sound_lab_page_mod_wheel_params(state.sound_lab_page)
        .iter()
        .map(|id| {
            let value = snapshot
                .and_then(|snapshot| direct_param_display_value(snapshot, *id))
                .unwrap_or_else(|| "-".to_string());
            format!("MW {}={value}", param_spec(*id).name)
        })
        .collect::<Vec<_>>()
        .join("  ");
    frame.render_widget(
        panel(
            "Page Macro / Export",
            vec![
                Line::from(macro_params),
                Line::from("e exports patch copy; Enter toggles booleans; arrows edit focused row"),
            ],
        ),
        rows[2],
    );
}

pub(super) fn draw_pc4(
    frame: &mut Frame<'_>,
    snapshot: Option<&EngineSnapshot>,
    input: &InputMetricsSnapshot,
    area: Rect,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(8)])
        .split(area);
    let last = input
        .last_control
        .as_ref()
        .map(last_control_summary)
        .unwrap_or_else(|| "none".to_string());
    frame.render_widget(
        panel(
            "PC4 Surface",
            vec![
                Line::from(format!("last {last}")),
                Line::from(format!(
                    "midi messages {} accepted {} dropped {}",
                    input.midi_messages, input.midi_messages_accepted, input.midi_messages_dropped
                )),
                Line::from("Sound Lab claims K1-K7/K9, S1-S8, MW only while Sound Lab is focused"),
            ],
        ),
        chunks[0],
    );
    let rows = snapshot
        .map(|snapshot| {
            vec![
                Row::new(vec![
                    Cell::from("K1"),
                    Cell::from("Gravitacija"),
                    Cell::from(format!("{:.2}", snapshot.live_macros.gravitacija)),
                ]),
                Row::new(vec![
                    Cell::from("K2"),
                    Cell::from("Bloom"),
                    Cell::from(format!("{:.2}", snapshot.live_macros.bloom)),
                ]),
                Row::new(vec![
                    Cell::from("K3"),
                    Cell::from("Heat"),
                    Cell::from(format!("{:.2}", snapshot.live_macros.heat)),
                ]),
                Row::new(vec![
                    Cell::from("K4"),
                    Cell::from("Ruin"),
                    Cell::from(format!("{:.2}", snapshot.live_macros.ruin)),
                ]),
                Row::new(vec![
                    Cell::from("K5"),
                    Cell::from("Swarm"),
                    Cell::from(format!("{:.2}", snapshot.live_macros.swarm)),
                ]),
                Row::new(vec![
                    Cell::from("S9"),
                    Cell::from("BCS Gain"),
                    Cell::from(format!("{:.2}", snapshot.bcs_layer.gain)),
                ]),
                Row::new(vec![
                    Cell::from("SW9"),
                    Cell::from("BCS Enable"),
                    Cell::from(on_off_bool(snapshot.bcs_layer.enabled).to_string()),
                ]),
            ]
        })
        .unwrap_or_default();
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Length(8),
                Constraint::Percentage(46),
                Constraint::Percentage(46),
            ],
        )
        .header(Row::new(["control", "action", "value"]).style(label()))
        .block(
            Block::default()
                .title("PC4 Canonical Map")
                .borders(Borders::ALL),
        ),
        chunks[1],
    );
}

pub(super) fn last_control_summary(event: &LastControlEvent) -> String {
    let action = compact_text(&event.action, 42);
    format!("{} -> {} ({:?})", event.label, action, event.verdict)
}

pub(super) fn draw_debug(
    frame: &mut Frame<'_>,
    snapshot: Option<&EngineSnapshot>,
    transport: &TransportMetricsSnapshot,
    input: &InputMetricsSnapshot,
    recording: &RecordingMetricsSnapshot,
    area: Rect,
) {
    let lines = vec![
        Line::from(format!(
            "transport queue={} target={} underruns={} overflows={}",
            transport.queued_frames,
            transport.queue_target_frames,
            transport.underrun_batches,
            transport.overflow_batches
        )),
        Line::from(format!(
            "recording state={} frames={} dropped={}",
            recording.state.label(),
            recording.frames_written,
            recording.frames_dropped
        )),
        Line::from(format!(
            "midi messages={} accepted={} dropped={} runtime_dropped={}",
            input.midi_messages,
            input.midi_messages_accepted,
            input.midi_messages_dropped,
            input.runtime_controls_dropped
        )),
        Line::from(
            snapshot
                .map(|snapshot| {
                    format!(
                        "safety peak={:.3} clip={} voices={} held={:?}",
                        snapshot.peak_output,
                        snapshot.clip_detected,
                        snapshot.active_voice_count,
                        snapshot.held_notes
                    )
                })
                .unwrap_or_else(|| "snapshot pending".to_string()),
        ),
        Line::from(
            snapshot
                .map(|snapshot| {
                    format!(
                        "identity h={:.2} p={:.2} b={:.2} g={:.2}",
                        snapshot.identity.horizont_open,
                        snapshot.identity.pec_mass,
                        snapshot.identity.baklja_ready,
                        snapshot.identity.grav_pull
                    )
                })
                .unwrap_or_default(),
        ),
        Line::from(
            snapshot
                .map(|snapshot| {
                    format!(
                        "derived mass={:.2} strain={:.2} headroom={:.2}",
                        snapshot.derived.mass, snapshot.derived.strain, snapshot.derived.headroom
                    )
                })
                .unwrap_or_default(),
        ),
    ];
    frame.render_widget(panel("Debug", lines), area);
}

pub(super) fn draw_footer(frame: &mut Frame<'_>, state: &TuiState, area: Rect) {
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("q", label()),
            Span::raw(" quit  "),
            Span::styled("?", label()),
            Span::raw(" help  "),
            Span::styled("r", label()),
            Span::raw(" record  "),
            Span::styled("p", label()),
            Span::raw(" panic  "),
            Span::styled("0..7", label()),
            Span::raw(" slots  "),
            Span::styled("status ", label()),
            Span::raw(state.status.clone()),
        ]))
        .block(Block::default().borders(Borders::ALL)),
        area,
    );
}

pub(super) fn draw_help(frame: &mut Frame<'_>, area: Rect) {
    let lines = vec![
        Line::from("F1 Live  F2 Sound Lab  F3 PC4  F4 Debug  Tab cycles screens"),
        Line::from("0..7 load live slot  n/b next/previous  p panic  c reset controllers"),
        Line::from("r toggles recording  e exports Sound Lab copy"),
        Line::from("Sound Lab: [ ] page, arrows/hjkl focus/edit, Shift+arrow coarse, Enter toggle"),
        Line::from(
            "Mouse: click top tab, wheel moves focus, drag increments focused Sound Lab value",
        ),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .title("Help")
                    .borders(Borders::ALL)
                    .style(active_style()),
            )
            .wrap(Wrap { trim: true }),
        area,
    );
}
