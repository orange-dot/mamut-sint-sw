use super::*;

pub(super) fn panel<'a>(title: &'a str, lines: Vec<Line<'a>>) -> Paragraph<'a> {
    Paragraph::new(lines).block(Block::default().title(title).borders(Borders::ALL))
}

pub(super) fn compact_status(value: &str) -> String {
    compact_text(value, 96)
}

pub(super) fn compact_text(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    let keep = max_chars.saturating_sub(3);
    let mut output = value.chars().take(keep).collect::<String>();
    output.push_str("...");
    output
}

pub(super) fn label() -> Style {
    Style::default()
        .fg(Color::LightCyan)
        .add_modifier(Modifier::BOLD)
}

pub(super) fn active_style() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(Color::LightCyan)
        .add_modifier(Modifier::BOLD)
}

pub(super) fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
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

pub(super) fn inner_line(rect: Rect, parent: Rect) -> Rect {
    Rect {
        x: rect.x.saturating_add(1),
        y: rect.y.max(parent.y.saturating_add(1)),
        width: rect.width.saturating_sub(2),
        height: rect.height.min(1),
    }
}
