// Project list widget for TUI ProjectView tab.

use ratatui::{
    style::{Style, Stylize},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::tui::TuiData;

use super::{selected_style, ACCENT, MUTED, TEXT_PRIMARY, TEXT_SECONDARY};

/// Project list view showing registered projects.
pub fn project_list(data: &TuiData, selected: usize) -> Paragraph<'static> {
    let mut lines: Vec<Line<'static>> = vec![
        "PROJECTS".bold().fg(ACCENT).into(),
        "ID       Name                 Path".fg(TEXT_SECONDARY).into(),
        "".into(),
    ];
    for (i, proj) in data.projects.iter().enumerate() {
        let style = if i == selected {
            selected_style()
        } else {
            Style::default().fg(TEXT_PRIMARY)
        };
        lines.push(
            Line::from(format!(
                "{:<8} {:<20} {}",
                proj.id, proj.name, proj.path
            ))
            .style(style),
        );
    }
    if data.projects.is_empty() {
        lines.push("No projects registered".fg(MUTED).into());
    }
    Paragraph::new(Text::from(lines))
        .block(Block::default().title(" Projects ").borders(Borders::ALL))
        .wrap(Wrap { trim: true })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_list_shows_header() {
        let data = TuiData::default();
        let p = project_list(&data, 0);
        let debug = format!("{p:?}");
        assert!(debug.contains("PROJECTS"), "Must contain header: {debug}");
    }

    #[test]
    fn project_list_empty_shows_fallback() {
        let data = TuiData::default();
        let p = project_list(&data, 0);
        let debug = format!("{p:?}");
        assert!(
            debug.contains("No projects registered"),
            "Empty state: {debug}"
        );
    }
}
