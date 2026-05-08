use super::*;

pub(super) fn handle_key(
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

pub(super) fn handle_mouse(
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

pub(super) fn select_screen(state: &mut TuiState, screen: TuiScreen) {
    state.screen = screen;
    state.focused_row = 0;
}

pub(super) fn cycle_screen(state: &mut TuiState, direction: isize) {
    let len = TuiScreen::ALL.len() as isize;
    let next = (state.screen.index() as isize + direction).rem_euclid(len) as usize;
    select_screen(state, TuiScreen::ALL[next]);
}

pub(super) fn move_focus(state: &mut TuiState, direction: isize) {
    let len = focused_row_count(state).max(1) as isize;
    state.focused_row = (state.focused_row as isize + direction).rem_euclid(len) as usize;
}

pub(super) fn focused_row_count(state: &TuiState) -> usize {
    match state.screen {
        TuiScreen::Live => 8,
        TuiScreen::SoundLab => sound_lab_bindings(state.sound_lab_page).len().max(1),
        TuiScreen::Pc4 => 32,
        TuiScreen::Debug => 16,
    }
}

pub(super) fn adjust_sound_lab_page(state: &mut TuiState, direction: isize) {
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

pub(super) fn step_for_key(key: KeyEvent) -> f32 {
    if key.modifiers.contains(KeyModifiers::SHIFT) {
        0.10
    } else {
        0.02
    }
}

pub(super) fn adjust_focused_value(
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

pub(super) fn activate_focused_value(
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

pub(super) fn toggle_recording(state: &mut TuiState, session: &RuntimeSession) -> Result<()> {
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

pub(super) fn run_status(state: &mut TuiState, result: Result<()>, action: &str) {
    match result {
        Ok(()) => state.status = format!("{action}: ok"),
        Err(error) => state.status = format!("{action}: {error}"),
    }
}

pub(super) fn sound_lab_bindings(page: SoundLabPage) -> Vec<SoundLabMidiParamBinding> {
    sound_lab_page_knob_bindings(page)
        .iter()
        .chain(sound_lab_page_slider_bindings(page).iter())
        .copied()
        .collect()
}
