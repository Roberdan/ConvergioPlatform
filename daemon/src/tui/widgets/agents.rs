// Agent org chart widget — extracted from shared.rs to keep files under 250 lines.

use ratatui::{
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::tui::TuiData;

use super::{selected_style, ACCENT, MUTED, OK, TEXT_PRIMARY, TEXT_SECONDARY, WARN};

/// Agent org chart with tree connectors and status dots.
pub fn agent_org_chart(data: &TuiData, selected: usize) -> Paragraph<'static> {
    let active = data.agents.iter().filter(|a| a.active_task.is_some()).count();
    let total = data.agents.len();
    let mut lines: Vec<Line<'static>> = vec![
        Line::from(vec![
            Span::styled("AGENT ORG CHART", Style::default().fg(ACCENT).bold()),
            Span::raw("  "),
            Span::styled(
                format!("{active}/{total} active"),
                Style::default().fg(if active > 0 { OK } else { MUTED }),
            ),
        ]),
        Line::from(vec![
            Span::styled(" \u{25c9} ", Style::default().fg(ACCENT)),
            Span::styled("ControlRoom", Style::default().fg(WARN)),
        ]),
    ];
    for (i, agent) in data.agents.iter().enumerate() {
        let is_last = i + 1 == data.agents.len();
        let branch = if is_last { "\u{2514}\u{2500}\u{2500}" } else { "\u{251c}\u{2500}\u{2500}" };
        let task = agent.active_task.clone().unwrap_or_else(|| "idle".to_string());
        let is_active = task != "idle";
        let (dot, dot_color) = if is_active {
            ("\u{25cf}", OK) // ●
        } else {
            ("\u{25cb}", MUTED) // ○
        };
        let is_sel = i == selected;
        if is_sel {
            lines.push(
                Line::from(format!(
                    " {branch} {dot} {} ({}) @{} [{task}]",
                    agent.name, agent.role, agent.host,
                ))
                .style(selected_style()),
            );
        } else {
            lines.push(Line::from(vec![
                Span::styled(format!(" {branch} "), Style::default().fg(MUTED)),
                Span::styled(format!("{dot} "), Style::default().fg(dot_color)),
                Span::styled(format!("{} ", agent.name), Style::default().fg(TEXT_PRIMARY).bold()),
                Span::styled(format!("({}) ", agent.role), Style::default().fg(TEXT_SECONDARY)),
                Span::styled(format!("@{} ", agent.host), Style::default().fg(MUTED)),
                Span::styled(
                    format!("[{task}]"),
                    Style::default().fg(if is_active { OK } else { MUTED }),
                ),
            ]));
        }
    }
    if data.agents.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(" \u{2514}\u{2500}\u{2500} ", Style::default().fg(MUTED)),
            Span::styled("\u{25cb} no active agents", Style::default().fg(MUTED)),
        ]));
    }
    Paragraph::new(Text::from(lines))
        .block(Block::default().title(" Agents ").borders(Borders::ALL))
        .wrap(Wrap { trim: true })
}
