// Agent org chart widget — extracted from shared.rs to keep files under 250 lines.

use ratatui::{
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::{collections::BTreeMap, sync::atomic::{AtomicBool, Ordering}};

use crate::tui::TuiData;

use super::{selected_style, ACCENT, MUTED, OK, TEXT_PRIMARY, TEXT_SECONDARY, WARN};

static ORG_HIERARCHY_MODE: AtomicBool = AtomicBool::new(false);

pub fn toggle_org_hierarchy_mode() {
    ORG_HIERARCHY_MODE.store(!ORG_HIERARCHY_MODE.load(Ordering::Relaxed), Ordering::Relaxed);
}

#[cfg(test)]
pub fn set_org_hierarchy_mode(value: bool) {
    ORG_HIERARCHY_MODE.store(value, Ordering::Relaxed);
}

/// Agent org chart with tree connectors and status dots.
pub fn agent_org_chart(data: &TuiData, selected: usize) -> Paragraph<'static> {
    if ORG_HIERARCHY_MODE.load(Ordering::Relaxed) {
        return agent_org_hierarchy(data);
    }
    agent_flat_list(data, selected)
}

fn agent_flat_list(data: &TuiData, selected: usize) -> Paragraph<'static> {
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

fn agent_org_hierarchy(data: &TuiData) -> Paragraph<'static> {
    let mut by_org: BTreeMap<String, Vec<&crate::tui::AgentOrgNode>> = BTreeMap::new();
    for agent in &data.agents {
        let org = agent.host.split('-').next().unwrap_or("core").to_string();
        by_org.entry(org).or_default().push(agent);
    }
    let mut lines: Vec<Line<'static>> = vec![Line::from(vec![
        Span::styled("ORG HIERARCHY (o toggles)", Style::default().fg(ACCENT).bold()),
    ])];
    if by_org.is_empty() {
        lines.push(Line::from(" └── ○ no agents"));
    }
    for (org, members) in by_org {
        let active = members.iter().filter(|a| a.active_task.is_some()).count();
        let budget = ((active as f32 / members.len().max(1) as f32) * 100.0).round() as i32;
        let ceo = members
            .iter()
            .find(|a| a.role.to_lowercase().contains("ceo"))
            .unwrap_or(&members[0]);
        let status = if budget >= 67 { "ACTIVE" } else if budget > 0 { "WARM" } else { "IDLE" };
        let status_color = if budget >= 67 { OK } else if budget > 0 { WARN } else { MUTED };
        lines.push(Line::from(vec![
            Span::styled("🏢 ", Style::default().fg(ACCENT)),
            Span::styled(format!("{org} "), Style::default().fg(TEXT_PRIMARY).bold()),
            Span::styled(format!("[{status}] "), Style::default().fg(status_color)),
            Span::styled(
                format!("members:{} budget:{}%", members.len(), budget),
                Style::default().fg(TEXT_SECONDARY),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  CEO: ", Style::default().fg(MUTED)),
            Span::styled(format!("{} ", ceo.name), Style::default().fg(TEXT_PRIMARY)),
            Span::styled(format!("({})", ceo.role), Style::default().fg(TEXT_SECONDARY)),
        ]));
        let mut by_dept: BTreeMap<String, Vec<&crate::tui::AgentOrgNode>> = BTreeMap::new();
        for a in members {
            let dept = a.role.split(':').next().unwrap_or("General").to_string();
            by_dept.entry(dept).or_default().push(a);
        }
        let dept_count = by_dept.len();
        for (idx, (dept, agents)) in by_dept.into_iter().enumerate() {
            let dept_branch = if idx + 1 == dept_count { "└──" } else { "├──" };
            lines.push(Line::from(format!("  {dept_branch} {dept}")));
            for a in agents {
                let dot = if a.active_task.is_some() { "●" } else { "○" };
                lines.push(Line::from(vec![
                    Span::styled("  │   ", Style::default().fg(MUTED)),
                    Span::styled(format!("{dot} {} ", a.name), Style::default().fg(TEXT_PRIMARY)),
                    Span::styled(format!("[{}] ", a.role), Style::default().fg(TEXT_SECONDARY)),
                    Span::styled(
                        format!("({})", a.active_task.as_deref().unwrap_or("idle")),
                        Style::default().fg(if a.active_task.is_some() { OK } else { MUTED }),
                    ),
                ]));
            }
        }
    }
    Paragraph::new(Text::from(lines))
        .block(Block::default().title(" Agents ").borders(Borders::ALL))
        .wrap(Wrap { trim: true })
}
