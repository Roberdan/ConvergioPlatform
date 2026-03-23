// Event Stream view — workspace activity feed with action-based coloring.

use ratatui::{
    style::{Style, Stylize},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::tui::{
    widgets::{selected_style, ACCENT, FAIL, MUTED, OK, TEXT_SECONDARY, WARN},
    TuiData, WorkspaceEvent,
};

/// Map an action string to a display color.
fn action_color(action: &str) -> ratatui::style::Color {
    match action {
        "file_write" | "file_edit" => WARN,
        "git_commit" | "git_push" => OK,
        "quality_gate_fail" => FAIL,
        "pr_created" | "pr_merged" => ACCENT,
        _ => MUTED,
    }
}

/// Extract HH:MM:SS from an ISO-8601 timestamp (last 8 chars before 'Z' or of string).
fn timestamp_display(created_at: &str) -> &str {
    let s = created_at.trim_end_matches('Z');
    if s.len() >= 8 {
        &s[s.len() - 8..]
    } else {
        s
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

/// Render a single event row as a formatted string.
fn format_event_row(ev: &WorkspaceEvent) -> String {
    let ts = timestamp_display(&ev.created_at);
    let fp = ev
        .file_path
        .as_deref()
        .map(|p| truncate(p, 30))
        .unwrap_or_else(|| "-".to_string());
    let detail = ev.detail.as_deref().unwrap_or("-");
    format!("{ts}  {:<20} {:<18} {:<31} {}", ev.agent, ev.action, fp, detail)
}

/// Render the Event Stream view. Returns a `Paragraph` ready for frame rendering.
pub fn event_stream(data: &TuiData, selected: usize) -> Paragraph<'static> {
    let events = &data.events;

    if events.is_empty() {
        return Paragraph::new("No events — waiting for workspace activity")
            .block(Block::default().title(" Event Stream ").borders(Borders::ALL))
            .style(Style::default().fg(MUTED));
    }

    let mut lines: Vec<Line<'static>> = Vec::new();

    // Header
    lines.push(
        Line::from(vec![
            "EVENTS".bold().fg(ACCENT),
            format!("  Showing {} events", events.len())
                .fg(TEXT_SECONDARY)
                .into(),
        ]),
    );

    // Column headers
    lines.push(
        Line::from(format!(
            "{:<8}  {:<20} {:<18} {:<31} {}",
            "Time", "Agent", "Action", "File", "Detail"
        ))
        .style(Style::default().fg(TEXT_SECONDARY)),
    );
    lines.push(Line::from("─".repeat(100)).style(Style::default().fg(MUTED)));

    // Event rows
    for (i, ev) in events.iter().enumerate() {
        let color = action_color(&ev.action);
        let row_text = format_event_row(ev);

        let style = if i == selected {
            selected_style()
        } else {
            Style::default().fg(color)
        };

        lines.push(Line::from(row_text).style(style));
    }

    Paragraph::new(Text::from(lines))
        .block(Block::default().title(" Event Stream ").borders(Borders::ALL))
        .wrap(Wrap { trim: true })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::{TuiData, WorkspaceEvent};

    fn sample_events() -> Vec<WorkspaceEvent> {
        vec![
            WorkspaceEvent {
                id: 1,
                workspace_id: "ws-1".to_string(),
                agent: "executor".to_string(),
                action: "file_write".to_string(),
                file_path: Some("daemon/src/tui/views/events.rs".to_string()),
                detail: None,
                created_at: "2026-03-23T10:44:15Z".to_string(),
            },
            WorkspaceEvent {
                id: 2,
                workspace_id: "ws-1".to_string(),
                agent: "thor".to_string(),
                action: "git_commit".to_string(),
                file_path: None,
                detail: Some("feat: add events view".to_string()),
                created_at: "2026-03-23T10:45:00Z".to_string(),
            },
        ]
    }

    #[test]
    fn event_stream_contains_header() {
        let data = TuiData {
            events: sample_events(),
            ..TuiData::default()
        };
        let p = event_stream(&data, 0);
        let debug = format!("{p:?}");
        assert!(debug.contains("EVENTS"), "Missing EVENTS header");
    }

    #[test]
    fn event_stream_contains_event_data() {
        let data = TuiData {
            events: sample_events(),
            ..TuiData::default()
        };
        let p = event_stream(&data, 0);
        let debug = format!("{p:?}");
        assert!(debug.contains("executor"), "Missing agent name");
        assert!(debug.contains("file_write"), "Missing action");
    }

    #[test]
    fn event_stream_shows_empty_state() {
        let data = TuiData::default();
        let p = event_stream(&data, 0);
        let debug = format!("{p:?}");
        assert!(
            debug.contains("No events"),
            "Missing empty-state message"
        );
    }

    #[test]
    fn timestamp_display_extracts_hhmmss() {
        assert_eq!(timestamp_display("2026-03-23T10:44:15Z"), "10:44:15");
        assert_eq!(timestamp_display("2026-03-23T10:44:15"), "10:44:15");
    }

    #[test]
    fn truncate_leaves_short_strings_unchanged() {
        assert_eq!(truncate("src/foo.rs", 30), "src/foo.rs");
    }

    #[test]
    fn truncate_shortens_long_strings() {
        let long = "daemon/src/tui/views/events_long_path.rs";
        let result = truncate(long, 30);
        assert!(result.chars().count() <= 30, "truncate exceeded max_len");
        assert!(result.ends_with('…'), "truncate missing ellipsis");
    }
}
