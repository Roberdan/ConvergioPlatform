// Project switcher overlay — Ctrl+P, list projects, Enter to switch, Esc to close.

use ratatui::{
    layout::Rect,
    style::{Style, Stylize},
    text::{Line, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::tui::{
    data::ProjectInfo,
    widgets::{ACCENT, MUTED, TEXT_PRIMARY},
};

const MIN_WIDTH: u16 = 50;
const BORDER_AND_HEADER: u16 = 4; // 2 border + 1 blank + 1 hint line

/// Render a project switcher overlay centred on `area`.
/// `projects` is the full list; `selected` is the highlighted index.
pub fn render_project_switcher(
    frame: &mut Frame<'_>,
    area: Rect,
    projects: &[ProjectInfo],
    selected: usize,
) {
    let height = (projects.len() as u16 + BORDER_AND_HEADER + 1).max(6);
    let width = projects
        .iter()
        .map(|p| p.name.len() + p.path.len() + 6)
        .max()
        .unwrap_or(20) as u16;
    let width = width.max(MIN_WIDTH);

    let popup = centered_rect(width, height, area);
    frame.render_widget(Clear, popup);

    let mut lines: Vec<Line<'static>> = vec![
        Line::from("  ↑↓ Navigate  Enter Select  Esc Close")
            .style(Style::default().fg(MUTED)),
        "".into(),
    ];

    for (i, proj) in projects.iter().enumerate() {
        let label = format!("  {} — {}", proj.name, proj.path);
        if i == selected {
            lines.push(
                Line::from(label).style(Style::default().fg(ACCENT).bold().reversed()),
            );
        } else {
            lines.push(Line::from(label).style(Style::default().fg(TEXT_PRIMARY)));
        }
    }

    if projects.is_empty() {
        lines.push(Line::from("  (no projects)").style(Style::default().fg(MUTED)));
    }

    let block = Block::default()
        .title(Line::from(" Switch Project ").fg(ACCENT).bold())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(MUTED));

    frame.render_widget(Paragraph::new(Text::from(lines)).block(block), popup);
}

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
