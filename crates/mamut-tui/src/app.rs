use super::*;

pub fn run_tui_session(mut session: RuntimeSession) -> Result<()> {
    if !io::stdin().is_terminal() {
        return Err(anyhow!("TUI requires an interactive terminal"));
    }

    session.set_terminal_output_enabled(false);
    enable_raw_mode()?;
    let _guard = TerminalRestore;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture, Hide)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    run_tui_loop(&mut terminal, session)
}

pub(super) struct TerminalRestore;

impl Drop for TerminalRestore {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, Show, DisableMouseCapture, LeaveAlternateScreen);
    }
}

pub(super) fn run_tui_loop<B>(terminal: &mut Terminal<B>, mut session: RuntimeSession) -> Result<()>
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
        for message in session.poll_runtime_control_messages()? {
            state.status = compact_status(&message);
        }
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
