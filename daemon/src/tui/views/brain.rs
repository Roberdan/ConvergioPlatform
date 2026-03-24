// Brain ASCII canvas view — renders session/agent/task hierarchy from TuiData.brain_nodes.
// brain_nodes is populated by T2-03 (WS client + fetch_brain); this view is display-only.

use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use super::super::data::TuiData;
use super::super::widgets;

/// Render the Brain Canvas view as a Paragraph (ratatui 0.30 has no Canvas widget).
/// Sections: header, summary, SESSIONS, AGENTS, TASKS, token footer.
pub fn brain_canvas(data: &TuiData, selected: usize) -> Paragraph<'static> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    // ── Header ──────────────────────────────────────────────────────────────
    lines.push(Line::from(Span::styled(
        "  BRAIN CANVAS",
        Style::default().fg(widgets::ACCENT).bold(),
    )));
    lines.push(Line::raw(""));

    if data.brain_nodes.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No brain data — waiting for WebSocket connection",
            Style::default().fg(widgets::MUTED),
        )));
        lines.push(Line::raw(""));
        push_token_footer(&mut lines, data);
        return build_paragraph(lines);
    }

    // ── Node counts ──────────────────────────────────────────────────────────
    let sessions: Vec<_> = data
        .brain_nodes
        .iter()
        .filter(|n| n.kind == "session")
        .collect();
    let agents: Vec<_> = data
        .brain_nodes
        .iter()
        .filter(|n| n.kind == "agent")
        .collect();
    let tasks: Vec<_> = data
        .brain_nodes
        .iter()
        .filter(|n| n.kind == "task")
        .collect();

    lines.push(Line::from(format!(
        "  Sessions: {} | Agents: {} | Tasks: {}",
        sessions.len(),
        agents.len(),
        tasks.len()
    )));
    lines.push(Line::raw(""));

    // ── SESSIONS section ─────────────────────────────────────────────────────
    lines.push(Line::from(Span::styled(
        "  SESSIONS",
        Style::default().fg(widgets::TEXT_SECONDARY).bold(),
    )));

    // Track global node index for selection highlight
    let mut node_idx: usize = 0;

    for node in &sessions {
        let color = status_color(&node.status);
        let label = format!("  ● {}", node.label);
        let style = node_style(color, node_idx == selected);
        lines.push(Line::from(Span::styled(label, style)));
        node_idx += 1;
    }
    lines.push(Line::raw(""));

    // ── AGENTS section ───────────────────────────────────────────────────────
    lines.push(Line::from(Span::styled(
        "  AGENTS",
        Style::default().fg(widgets::TEXT_SECONDARY).bold(),
    )));

    for node in &agents {
        let color = status_color(&node.status);
        let prefix = if node.parent_id.is_some() {
            "  └── "
        } else {
            "  ○ "
        };
        let label = format!("{}{}", prefix, node.label);
        let style = node_style(color, node_idx == selected);
        lines.push(Line::from(Span::styled(label, style)));
        node_idx += 1;
    }
    lines.push(Line::raw(""));

    // ── TASKS section ────────────────────────────────────────────────────────
    lines.push(Line::from(Span::styled(
        "  TASKS",
        Style::default().fg(widgets::TEXT_SECONDARY).bold(),
    )));

    for node in &tasks {
        let color = status_color(&node.status);
        let prefix = if node.parent_id.is_some() {
            "      └── "
        } else {
            "  ▸ "
        };
        let label = format!("{}{}", prefix, node.label);
        let style = node_style(color, node_idx == selected);
        lines.push(Line::from(Span::styled(label, style)));
        node_idx += 1;
    }
    lines.push(Line::raw(""));

    push_token_footer(&mut lines, data);
    build_paragraph(lines)
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn status_color(status: &str) -> ratatui::style::Color {
    match status {
        "running" => widgets::OK,
        "submitted" => widgets::WARN,
        "completed" | "done" => widgets::MUTED,
        "failed" => widgets::FAIL,
        _ => widgets::TEXT_SECONDARY,
    }
}

fn node_style(color: ratatui::style::Color, is_selected: bool) -> Style {
    let base = Style::default().fg(color);
    if is_selected {
        base.reversed()
    } else {
        base
    }
}

fn push_token_footer(lines: &mut Vec<Line<'static>>, data: &TuiData) {
    let daily_k = data.kpis.daily_tokens / 1000;
    let cost = data.kpis.daily_cost;
    lines.push(Line::from(Span::styled(
        format!("  Today: {}k tokens | ${:.2}", daily_k, cost),
        Style::default().fg(widgets::MUTED),
    )));
}

fn build_paragraph(lines: Vec<Line<'static>>) -> Paragraph<'static> {
    Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Brain Canvas"),
        )
        .style(Style::default().fg(widgets::TEXT_PRIMARY))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::{BrainNode, TuiData};

    #[test]
    fn brain_canvas_shows_empty_state_when_no_nodes() {
        let data = TuiData::default();
        let p = brain_canvas(&data, 0);
        let debug = format!("{p:?}");
        assert!(debug.contains("No brain data"), "Missing empty-state text");
    }

    #[test]
    fn brain_canvas_contains_header_with_nodes() {
        let data = TuiData {
            brain_nodes: vec![BrainNode {
                id: "s1".to_string(),
                label: "session-alpha".to_string(),
                kind: "session".to_string(),
                parent_id: None,
                status: "running".to_string(),
            }],
            ..TuiData::default()
        };
        let p = brain_canvas(&data, 0);
        let debug = format!("{p:?}");
        assert!(debug.contains("BRAIN CANVAS"), "Missing BRAIN CANVAS header");
        assert!(debug.contains("session-alpha"), "Missing node label");
    }

    #[test]
    fn brain_canvas_counts_nodes_by_kind() {
        let data = TuiData {
            brain_nodes: vec![
                BrainNode { id: "s1".to_string(), label: "sess".to_string(), kind: "session".to_string(), parent_id: None, status: "running".to_string() },
                BrainNode { id: "a1".to_string(), label: "agent-thor".to_string(), kind: "agent".to_string(), parent_id: Some("s1".to_string()), status: "idle".to_string() },
                BrainNode { id: "t1".to_string(), label: "task-9120".to_string(), kind: "task".to_string(), parent_id: Some("a1".to_string()), status: "submitted".to_string() },
            ],
            ..TuiData::default()
        };
        let p = brain_canvas(&data, 0);
        let debug = format!("{p:?}");
        assert!(debug.contains("Sessions: 1"), "Wrong session count");
        assert!(debug.contains("Agents: 1"), "Wrong agent count");
        assert!(debug.contains("Tasks: 1"), "Wrong task count");
    }
}
