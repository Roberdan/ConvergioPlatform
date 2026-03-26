// Project hierarchy tree view — replaces flat kanban with master/child structure.

use ratatui::{
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::tui::data::{ProjectTreeData, ProjectTreeNode, TuiData};
use crate::tui::widgets::{selected_style, ACCENT, FAIL, MUTED, OK, TEXT_PRIMARY, TEXT_SECONDARY, WARN};

fn status_color(status: &str) -> ratatui::style::Color {
    match status {
        "done" => OK,
        "doing" => WARN,
        "blocked" => FAIL,
        "cancelled" => MUTED,
        _ => TEXT_SECONDARY, // draft, todo
    }
}

fn status_icon(status: &str) -> &'static str {
    match status {
        "done" => "\u{2713}",     // ✓
        "doing" => "\u{25c9}",    // ◉
        "blocked" => "\u{2715}",  // ✕
        "cancelled" => "\u{2012}",// ‒
        _ => "\u{25cb}",          // ○
    }
}

fn progress_bar(done: i64, total: i64, width: usize) -> Vec<Span<'static>> {
    if total == 0 {
        return vec![Span::styled(
            format!("[{}]", "\u{2591}".repeat(width)),
            Style::default().fg(MUTED),
        )];
    }
    let pct = ((done * 100) / total) as u16;
    let filled = ((pct as usize) * width / 100).min(width);
    let empty = width - filled;
    let color = if pct >= 80 { OK } else if pct >= 50 { WARN } else { FAIL };
    vec![
        Span::raw("["),
        Span::styled("\u{2588}".repeat(filled), Style::default().fg(color)),
        Span::styled("\u{2591}".repeat(empty), Style::default().fg(MUTED)),
        Span::raw("]"),
    ]
}

fn truncate(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        s.to_string()
    } else {
        chars[..max.saturating_sub(1)].iter().collect::<String>() + "\u{2026}"
    }
}

/// Compute aggregate progress from children (fallback to own if no children).
fn aggregate(node: &ProjectTreeNode) -> (i64, i64) {
    if node.children.is_empty() {
        return (node.tasks_done, node.tasks_total);
    }
    node.children
        .iter()
        .fold((0, 0), |(d, t), c| (d + c.tasks_done, t + c.tasks_total))
}

fn depends_label(dep: &Option<String>) -> String {
    match dep {
        // → depends on: <name>
        Some(d) => format!(" \u{2192} depends on: {d}"),
        None => String::new(),
    }
}

/// Returns colored badge spans for the execution mode. Empty if no mode set.
/// sequential=MUTED (single-threaded), parallel=OK (fast), mixed=WARN, conditional=ACCENT.
pub fn mode_badge_spans(mode: &Option<String>) -> Vec<Span<'static>> {
    let (label, color) = match mode.as_deref() {
        Some("sequential") => ("[SEQ]", MUTED),
        Some("parallel")   => ("[PAR]", OK),
        Some("mixed")      => ("[MIX]", WARN),
        Some("conditional") => ("[CND]", ACCENT),
        Some(m) if !m.is_empty() => return vec![
            Span::raw(" "),
            Span::styled(format!("[{m}]"), Style::default().fg(MUTED)),
        ],
        _ => return vec![],
    };
    vec![
        Span::raw(" "),
        Span::styled(label, Style::default().fg(color)),
    ]
}

/// Flatten tree into renderable lines. Returns (lines, total_selectable_items).
pub fn build_tree_lines(
    tree: &ProjectTreeData,
    selected: usize,
    expanded: &[i64],
) -> (Vec<Line<'static>>, usize) {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut idx: usize = 0;

    // Header
    let pct = if tree.total_tasks > 0 {
        (tree.done_tasks * 100 / tree.total_tasks) as u16
    } else {
        0
    };
    let header = format!(
        "  {} \u{2014} {}/{} tasks ({pct}%)",
        tree.project_name, tree.done_tasks, tree.total_tasks,
    );
    lines.push(Line::from(Span::styled(header, Style::default().fg(ACCENT).bold())));
    lines.push("".into());

    let (masters, orphans): (Vec<_>, Vec<_>) =
        tree.plans.iter().partition(|p| p.is_master);

    for master in &masters {
        let is_expanded = expanded.contains(&master.id);
        let (agg_done, agg_total) = aggregate(master);
        let toggle = if is_expanded { "\u{25bc}" } else { "\u{25b6}" };
        let is_sel = idx == selected;
        idx += 1;

        let name = truncate(&master.name, 50);
        let badge = mode_badge_spans(&master.execution_mode);
        let frac = format!("{agg_done}/{agg_total}");

        let name_style = if is_sel { selected_style() } else { Style::default().fg(TEXT_PRIMARY).bold() };
        let mut spans = vec![
            Span::styled(format!(" {toggle} "), Style::default().fg(ACCENT)),
            Span::styled(name, name_style),
        ];
        spans.extend(badge);
        spans.push(Span::raw("  "));
        spans.extend(progress_bar(agg_done, agg_total, 16));
        let pct = if agg_total > 0 { agg_done * 100 / agg_total } else { 0 };
        spans.push(Span::styled(format!(" {frac}"), Style::default().fg(TEXT_SECONDARY)));
        spans.push(Span::styled(format!(" ({pct}%)"), Style::default().fg(TEXT_SECONDARY)));
        lines.push(Line::from(spans));

        if is_expanded {
            let child_count = master.children.len();
            for (ci, child) in master.children.iter().enumerate() {
                let is_sel = idx == selected;
                let is_last = ci + 1 == child_count;
                idx += 1;
                render_child(&mut lines, child, is_sel, is_last);
            }
            // Rollup summary: aggregate progress across all children.
            lines.push(Line::from(vec![
                Span::styled("    Rollup: ", Style::default().fg(MUTED)),
                Span::styled(format!("{agg_done}/{agg_total} ({pct}%)"), Style::default().fg(ACCENT)),
            ]));
        }
        lines.push("".into());
    }

    if !orphans.is_empty() {
        lines.push(Line::from(Span::styled(
            " \u{2500}\u{2500} Other Plans \u{2500}\u{2500}",
            Style::default().fg(MUTED),
        )));
        let orphan_count = orphans.len();
        for (oi, orphan) in orphans.iter().enumerate() {
            let is_sel = idx == selected;
            let is_last = oi + 1 == orphan_count;
            idx += 1;
            render_child(&mut lines, orphan, is_sel, is_last);
        }
        lines.push("".into());
    }

    if tree.plans.is_empty() {
        lines.push(Line::from(Span::styled(
            " No project data available",
            Style::default().fg(MUTED),
        )));
    }

    (lines, idx)
}

fn render_child(lines: &mut Vec<Line<'static>>, node: &ProjectTreeNode, is_sel: bool, is_last: bool) {
    let icon = status_icon(&node.status);
    let color = status_color(&node.status);
    let name = truncate(&node.name, 44);
    let dep = depends_label(&node.depends_on);
    // [N/M] bracket notation as required
    let frac = format!("[{}/{}]", node.tasks_done, node.tasks_total);
    // ├── for non-last children, └── for the last child
    let branch = if is_last {
        "\u{2514}\u{2500}\u{2500}" // └──
    } else {
        "\u{251c}\u{2500}\u{2500}" // ├──
    };

    let name_style = if is_sel {
        selected_style()
    } else {
        Style::default().fg(TEXT_PRIMARY)
    };

    let mut spans = vec![
        Span::styled(format!(" {branch} {icon} "), Style::default().fg(color)),
        Span::styled(format!("#{:<4} ", node.id), Style::default().fg(ACCENT)),
        Span::styled(name, name_style),
    ];
    if !dep.is_empty() {
        spans.push(Span::styled(dep, Style::default().fg(MUTED)));
    }
    spans.push(Span::raw("  "));
    spans.push(Span::styled(frac, Style::default().fg(TEXT_SECONDARY)));
    lines.push(Line::from(spans));
}

/// Top-level widget: renders the project tree into a Paragraph.
pub fn project_tree_view(data: &TuiData, selected: usize, expanded: &[i64]) -> Paragraph<'static> {
    let (lines, _count) = build_tree_lines(&data.project_tree, selected, expanded);
    Paragraph::new(Text::from(lines))
        .block(Block::default().title(" Project Tree ").borders(Borders::ALL))
        .wrap(Wrap { trim: false })
}