//! Live-mode rendering (ratatui). Plain-text panels; no colour dependence.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::state::LiveState;

const LOCKED_SLOTS: [&str; 8] = [
    "molten-horizon",
    "cathedral-bloom",
    "ember-vault",
    "razor-thaw",
    "gravity-wake",
    "furnace-choir",
    "granite-plain",
    "glass-tide",
];

pub fn draw(frame: &mut Frame<'_>, state: &LiveState) {
    let area = frame.area();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(area);

    let header = Paragraph::new(format!(
        "mamut-seq live  ·  port `{}`  ·  channel {}",
        state.port_name(),
        state.channel()
    ))
    .block(Block::default().borders(Borders::ALL).title("mamut-seq"));
    frame.render_widget(header, rows[0]);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(52), Constraint::Percentage(48)])
        .split(rows[1]);

    let status = Paragraph::new(status_text(state))
        .block(Block::default().borders(Borders::ALL).title("state"));
    frame.render_widget(status, body[0]);

    let help =
        Paragraph::new(help_text()).block(Block::default().borders(Borders::ALL).title("keys"));
    frame.render_widget(help, body[1]);

    let footer = Paragraph::new(format!("{}    (Esc quit · Backspace panic)", state.log()))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, rows[2]);
}

fn status_text(state: &LiveState) -> String {
    let release = if state.release_available() {
        "available (kitty)"
    } else {
        "unavailable"
    };
    let program = match state.program() {
        Some(slot) => format!(
            "{slot} ({})",
            LOCKED_SLOTS.get(slot as usize).copied().unwrap_or("?")
        ),
        None => "—".to_string(),
    };
    let cc = match state.selected_control() {
        Some((name, value)) => format!("{name}  {}  {:.0}%", bar(value, 10), value * 100.0),
        None => "(no controls in profile)".to_string(),
    };
    format!(
        "octave:      {}   (base note {})\n\
         note mode:   {}   (release events: {})\n\
         gate:        {} ms\n\
         sustain:     {}\n\
         program:     {}\n\
         pitch bend:  {:+.2}\n\
         aftertouch:  {}  {:.0}%\n\
         cc lane:     {}\n\
         held notes:  {}\n\
         fire scene:  {}",
        state.octave(),
        state.base_note(),
        state.note_mode().label(),
        release,
        state.gate_ms(),
        if state.sustain() { "on" } else { "off" },
        program,
        state.bend(),
        bar(state.aftertouch(), 10),
        state.aftertouch() * 100.0,
        cc,
        state.held_count(),
        if state.has_fire_scenario() {
            "ready (press f)"
        } else {
            "none"
        },
    )
}

fn help_text() -> String {
    let fire = "f  fire preloaded scenario";
    format!(
        "piano (two rows, one octave):\n\
         white  z x c v b n m ,\n\
         black   s d   g h j\n\
         \n\
         - / =    octave down / up\n\
         space    sustain toggle\n\
         w q e    aftertouch ramp up / down / hold\n\
         Up/Dn    pitch bend  ·  0 centre\n\
         Tab/S-Tab  select cc  ·  Left/Right adjust\n\
         PgUp/PgDn  cc set max / min\n\
         1..8     program change 0..7\n\
         Insert   cycle note mode (gate/latch)\n\
         {fire}\n\
         Backspace  panic  ·  Esc  quit"
    )
}

/// A `width`-cell text bar for a normalized `0.0..=1.0` value.
fn bar(value: f32, width: usize) -> String {
    let filled = (value.clamp(0.0, 1.0) * width as f32).round() as usize;
    let filled = filled.min(width);
    let mut out = String::with_capacity(width + 2);
    out.push('[');
    for _ in 0..filled {
        out.push('#');
    }
    for _ in filled..width {
        out.push('-');
    }
    out.push(']');
    out
}
