#![allow(clippy::expect_used)]

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
