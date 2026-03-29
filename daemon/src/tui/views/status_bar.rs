// Status bar and command footer renderers extracted from views/mod.rs.
// Why: keep mod.rs ≤250 lines per CONSTITUTION Article V.
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::tui::widgets::{ACCENT, MUTED, OK, TEXT_PRIMARY};

pub fn render_status_bar(
    frame: &mut Frame<'_>,
    area: Rect,
    api_url: &str,
    auto_refresh: bool,
    refresh_interval_secs: u64,
    unread_notifications: usize,
) {
    // Strip scheme for compact display: "http://localhost:8420" → "localhost:8420"
    let host = api_url
        .trim_start_matches("https://")
        .trim_start_matches("http://");

    // Refresh state indicator: "Auto: 5s" or "Auto: OFF"
    let refresh_str = if auto_refresh {
        format!("Auto: {}s", refresh_interval_secs)
    } else {
        "Auto: OFF".to_string()
    };

    // Right side hints differ based on refresh state.
    let right = if auto_refresh {
        format!(" {} │ R Toggle  +/- Interval  ↑↓ Navigate  ? Help  q Quit ", refresh_str)
    } else {
        format!(" {} │ R Toggle  r Refresh  ↑↓ Navigate  ? Help  q Quit ", refresh_str)
    };

    let left = format!(" ◆ Connected {}  │  WS: active ", host);

    let mut spans = vec![Span::styled(left, Style::default().fg(OK))];
    // Unread notification badge (only when count > 0)
    if unread_notifications > 0 {
        spans.push(Span::styled(
            format!("[{} unread]", unread_notifications),
            Style::default().fg(ACCENT).bold(),
        ));
    }
    spans.push(Span::styled("│", Style::default().fg(MUTED)));
    spans.push(Span::styled(right, Style::default().fg(TEXT_PRIMARY)));

    let paragraph = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded),
    );
    frame.render_widget(paragraph, area);
}

/// Renders the bottom command bar. Shows input when in command mode, hints otherwise.
pub fn render_command_footer(frame: &mut Frame<'_>, area: Rect, command_input: Option<&str>) {
    let text = if let Some(input) = command_input {
        format!("> {}", input)
    } else {
        " [1]Tree [2]Chat [3]Pipeline [4]Mesh [5]Agents [6]Brain [7]Cost [8]Events [9]WS [0]Deliv  /  Tab  q".to_string()
    };
    let paragraph = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(Style::default().fg(MUTED)),
    );
    frame.render_widget(paragraph, area);
}
