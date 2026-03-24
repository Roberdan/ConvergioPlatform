// Help overlay — rendered on top of any active view when show_help=true.

use ratatui::{
    layout::Rect,
    style::{Style, Stylize},
    text::{Line, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::tui::widgets::{ACCENT, MUTED, TEXT_PRIMARY};

const HELP_ROWS: &[(&str, &str)] = &[
    ("1-9", "Switch view"),
    ("↑ ↓", "Navigate"),
    ("Tab", "Next view"),
    ("Enter", "Drill down"),
    ("/", "Command"),
    ("r", "Manual refresh"),
    ("R", "Toggle auto-refresh"),
    ("+/-", "Adjust interval"),
    ("?", "Toggle help"),
    ("q", "Quit"),
];

/// Renders a centered help overlay using Clear + Rounded block.
pub fn render_help_overlay(frame: &mut Frame<'_>, area: Rect) {
    let width: u16 = 30;
    // +4: 2 border + 2 padding rows (title + blank)
    let height: u16 = HELP_ROWS.len() as u16 + 4;
    let popup = centered_rect(width, height, area);

    frame.render_widget(Clear, popup);

    let mut lines: Vec<Line<'static>> = vec!["".into()];
    for (key, desc) in HELP_ROWS {
        lines.push(
            Line::from(format!("  {:<8} {}", key, desc))
                .style(Style::default().fg(TEXT_PRIMARY)),
        );
    }

    let block = Block::default()
        .title(Line::from(" Help ").fg(ACCENT).bold())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(MUTED));

    let paragraph = Paragraph::new(Text::from(lines)).block(block);
    frame.render_widget(paragraph, popup);
}

/// Returns a centered Rect of (width x height) within the given area.
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}
