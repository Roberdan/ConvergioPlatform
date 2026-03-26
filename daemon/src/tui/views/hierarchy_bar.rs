// Hierarchy context bar — compact one-line strip shown above TaskPipeline when
// the user drilled into a sub-plan from the project tree.
// Shows: parent master name + sibling plan statuses + depends_on indicators.

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::data::PlanHierarchyContext;
use crate::tui::widgets::{ACCENT, FAIL, MUTED, OK, TEXT_PRIMARY, TEXT_SECONDARY, WARN};

fn status_icon(status: &str) -> &'static str {
    match status {
        "done" => "\u{2713}",      // ✓
        "doing" => "\u{25c9}",     // ◉
        "blocked" => "\u{2715}",   // ✕
        "cancelled" => "\u{2012}", // ‒
        _ => "\u{25cb}",           // ○
    }
}

fn status_color(status: &str) -> ratatui::style::Color {
    match status {
        "done" => OK,
        "doing" => WARN,
        "blocked" => FAIL,
        "cancelled" => MUTED,
        _ => TEXT_SECONDARY,
    }
}

/// Render the hierarchy context bar into `area` (expected height: 3).
pub fn render_hierarchy_bar(frame: &mut Frame<'_>, area: Rect, ctx: &PlanHierarchyContext) {
    let mut spans: Vec<Span<'static>> = vec![
        Span::styled(" \u{25c6} ".to_string(), Style::default().fg(ACCENT)),
        Span::styled(ctx.master_name.clone(), Style::default().fg(TEXT_PRIMARY).bold()),
        Span::styled("  \u{2502}  Siblings: ".to_string(), Style::default().fg(MUTED)),
    ];

    for (i, sibling) in ctx.siblings.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  \u{2502}  ".to_string(), Style::default().fg(MUTED)));
        }
        let icon = status_icon(&sibling.status);
        let color = status_color(&sibling.status);

        // Highlight current sibling with reversed style.
        let name_style = if sibling.is_current {
            Style::default().fg(ACCENT).bold().reversed()
        } else {
            Style::default().fg(color)
        };

        let label = format!(
            "{icon} {} [{}/{}]",
            sibling.name, sibling.tasks_done, sibling.tasks_total
        );
        spans.push(Span::styled(label, name_style));

        // Show depends_on arrow if present.
        if let Some(dep) = &sibling.depends_on {
            spans.push(Span::styled(
                format!(" \u{2190}{dep}"),
                Style::default().fg(MUTED),
            ));
        }
    }

    let paragraph = Paragraph::new(Line::from(spans))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(paragraph, area);
}
