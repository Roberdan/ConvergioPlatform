use std::collections::BTreeMap;

use ratatui::{
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use super::TuiData;

// --- Maranello color palette ---

pub const ACCENT_U32: u32 = 0x00FFC72C;
pub const BG_DARK_U32: u32 = 0x001A1A1A;
pub const OK_U32: u32 = 0x0022C55E;
pub const FAIL_U32: u32 = 0x00EF4444;
pub const WARN_U32: u32 = 0x00F59E0B;
pub const MUTED_U32: u32 = 0x006B7280;

const ACCENT: Color = Color::from_u32(ACCENT_U32);
const OK: Color = Color::from_u32(OK_U32);
const FAIL: Color = Color::from_u32(FAIL_U32);
const WARN: Color = Color::from_u32(WARN_U32);
const MUTED: Color = Color::from_u32(MUTED_U32);

fn selected_style() -> Style {
    Style::default().reversed()
}

// --- KPI strip (5 gauges) ---

pub fn kpi_strip(data: &TuiData) -> Paragraph<'static> {
    let k = &data.kpis;
    let cost_str = format!("{:.2}", k.daily_cost);
    let token_k = k.daily_tokens / 1000;

    let spans = vec![
        Span::styled(
            format!(" Plans:{} ", k.plans_active),
            Style::default().fg(ACCENT).bold(),
        ),
        Span::raw("| "),
        Span::styled(
            format!("Agents:{} ", k.agents_running),
            Style::default().fg(OK),
        ),
        Span::raw("| "),
        Span::styled(format!("Tokens:{}k ", token_k), Style::default().fg(WARN)),
        Span::raw("| "),
        Span::styled(format!("Cost:${} ", cost_str), Style::default().fg(WARN)),
        Span::raw("| "),
        Span::styled(
            format!("Mesh:{} ", k.mesh_online),
            Style::default().fg(ACCENT),
        ),
    ];
    Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(MUTED)),
    )
}

pub fn plan_kanban(data: &TuiData, selected: usize) -> Paragraph<'static> {
    let mut cols: BTreeMap<&str, Vec<(usize, String)>> = BTreeMap::new();
    for key in ["BLOCKED", "DOING", "DONE", "TODO"] {
        cols.insert(key, Vec::new());
    }
    for (i, plan) in data.plans.iter().enumerate() {
        let key = match plan.status.as_str() {
            "todo" => "TODO",
            "doing" => "DOING",
            "blocked" => "BLOCKED",
            "done" => "DONE",
            _ => "TODO",
        };
        let pct = if plan.tasks_total > 0 {
            (plan.tasks_done * 100) / plan.tasks_total
        } else {
            0
        };
        let bar = progress_bar(pct as u16, 12);
        cols.entry(key).or_default().push((
            i,
            format!("#{:<5} {} {:>3}% {}", plan.id, bar, pct, plan.name),
        ));
    }

    let mut lines: Vec<Line<'static>> = vec!["PLAN KANBAN".bold().fg(ACCENT).into(), "".into()];
    for col in ["TODO", "DOING", "BLOCKED", "DONE"] {
        let col_color = match col {
            "DONE" => OK,
            "DOING" => WARN,
            "BLOCKED" => FAIL,
            _ => MUTED,
        };
        lines.push(Line::from(format!("{}:", col)).style(Style::default().fg(col_color)));
        if let Some(items) = cols.get(col) {
            if items.is_empty() {
                lines.push("  -".fg(MUTED).into());
            } else {
                for (idx, text) in items {
                    let style = if *idx == selected {
                        selected_style()
                    } else {
                        Style::default()
                    };
                    lines.push(Line::from(format!("  {}", text)).style(style));
                }
            }
        }
        lines.push("".into());
    }

    Paragraph::new(Text::from(lines))
        .block(Block::default().title(" Plans ").borders(Borders::ALL))
        .wrap(Wrap { trim: true })
}

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

pub fn mesh_status(data: &TuiData, selected: usize) -> Paragraph<'static> {
    let online = data.mesh_nodes.iter().filter(|n| n.online).count();
    let mut lines: Vec<Line<'static>> = vec![
        "MESH STATUS".bold().fg(ACCENT).into(),
        Line::from(format!(
            "Online nodes: {}/{}",
            online,
            data.mesh_nodes.len()
        ))
        .style(Style::default().fg(OK)),
        "".into(),
    ];
    for (i, node) in data.mesh_nodes.iter().enumerate() {
        let (status, color) = if node.online {
            ("ONLINE", OK)
        } else {
            ("OFFLINE", FAIL)
        };
        let cpu_int = node.cpu_percent as i64;
        let cpu_bar = spark(cpu_int);
        let base = Style::default().fg(color);
        let style = if i == selected { base.reversed() } else { base };
        lines.push(
            Line::from(format!(
                "{:<16} {:<8} {:<10} cpu:{:<3}% {}",
                node.name, status, node.role, cpu_int, cpu_bar
            ))
            .style(style),
        );
    }
    if data.mesh_nodes.is_empty() {
        lines.push("No mesh peers found".fg(MUTED).into());
    }
    Paragraph::new(Text::from(lines))
        .block(Block::default().title(" Mesh ").borders(Borders::ALL))
        .wrap(Wrap { trim: true })
}

pub fn agent_org_chart(data: &TuiData, selected: usize) -> Paragraph<'static> {
    let mut lines: Vec<Line<'static>> = vec![
        "AGENT ORG CHART".bold().fg(ACCENT).into(),
        "ControlRoom".fg(WARN).into(),
    ];
    for (i, agent) in data.agents.iter().enumerate() {
        let branch = if i + 1 == data.agents.len() {
            "└──"
        } else {
            "├──"
        };
        let task = agent
            .active_task
            .clone()
            .unwrap_or_else(|| "idle".to_string());
        let task_color = if task == "idle" { MUTED } else { OK };
        let base = Style::default().fg(task_color);
        let style = if i == selected { base.reversed() } else { base };
        lines.push(
            Line::from(format!(
                "{} {} ({}) @{} [{}]",
                branch, agent.name, agent.role, agent.host, task
            ))
            .style(style),
        );
    }
    if data.agents.is_empty() {
        lines.push("└── no active agents".fg(MUTED).into());
    }
    Paragraph::new(Text::from(lines))
        .block(Block::default().title(" Agents ").borders(Borders::ALL))
        .wrap(Wrap { trim: true })
}

fn spark(cpu: i64) -> String {
    let levels = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
    let clamped = cpu.clamp(0, 100) as usize;
    let idx = clamped * (levels.len() - 1) / 100;
    levels[idx].repeat(6)
}

fn progress_bar(pct: u16, width: u16) -> String {
    let filled = ((pct as u32 * width as u32) / 100) as usize;
    let empty = width as usize - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}
