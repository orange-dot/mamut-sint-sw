use std::{
    io::{self, IsTerminal},
    time::{Duration, Instant},
};

use anyhow::{Result, anyhow};
use crossterm::{
    cursor::{Hide, Show},
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
        MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use mamut_engine::{BcsLayerMode, EngineSnapshot, GfmLayerMode};
use mamut_params::{ParamUnit, param_spec};
use mamut_runtime::*;
use ratatui::{
    Frame, Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Gauge, Paragraph, Row, Table, Tabs, Wrap},
};

const TUI_TICK: Duration = Duration::from_millis(80);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiScreen {
    Live,
    SoundLab,
    Pc4,
    Debug,
}

impl TuiScreen {
    const ALL: [Self; 4] = [Self::Live, Self::SoundLab, Self::Pc4, Self::Debug];

    fn title(self) -> &'static str {
        match self {
            Self::Live => "Live",
            Self::SoundLab => "Sound Lab",
            Self::Pc4 => "PC4",
            Self::Debug => "Debug",
        }
    }

    fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|screen| *screen == self)
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone)]
pub struct TuiState {
    pub screen: TuiScreen,
    pub sound_lab_page: SoundLabPage,
    pub focused_row: usize,
    pub record_seconds: u64,
    pub take_tag: String,
    pub show_help: bool,
    pub status: String,
}

impl Default for TuiState {
    fn default() -> Self {
        Self {
            screen: TuiScreen::Live,
            sound_lab_page: SoundLabPage::Osc1,
            focused_row: 0,
            record_seconds: DEFAULT_LIVE_TAKE_SECONDS,
            take_tag: DEFAULT_LIVE_TAKE_TAG.to_string(),
            show_help: false,
            status: "ready".to_string(),
        }
    }
}

pub fn run_tui_session(session: RuntimeSession) -> Result<()> {
    if !io::stdin().is_terminal() {
        return Err(anyhow!("TUI requires an interactive terminal"));
    }

    enable_raw_mode()?;
    let _guard = TerminalRestore;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture, Hide)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    run_tui_loop(&mut terminal, session)
}

struct TerminalRestore;

impl Drop for TerminalRestore {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, Show, DisableMouseCapture, LeaveAlternateScreen);
    }
}

fn run_tui_loop<B>(terminal: &mut Terminal<B>, mut session: RuntimeSession) -> Result<()>
where
    B: Backend,
    B::Error: Send + Sync + 'static,
{
    let mut state = TuiState::default();
    let live_set = live_patch_entries().unwrap_or_default();
    let factory = factory_patch_entries().unwrap_or_default();
    let mut snapshot = session.request_snapshot().ok();
    let mut last_snapshot_refresh = Instant::now();

    loop {
        if last_snapshot_refresh.elapsed() >= PERFORMANCE_UI_REFRESH {
            match session.request_snapshot() {
                Ok(next) => {
                    snapshot = Some(next);
                }
                Err(error) => {
                    state.status = format!("snapshot failed: {error}");
                }
            }
            last_snapshot_refresh = Instant::now();
        }
        session.print_runtime_control_messages()?;
        session.set_sound_lab_midi_focus(
            (state.screen == TuiScreen::SoundLab).then_some(state.sound_lab_page),
        );

        let transport = session.transport_metrics_snapshot();
        let input = session.input_metrics_snapshot();
        let recording = session.recording_metrics_snapshot();
        terminal.draw(|frame| {
            draw_tui(
                frame,
                &state,
                snapshot.as_ref(),
                &transport,
                &input,
                &recording,
                &live_set,
                &factory,
            );
        })?;

        if event::poll(TUI_TICK)? {
            match event::read()? {
                Event::Key(key) => {
                    if handle_key(key, &mut state, &mut session, snapshot.as_ref(), &live_set)? {
                        break;
                    }
                }
                Event::Mouse(mouse) => {
                    handle_mouse(mouse, &mut state, &mut session, snapshot.as_ref())?
                }
                Event::Resize(_, _) | Event::FocusGained | Event::FocusLost | Event::Paste(_) => {}
            }
        }
    }

    session.set_sound_lab_midi_focus(None);
    Ok(())
}

fn handle_key(
    key: KeyEvent,
    state: &mut TuiState,
    session: &mut RuntimeSession,
    snapshot: Option<&EngineSnapshot>,
    live_set: &[FactoryPatchEntry],
) -> Result<bool> {
    match key.code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Char('?') => state.show_help = !state.show_help,
        KeyCode::F(1) => select_screen(state, TuiScreen::Live),
        KeyCode::F(2) => select_screen(state, TuiScreen::SoundLab),
        KeyCode::F(3) => select_screen(state, TuiScreen::Pc4),
        KeyCode::F(4) => select_screen(state, TuiScreen::Debug),
        KeyCode::Tab => cycle_screen(state, 1),
        KeyCode::BackTab => cycle_screen(state, -1),
        KeyCode::Char('n') => run_status(state, session.switch_favorite(1), "next favorite"),
        KeyCode::Char('b') => run_status(state, session.switch_favorite(-1), "previous favorite"),
        KeyCode::Char('p') => run_status(state, session.panic(), "panic"),
        KeyCode::Char('c') => run_status(state, session.reset_controllers(), "reset controllers"),
        KeyCode::Char('r') => toggle_recording(state, session)?,
        KeyCode::Char('e') if state.screen == TuiScreen::SoundLab => {
            let name = snapshot
                .map(|snapshot| format!("{} TUI", snapshot.patch_name))
                .unwrap_or_else(|| "Mamut TUI".to_string());
            match session.export_sound_lab_patch(&name) {
                Ok(path) => state.status = format!("exported {}", path.display()),
                Err(error) => state.status = format!("export failed: {error}"),
            }
        }
        KeyCode::Char(ch @ '0'..='7') => {
            let slot = usize::from(ch as u8 - b'0');
            if slot < live_set.len() {
                run_status(
                    state,
                    session.load_favorite_slot(slot),
                    &format!("slot {slot}"),
                );
            }
        }
        KeyCode::Char('[') => adjust_sound_lab_page(state, -1),
        KeyCode::Char(']') => adjust_sound_lab_page(state, 1),
        KeyCode::Up | KeyCode::Char('k') => move_focus(state, -1),
        KeyCode::Down | KeyCode::Char('j') => move_focus(state, 1),
        KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('-') => {
            adjust_focused_value(state, session, snapshot, -step_for_key(key))?;
        }
        KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('+') | KeyCode::Char('=') => {
            adjust_focused_value(state, session, snapshot, step_for_key(key))?;
        }
        KeyCode::Enter => activate_focused_value(state, session, snapshot)?,
        _ => {}
    }
    Ok(false)
}

fn handle_mouse(
    mouse: MouseEvent,
    state: &mut TuiState,
    session: &mut RuntimeSession,
    snapshot: Option<&EngineSnapshot>,
) -> Result<()> {
    match mouse.kind {
        MouseEventKind::Down(_) => {
            if mouse.row <= 2 {
                let width = 16;
                let index = usize::from(mouse.column) / width;
                if let Some(screen) = TuiScreen::ALL.get(index) {
                    select_screen(state, *screen);
                }
            }
        }
        MouseEventKind::ScrollUp => move_focus(state, -1),
        MouseEventKind::ScrollDown => move_focus(state, 1),
        MouseEventKind::Drag(_) => adjust_focused_value(state, session, snapshot, 0.01)?,
        MouseEventKind::Up(_)
        | MouseEventKind::Moved
        | MouseEventKind::ScrollLeft
        | MouseEventKind::ScrollRight => {}
    }
    Ok(())
}

fn select_screen(state: &mut TuiState, screen: TuiScreen) {
    state.screen = screen;
    state.focused_row = 0;
}

fn cycle_screen(state: &mut TuiState, direction: isize) {
    let len = TuiScreen::ALL.len() as isize;
    let next = (state.screen.index() as isize + direction).rem_euclid(len) as usize;
    select_screen(state, TuiScreen::ALL[next]);
}

fn move_focus(state: &mut TuiState, direction: isize) {
    let len = focused_row_count(state).max(1) as isize;
    state.focused_row = (state.focused_row as isize + direction).rem_euclid(len) as usize;
}

fn focused_row_count(state: &TuiState) -> usize {
    match state.screen {
        TuiScreen::Live => 8,
        TuiScreen::SoundLab => sound_lab_bindings(state.sound_lab_page).len().max(1),
        TuiScreen::Pc4 => 32,
        TuiScreen::Debug => 16,
    }
}

fn adjust_sound_lab_page(state: &mut TuiState, direction: isize) {
    if state.screen != TuiScreen::SoundLab {
        return;
    }
    let pages = SoundLabPage::ALL;
    let index = pages
        .iter()
        .position(|page| *page == state.sound_lab_page)
        .unwrap_or(0);
    let next = (index as isize + direction).rem_euclid(pages.len() as isize) as usize;
    state.sound_lab_page = pages[next];
    state.focused_row = 0;
}

fn step_for_key(key: KeyEvent) -> f32 {
    if key.modifiers.contains(KeyModifiers::SHIFT) {
        0.10
    } else {
        0.02
    }
}

fn adjust_focused_value(
    state: &mut TuiState,
    session: &RuntimeSession,
    snapshot: Option<&EngineSnapshot>,
    normalized_delta: f32,
) -> Result<()> {
    if state.screen != TuiScreen::SoundLab {
        return Ok(());
    }
    let Some(snapshot) = snapshot else {
        return Ok(());
    };
    let Some(binding) = sound_lab_bindings(state.sound_lab_page)
        .get(state.focused_row)
        .copied()
    else {
        return Ok(());
    };
    let spec = param_spec(binding.id);
    let current = direct_param_raw_value(snapshot, binding.id).unwrap_or(spec.default);
    let value = match spec.unit {
        ParamUnit::Boolean => {
            if normalized_delta > 0.0 {
                1.0
            } else {
                0.0
            }
        }
        ParamUnit::Indexed | ParamUnit::Semitones => (current + normalized_delta.signum())
            .round()
            .clamp(spec.min, spec.max),
        _ => {
            let range = spec.max - spec.min;
            (current + range * normalized_delta).clamp(spec.min, spec.max)
        }
    };
    session.set_direct_param(binding.id, value)?;
    state.status = format!("{} -> {}", spec.name, format_param_value(binding.id, value));
    Ok(())
}

fn activate_focused_value(
    state: &mut TuiState,
    session: &RuntimeSession,
    snapshot: Option<&EngineSnapshot>,
) -> Result<()> {
    if state.screen != TuiScreen::SoundLab {
        return Ok(());
    }
    let Some(snapshot) = snapshot else {
        return Ok(());
    };
    let Some(binding) = sound_lab_bindings(state.sound_lab_page)
        .get(state.focused_row)
        .copied()
    else {
        return Ok(());
    };
    if param_spec(binding.id).unit == ParamUnit::Boolean {
        let current = direct_param_raw_value(snapshot, binding.id).unwrap_or(0.0);
        let next = if current >= 0.5 { 0.0 } else { 1.0 };
        session.set_direct_param(binding.id, next)?;
        state.status = format!(
            "{} -> {}",
            param_spec(binding.id).name,
            format_param_value(binding.id, next)
        );
    }
    Ok(())
}

fn toggle_recording(state: &mut TuiState, session: &RuntimeSession) -> Result<()> {
    let recording = session.recording_metrics_snapshot();
    if recording.state == RecordingState::Active {
        match session.stop_output_recording()? {
            Some(path) => state.status = format!("recording stop -> {}", path.display()),
            None => state.status = "no active recording".to_string(),
        }
    } else {
        match session.start_tagged_output_recording(state.record_seconds, &state.take_tag) {
            Ok(path) => {
                state.status = format!("recording {}s -> {}", state.record_seconds, path.display())
            }
            Err(error) => state.status = format!("record failed: {error}"),
        }
    }
    Ok(())
}

fn run_status(state: &mut TuiState, result: Result<()>, action: &str) {
    match result {
        Ok(()) => state.status = format!("{action}: ok"),
        Err(error) => state.status = format!("{action}: {error}"),
    }
}

fn sound_lab_bindings(page: SoundLabPage) -> Vec<SoundLabMidiParamBinding> {
    sound_lab_page_knob_bindings(page)
        .iter()
        .chain(sound_lab_page_slider_bindings(page).iter())
        .copied()
        .collect()
}

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

fn draw_tabs(frame: &mut Frame<'_>, state: &TuiState, area: Rect) {
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

fn draw_live(
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

fn draw_patch_status(frame: &mut Frame<'_>, snapshot: Option<&EngineSnapshot>, area: Rect) {
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

fn draw_recording(
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

fn draw_transport(
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
                .map(|event| format!("last {} -> {}", event.label, event.action))
                .unwrap_or_else(|| "last none".to_string()),
        ),
    ];
    frame.render_widget(panel("Runtime Health", lines), area);
}

fn draw_live_set(
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

fn draw_macros(frame: &mut Frame<'_>, snapshot: Option<&EngineSnapshot>, area: Rect) {
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

fn draw_layers(frame: &mut Frame<'_>, snapshot: Option<&EngineSnapshot>, area: Rect) {
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

fn draw_factory(
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

fn draw_sound_lab(
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

fn draw_pc4(
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
        .map(|event| format!("{} -> {} ({:?})", event.label, event.action, event.verdict))
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

fn draw_debug(
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

fn draw_footer(frame: &mut Frame<'_>, state: &TuiState, area: Rect) {
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

fn draw_help(frame: &mut Frame<'_>, area: Rect) {
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

fn panel<'a>(title: &'a str, lines: Vec<Line<'a>>) -> Paragraph<'a> {
    Paragraph::new(lines)
        .block(Block::default().title(title).borders(Borders::ALL))
        .wrap(Wrap { trim: true })
}

fn label() -> Style {
    Style::default()
        .fg(Color::LightCyan)
        .add_modifier(Modifier::BOLD)
}

fn active_style() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(Color::LightCyan)
        .add_modifier(Modifier::BOLD)
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1]);
    horizontal[1]
}

fn inner_line(rect: Rect, parent: Rect) -> Rect {
    Rect {
        x: rect.x.saturating_add(1),
        y: rect.y.max(parent.y.saturating_add(1)),
        width: rect.width.saturating_sub(2),
        height: rect.height.min(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    fn smoke_screen(screen: TuiScreen) {
        let backend = TestBackend::new(100, 32);
        let mut terminal = Terminal::new(backend).expect("test terminal builds");
        let mut state = TuiState {
            screen,
            ..TuiState::default()
        };
        if screen == TuiScreen::SoundLab {
            state.sound_lab_page = SoundLabPage::Spectral;
        }
        let transport = TransportMetrics::default().snapshot();
        let input = InputMetrics::default().snapshot();
        let recording = RecordingMetrics::default().snapshot();
        terminal
            .draw(|frame| {
                draw_tui(
                    frame,
                    &state,
                    None,
                    &transport,
                    &input,
                    &recording,
                    &[],
                    &[],
                );
            })
            .expect("screen renders");
    }

    #[test]
    fn renders_all_canonical_screens() {
        for screen in TuiScreen::ALL {
            smoke_screen(screen);
        }
    }

    #[test]
    fn sound_lab_focus_wraps_over_page_bindings() {
        let mut state = TuiState {
            screen: TuiScreen::SoundLab,
            sound_lab_page: SoundLabPage::Osc1,
            ..TuiState::default()
        };
        move_focus(&mut state, -1);
        assert_eq!(
            state.focused_row,
            sound_lab_bindings(SoundLabPage::Osc1).len() - 1
        );
        adjust_sound_lab_page(&mut state, 1);
        assert_eq!(state.sound_lab_page, SoundLabPage::Osc2);
        assert_eq!(state.focused_row, 0);
    }
}
