// Workspace View — lists active worktrees with branch/plan/status columns.

use ratatui::{
    style::{Style, Stylize},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::tui::{
    widgets::{selected_style, ACCENT, FAIL, MUTED, OK, TEXT_SECONDARY},
    TuiData, WorkspaceInfo,
};

/// Map workspace status to a display color.
fn status_color(status: &str) -> ratatui::style::Color {
    match status {
        "active" => OK,
        "merged" => MUTED,
        "deleted" => FAIL,
        _ => TEXT_SECONDARY,
    }
}

/// Truncate a string to max_len chars; append "…" if truncated.
fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(1)).collect();
        format!("{truncated}…")
    }
}

/// Format a workspace row: id (20), branch (20), plan_id (8), status (10).
fn format_workspace_row(ws: &WorkspaceInfo) -> String {
    let id = truncate(&ws.workspace_id, 20);
    let branch = truncate(&ws.branch, 20);
    let plan = ws
        .plan_id
        .map(|p| p.to_string())
        .unwrap_or_else(|| "-".to_string());
    format!("{:<20} {:<20} {:>8}  {}", id, branch, plan, ws.status)
}

/// Render the Workspace View. Returns a `Paragraph` ready for frame rendering.
pub fn workspace_view(data: &TuiData, selected: usize) -> Paragraph<'static> {
    let workspaces = &data.workspaces;

    if workspaces.is_empty() {
        return Paragraph::new("No active workspaces")
            .block(Block::default().title(" Workspaces ").borders(Borders::ALL))
            .style(Style::default().fg(MUTED));
    }

    let mut lines: Vec<Line<'static>> = Vec::new();

    // Header
    lines.push(Line::from(vec![
        "WORKSPACES".bold().fg(ACCENT),
        "  ".into(),
        format!("({})", workspaces.len())
            .fg(TEXT_SECONDARY),
    ]));

    // Column headers
    lines.push(
        Line::from(format!(
            "{:<20} {:<20} {:>8}  {}",
            "Workspace ID", "Branch", "Plan", "Status"
        ))
        .style(Style::default().fg(TEXT_SECONDARY)),
    );
    lines.push(Line::from("─".repeat(64)).style(Style::default().fg(MUTED)));

    // Workspace rows
    for (i, ws) in workspaces.iter().enumerate() {
        let color = status_color(&ws.status);
        let row_text = format_workspace_row(ws);

        let style = if i == selected {
            selected_style()
        } else {
            Style::default().fg(color)
        };

        lines.push(Line::from(row_text).style(style));
    }

    Paragraph::new(Text::from(lines))
        .block(Block::default().title(" Workspaces ").borders(Borders::ALL))
        .wrap(Wrap { trim: true })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::{TuiData, WorkspaceInfo};

    fn sample_workspaces() -> Vec<WorkspaceInfo> {
        vec![
            WorkspaceInfo {
                workspace_id: "ws-123".to_string(),
                path: "/tmp/ws1".to_string(),
                branch: "plan-708-W1".to_string(),
                plan_id: Some(708),
                status: "active".to_string(),
                created_at: "2026-03-23T00:00:00Z".to_string(),
            },
            WorkspaceInfo {
                workspace_id: "ws-456".to_string(),
                path: "/tmp/ws2".to_string(),
                branch: "plan-700-W1".to_string(),
                plan_id: Some(700),
                status: "merged".to_string(),
                created_at: "2026-03-20T00:00:00Z".to_string(),
            },
            WorkspaceInfo {
                workspace_id: "ws-789".to_string(),
                path: "/tmp/ws3".to_string(),
                branch: "plan-600-W1".to_string(),
                plan_id: None,
                status: "deleted".to_string(),
                created_at: "2026-03-15T00:00:00Z".to_string(),
            },
        ]
    }

    #[test]
    fn workspace_view_shows_empty_state() {
        let data = TuiData::default();
        let p = workspace_view(&data, 0);
        let debug = format!("{p:?}");
        assert!(debug.contains("No active workspaces"), "Missing empty state");
    }

    #[test]
    fn workspace_view_contains_header() {
        let data = TuiData {
            workspaces: sample_workspaces(),
            ..TuiData::default()
        };
        let p = workspace_view(&data, 0);
        let debug = format!("{p:?}");
        assert!(debug.contains("WORKSPACES"), "Missing WORKSPACES header");
    }

    #[test]
    fn workspace_view_contains_workspace_data() {
        let data = TuiData {
            workspaces: sample_workspaces(),
            ..TuiData::default()
        };
        let p = workspace_view(&data, 0);
        let debug = format!("{p:?}");
        assert!(debug.contains("ws-123"), "Missing workspace id");
        assert!(debug.contains("plan-708-W1"), "Missing branch");
        assert!(debug.contains("active"), "Missing status");
    }

    #[test]
    fn workspace_view_shows_count() {
        let data = TuiData {
            workspaces: sample_workspaces(),
            ..TuiData::default()
        };
        let p = workspace_view(&data, 0);
        let debug = format!("{p:?}");
        assert!(debug.contains("3"), "Missing count");
    }

    #[test]
    fn truncate_leaves_short_strings_unchanged() {
        assert_eq!(truncate("ws-123", 20), "ws-123");
    }

    #[test]
    fn truncate_shortens_long_strings() {
        let long = "workspace-with-very-long-id-that-exceeds-limit";
        let result = truncate(long, 20);
        assert!(result.chars().count() <= 20, "truncate exceeded max_len");
        assert!(result.ends_with('…'), "truncate missing ellipsis");
    }

    #[test]
    fn format_workspace_row_uses_dash_for_missing_plan() {
        let ws = WorkspaceInfo {
            workspace_id: "ws-no-plan".to_string(),
            path: "/tmp".to_string(),
            branch: "main".to_string(),
            plan_id: None,
            status: "active".to_string(),
            created_at: "2026-03-23T00:00:00Z".to_string(),
        };
        let row = format_workspace_row(&ws);
        assert!(row.contains('-'), "Missing dash for null plan_id");
    }
}
