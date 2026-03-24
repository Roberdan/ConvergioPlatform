// Deliverables Browser — lists deliverables with status/type/version columns.

use ratatui::{
    style::{Style, Stylize},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::tui::{
    widgets::{selected_style, ACCENT, FAIL, MUTED, OK, TEXT_SECONDARY, WARN},
    DeliverableInfo, TuiData,
};

/// Map deliverable status to a display color.
fn status_color(status: &str) -> ratatui::style::Color {
    match status {
        "approved" => OK,
        "pending" => WARN,
        "rejected" => FAIL,
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

/// Format a deliverable row: name (24), type (8), status (10), version (4), project (16).
fn format_deliverable_row(d: &DeliverableInfo) -> String {
    let name = truncate(&d.name, 24);
    let proj = truncate(&d.project_id, 16);
    format!(
        "{:<24} {:<8} {:<10} {:>4}  {}",
        name, d.output_type, d.status, d.version, proj
    )
}

/// Render the Deliverables view. Returns a `Paragraph` ready for frame rendering.
pub fn deliverables_view(data: &TuiData, selected: usize) -> Paragraph<'static> {
    let deliverables = &data.deliverables;

    if deliverables.is_empty() {
        return Paragraph::new("No deliverables")
            .block(Block::default().title(" Deliverables ").borders(Borders::ALL))
            .style(Style::default().fg(MUTED));
    }

    let mut lines: Vec<Line<'static>> = Vec::new();

    // Header
    lines.push(Line::from(vec![
        "DELIVERABLES".bold().fg(ACCENT),
        "  ".into(),
        format!("({})", deliverables.len())
            .fg(TEXT_SECONDARY),
    ]));

    // Status summary
    let pending = deliverables.iter().filter(|d| d.status == "pending").count();
    let approved = deliverables.iter().filter(|d| d.status == "approved").count();
    let rejected = deliverables.iter().filter(|d| d.status == "rejected").count();
    lines.push(
        Line::from(format!(
            "Pending: {pending} | Approved: {approved} | Rejected: {rejected}"
        ))
        .style(Style::default().fg(TEXT_SECONDARY)),
    );
    lines.push(Line::raw(""));

    // Column headers
    lines.push(
        Line::from(format!(
            "{:<24} {:<8} {:<10} {:>4}  {}",
            "Name", "Type", "Status", "Ver", "Project"
        ))
        .style(Style::default().fg(TEXT_SECONDARY)),
    );
    lines.push(Line::from("─".repeat(72)).style(Style::default().fg(MUTED)));

    // Deliverable rows
    for (i, d) in deliverables.iter().enumerate() {
        let color = status_color(&d.status);
        let row_text = format_deliverable_row(d);

        let style = if i == selected {
            selected_style()
        } else {
            Style::default().fg(color)
        };

        lines.push(Line::from(row_text).style(style));
    }

    Paragraph::new(Text::from(lines))
        .block(Block::default().title(" Deliverables ").borders(Borders::ALL))
        .wrap(Wrap { trim: true })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::{DeliverableInfo, TuiData};

    fn sample_deliverables() -> Vec<DeliverableInfo> {
        vec![
            DeliverableInfo {
                id: 1,
                name: "Q1 Report".to_string(),
                output_type: "doc".to_string(),
                status: "approved".to_string(),
                version: 2,
                project_id: "convergio".to_string(),
                created_at: "2026-03-20T00:00:00Z".to_string(),
            },
            DeliverableInfo {
                id: 2,
                name: "Design Spec".to_string(),
                output_type: "pdf".to_string(),
                status: "pending".to_string(),
                version: 1,
                project_id: "dashboard".to_string(),
                created_at: "2026-03-22T00:00:00Z".to_string(),
            },
            DeliverableInfo {
                id: 3,
                name: "Old Proposal".to_string(),
                output_type: "doc".to_string(),
                status: "rejected".to_string(),
                version: 1,
                project_id: "convergio".to_string(),
                created_at: "2026-03-15T00:00:00Z".to_string(),
            },
        ]
    }

    #[test]
    fn deliverables_view_shows_empty_state() {
        let data = TuiData::default();
        let p = deliverables_view(&data, 0);
        let debug = format!("{p:?}");
        assert!(debug.contains("No deliverables"), "Missing empty state");
    }

    #[test]
    fn deliverables_view_contains_header() {
        let data = TuiData {
            deliverables: sample_deliverables(),
            ..TuiData::default()
        };
        let p = deliverables_view(&data, 0);
        let debug = format!("{p:?}");
        assert!(debug.contains("DELIVERABLES"), "Missing DELIVERABLES header");
    }

    #[test]
    fn deliverables_view_contains_status_summary() {
        let data = TuiData {
            deliverables: sample_deliverables(),
            ..TuiData::default()
        };
        let p = deliverables_view(&data, 0);
        let debug = format!("{p:?}");
        assert!(debug.contains("Pending: 1"), "Missing pending count");
        assert!(debug.contains("Approved: 1"), "Missing approved count");
        assert!(debug.contains("Rejected: 1"), "Missing rejected count");
    }

    #[test]
    fn deliverables_view_contains_deliverable_data() {
        let data = TuiData {
            deliverables: sample_deliverables(),
            ..TuiData::default()
        };
        let p = deliverables_view(&data, 0);
        let debug = format!("{p:?}");
        assert!(debug.contains("Q1 Report"), "Missing deliverable name");
        assert!(debug.contains("approved"), "Missing status");
        assert!(debug.contains("convergio"), "Missing project");
    }

    #[test]
    fn deliverables_view_shows_count() {
        let data = TuiData {
            deliverables: sample_deliverables(),
            ..TuiData::default()
        };
        let p = deliverables_view(&data, 0);
        let debug = format!("{p:?}");
        assert!(debug.contains("3"), "Missing count");
    }

    #[test]
    fn truncate_leaves_short_strings_unchanged() {
        assert_eq!(truncate("Report", 24), "Report");
    }

    #[test]
    fn truncate_shortens_long_strings() {
        let long = "A Very Long Deliverable Name That Exceeds The Column Width";
        let result = truncate(long, 24);
        assert!(result.chars().count() <= 24, "truncate exceeded max_len");
        assert!(result.ends_with('…'), "truncate missing ellipsis");
    }
}
