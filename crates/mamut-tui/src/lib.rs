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

mod app;
pub use app::run_tui_session;

mod input;
use input::*;

mod render;
pub use render::draw_tui;

mod widgets;
use widgets::*;

#[cfg(test)]
mod tests;
