use ratatui::{
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::tui::TuiData;

use super::{selected_style, ACCENT, FAIL, MUTED, OK, TEXT_PRIMARY, TEXT_SECONDARY, WARN};

pub fn task_pipeline(data: &TuiData, selected: usize) -> Paragraph<'static> {
    let mut lines: Vec<Line<'static>> = vec![
        "TASK PIPELINE".bold().fg(ACCENT).into(),
        "ID       Status        Agent       Title".fg(WARN).into(),
        "".into(),
    ];
    for (i, task) in data.pipeline.iter().enumerate() {
        let (status, color) = match task.status.as_str() {
            "in_progress" => ("IN_PROGRESS", WARN),
            "submitted" => ("SUBMITTED", OK),
            "done" => ("DONE", OK),
            "blocked" => ("BLOCKED", FAIL),
            _ => ("PENDING", MUTED),
        };
        let base = Style::default().fg(color);
        let style = if i == selected { base.reversed() } else { base };
        lines.push(
            Line::from(format!(
                "{:<8} {:<13} {:<10} {}",
                task.task_id, status, task.agent, task.title
            ))
            .style(style),
        );
    }
    if data.pipeline.is_empty() {
        lines.push("No tasks in pipeline".fg(MUTED).into());
    }
    Paragraph::new(Text::from(lines))
        .block(Block::default().title(" Tasks ").borders(Borders::ALL))
        .wrap(Wrap { trim: true })
}

/// Mesh status with status dots, spark bars, and role badges.
pub fn mesh_status(data: &TuiData, selected: usize) -> Paragraph<'static> {
    let online = data.mesh_nodes.iter().filter(|n| n.online).count();
    let total = data.mesh_nodes.len();
    let mut lines: Vec<Line<'static>> = vec![
        Line::from(vec![
            Span::styled("MESH STATUS", Style::default().fg(ACCENT).bold()),
            Span::raw("  "),
            Span::styled(
                format!("\u{25cf} {online}/{total} online"),
                Style::default().fg(if online == total { OK } else { WARN }),
            ),
        ]),
        "".into(),
        Line::from(vec![
            Span::styled(
                format!("{:<16} {:<4} {:<12} {:<6} {}",
                    "Node", "St", "Role", "CPU", "Load"),
                Style::default().fg(TEXT_SECONDARY),
            ),
        ]),
    ];
    for (i, node) in data.mesh_nodes.iter().enumerate() {
        let (dot, color) = if node.online {
            ("\u{25cf}", OK) // ●
        } else {
            ("\u{25cb}", FAIL) // ○
        };
        let cpu_int = node.cpu_percent as i64;
        let cpu_color = if cpu_int > 80 { FAIL } else if cpu_int > 50 { WARN } else { OK };
        let spark_str = spark(cpu_int);
        let base = Style::default();
        let is_sel = i == selected;
        if is_sel {
            lines.push(
                Line::from(format!(
                    " {dot} {:<14} {:<12} {:>3}%  {spark_str}",
                    node.name, node.role, cpu_int,
                ))
                .style(selected_style()),
            );
        } else {
            lines.push(Line::from(vec![
                Span::raw(" "),
                Span::styled(format!("{dot} "), Style::default().fg(color)),
                Span::styled(format!("{:<14} ", node.name), Style::default().fg(TEXT_PRIMARY)),
                Span::styled(format!("{:<12} ", node.role), Style::default().fg(TEXT_SECONDARY)),
                Span::styled(format!("{:>3}%", cpu_int), Style::default().fg(cpu_color)),
                Span::raw("  "),
                Span::styled(spark_str, base.fg(cpu_color)),
            ]));
        }
    }
    if data.mesh_nodes.is_empty() {
        lines.push(Line::from(vec![
            Span::raw(" "),
            Span::styled("\u{25cb} No mesh peers found", Style::default().fg(MUTED)),
        ]));
    }
    Paragraph::new(Text::from(lines))
        .block(Block::default().title(" Mesh ").borders(Borders::ALL))
        .wrap(Wrap { trim: true })
}

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

pub fn spark(cpu: i64) -> String {
    let levels = ["\u{2581}", "\u{2582}", "\u{2583}", "\u{2584}", "\u{2585}", "\u{2586}", "\u{2587}", "\u{2588}"];
    let clamped = cpu.clamp(0, 100) as usize;
    let idx = clamped * (levels.len() - 1) / 100;
    levels[idx].repeat(6)
}

pub fn progress_bar(pct: u16, width: u16) -> String {
    let filled = ((pct as u32 * width as u32) / 100) as usize;
    let empty = width as usize - filled;
    format!("[{}{}]", "\u{2588}".repeat(filled), "\u{2591}".repeat(empty))
}

/// Colored progress bar as styled spans: green >=80%, yellow >=50%, red <50%.
pub fn progress_bar_line(pct: u16, width: u16) -> Line<'static> {
    let color = if pct >= 80 { OK } else if pct >= 50 { WARN } else { FAIL };
    let filled = ((pct as u32 * width as u32) / 100) as usize;
    let empty = width as usize - filled;
    Line::from(vec![
        Span::raw("["),
        Span::styled("\u{2588}".repeat(filled), Style::default().fg(color)),
        Span::styled("\u{2591}".repeat(empty), Style::default().fg(MUTED)),
        Span::raw("]"),
    ])
}
