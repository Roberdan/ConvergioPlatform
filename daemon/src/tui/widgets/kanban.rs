// Modern card-based kanban for the Plan Kanban view.
// Redesigned with rounded borders, color-gradient progress bars, and compact/expanded modes.

use std::collections::BTreeMap;

use ratatui::{
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::tui::TuiData;

use super::{selected_style, ACCENT, FAIL, MUTED, OK, TEXT_PRIMARY, WARN};

const SECTIONS: &[(&str, &str)] = &[
    ("DOING", "\u{25c9}"),  // ◉
    ("TODO", "\u{25cb}"),   // ○
    ("BLOCKED", "\u{2715}"),// ✕
    ("DONE", "\u{2713}"),   // ✓
];

/// Card width for border drawing (inner content width).
const CARD_W: usize = 66;

fn section_color(key: &str) -> ratatui::style::Color {
    match key {
        "DOING" => WARN,
        "DONE" => OK,
        "BLOCKED" => FAIL,
        _ => MUTED,
    }
}

fn progress_icon(pct: u16) -> &'static str {
    if pct >= 100 { "\u{2713}" } else if pct >= 50 { "\u{25d0}" } else if pct > 0 { "\u{25d4}" } else { "\u{25cb}" }
}

fn progress_color(pct: u16) -> ratatui::style::Color {
    if pct >= 80 { OK } else if pct >= 50 { WARN } else { FAIL }
}

fn truncate(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max { s.to_string() } else {
        chars[..max.saturating_sub(1)].iter().collect::<String>() + "\u{2026}"
    }
}

fn border_top() -> Line<'static> {
    let inner = "\u{2500}".repeat(CARD_W);
    Line::from(Span::styled(
        format!("  \u{256d}{inner}\u{256e}"), Style::default().fg(MUTED),
    ))
}

fn border_bottom() -> Line<'static> {
    let inner = "\u{2500}".repeat(CARD_W);
    Line::from(Span::styled(
        format!("  \u{2570}{inner}\u{256f}"), Style::default().fg(MUTED),
    ))
}

fn border_sep() -> Line<'static> {
    let inner = "\u{2500}".repeat(CARD_W);
    Line::from(Span::styled(
        format!("  \u{251c}{inner}\u{2524}"), Style::default().fg(MUTED),
    ))
}

/// Render the section header: " ◉ DOING                          3 plans"
fn section_header(icon: &str, label: &str, count: usize) -> Line<'static> {
    let col = section_color(label);
    let count_str = format!("{} plan{}", count, if count == 1 { "" } else { "s" });
    let pad = CARD_W.saturating_sub(icon.len() + 1 + label.len() + 1 + count_str.len()) + 2;
    Line::from(vec![
        Span::raw("  "),
        Span::styled(format!("{icon} "), Style::default().fg(col).bold()),
        Span::styled(label.to_string(), Style::default().fg(col).bold()),
        Span::raw(" ".repeat(pad)),
        Span::styled(count_str, Style::default().fg(MUTED)),
    ])
}

/// Expanded card for DOING: two lines (name + progress bar).
fn expanded_card(
    plan: &crate::tui::data::PlanCard, is_selected: bool, lines: &mut Vec<Line<'static>>,
) {
    let pct = if plan.tasks_total > 0 {
        ((plan.tasks_done * 100) / plan.tasks_total) as u16
    } else { 0 };
    let id_str = format!("{:>6}", format!("#{}", plan.id));
    let name = truncate(&plan.name, 48);
    // Row 1: "  │    #708 Plan Name                              │"
    let content_used = 2 + 6 + 1 + name.chars().count();
    let pad1 = CARD_W.saturating_sub(content_used);
    if is_selected {
        let inner = format!("  {} {}{}", id_str, name, " ".repeat(pad1));
        lines.push(Line::from(vec![
            Span::styled("  \u{2502}", Style::default().fg(MUTED)),
            Span::styled(inner, selected_style()),
            Span::styled("\u{2502}", Style::default().fg(MUTED)),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled("  \u{2502}  ", Style::default().fg(MUTED)),
            Span::styled(id_str, Style::default().fg(ACCENT).bold()),
            Span::raw(" "),
            Span::styled(name, Style::default().fg(TEXT_PRIMARY)),
            Span::raw(" ".repeat(pad1)),
            Span::styled("\u{2502}", Style::default().fg(MUTED)),
        ]));
    }
    // Row 2: "  │  ████████████░░░░░░░░  80%  12/18  ◐          │"
    let bar_w = 36;
    let filled = ((pct as usize) * bar_w / 100).min(bar_w);
    let empty = bar_w - filled;
    let bar_filled = "\u{2588}".repeat(filled);
    let bar_empty = "\u{2591}".repeat(empty);
    let pct_str = format!("{:>3}%", pct);
    let frac = format!("{}/{}", plan.tasks_done, plan.tasks_total);
    let icon = progress_icon(pct);
    let bar_color = progress_color(pct);
    // Calculate remaining space: 2(pad) + bar_w + 2 + pct(4) + 3 + frac + 2 + icon + 2
    let tail_len = 2 + bar_w + 2 + 4 + 3 + frac.len() + 2 + 1 + 2;
    let pad2 = CARD_W.saturating_sub(tail_len);
    if is_selected {
        let inner = format!(
            "  {}{} {} {}  {}  {}",
            bar_filled, bar_empty, pct_str, frac, icon, " ".repeat(pad2),
        );
        lines.push(Line::from(vec![
            Span::styled("  \u{2502}", Style::default().fg(MUTED)),
            Span::styled(inner, selected_style()),
            Span::styled("\u{2502}", Style::default().fg(MUTED)),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled("  \u{2502}  ", Style::default().fg(MUTED)),
            Span::styled(bar_filled, Style::default().fg(bar_color)),
            Span::styled(bar_empty, Style::default().fg(MUTED)),
            Span::raw(format!(" {} ", pct_str)),
            Span::raw(format!("{}  ", frac)),
            Span::styled(icon.to_string(), Style::default().fg(bar_color)),
            Span::raw(" ".repeat(pad2 + 2)),
            Span::styled("\u{2502}", Style::default().fg(MUTED)),
        ]));
    }
}

/// Compact card for TODO/DONE: single line with right-aligned fraction + icon.
fn compact_card(
    plan: &crate::tui::data::PlanCard, col_color: ratatui::style::Color,
    is_selected: bool, lines: &mut Vec<Line<'static>>,
) {
    let pct = if plan.tasks_total > 0 {
        ((plan.tasks_done * 100) / plan.tasks_total) as u16
    } else { 0 };
    let id_str = format!("{:>6}", format!("#{}", plan.id));
    let name = truncate(&plan.name, 40);
    let frac = format!("{}/{}", plan.tasks_done, plan.tasks_total);
    let icon = progress_icon(pct);
    // "  │    #705 name                        0/8   ○  │"
    let left_len = 2 + 6 + 1 + name.chars().count();
    let right_len = frac.len() + 3 + 1 + 2;
    let pad = CARD_W.saturating_sub(left_len + right_len);
    if is_selected {
        let inner = format!(
            "  {} {}{}{}   {}  ",
            id_str, name, " ".repeat(pad), frac, icon,
        );
        lines.push(Line::from(vec![
            Span::styled("  \u{2502}", Style::default().fg(MUTED)),
            Span::styled(inner, selected_style()),
            Span::styled("\u{2502}", Style::default().fg(MUTED)),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled("  \u{2502}  ", Style::default().fg(MUTED)),
            Span::styled(id_str, Style::default().fg(ACCENT).bold()),
            Span::raw(" "),
            Span::styled(name, Style::default().fg(TEXT_PRIMARY)),
            Span::raw(" ".repeat(pad)),
            Span::raw(format!("{}   ", frac)),
            Span::styled(icon.to_string(), Style::default().fg(col_color)),
            Span::raw("  "),
            Span::styled("\u{2502}", Style::default().fg(MUTED)),
        ]));
    }
}

pub fn plan_kanban(data: &TuiData, selected: usize, show_all: bool) -> Paragraph<'static> {
    let mut cols: BTreeMap<&str, Vec<(usize, &crate::tui::data::PlanCard)>> = BTreeMap::new();
    for key in ["BLOCKED", "DOING", "DONE", "TODO"] {
        cols.insert(key, Vec::new());
    }
    for (i, plan) in data.plans.iter().enumerate() {
        let key = match plan.status.as_str() {
            "todo" => "TODO", "doing" => "DOING", "blocked" => "BLOCKED",
            "done" => "DONE", _ => "TODO",
        };
        cols.entry(key).or_default().push((i, plan));
    }

    let mut lines: Vec<Line<'static>> = vec!["".into()];

    // Show/hide toggle hint
    let hint = if show_all { "[a] Show active only" } else { "[a] Show all plans" };
    lines.push(Line::from(Span::styled(format!("  {hint}"), Style::default().fg(MUTED))));
    lines.push("".into());

    for (key, icon) in SECTIONS {
        let all_items = cols.get(key).map(|v| v.as_slice()).unwrap_or(&[]);
        // Limit DONE/TODO to 10 unless show_all
        let max = if !show_all && (*key == "DONE" || *key == "TODO") { 10 } else { usize::MAX };
        let items = &all_items[..all_items.len().min(max)];
        let hidden = all_items.len().saturating_sub(max);
        let col_color = section_color(key);
        lines.push(section_header(icon, key, all_items.len()));

        if items.is_empty() {
            lines.push("".into());
            continue;
        }

        let is_expanded = *key == "DOING" || *key == "BLOCKED";
        if is_expanded {
            // Each card gets its own rounded border box.
            for (_li, (gi, plan)) in items.iter().enumerate() {
                let is_sel = *gi == selected;
                lines.push(border_top());
                expanded_card(plan, is_sel, &mut lines);
                lines.push(border_bottom());
            }
        } else {
            // All compact cards share one border box.
            lines.push(border_top());
            for (li, (gi, plan)) in items.iter().enumerate() {
                let is_sel = *gi == selected;
                compact_card(plan, col_color, is_sel, &mut lines);
                if li + 1 < items.len() {
                    lines.push(border_sep());
                }
            }
            lines.push(border_bottom());
        }
        if hidden > 0 {
            lines.push(Line::from(Span::styled(
                format!("  ... and {hidden} more (press [a] to show all)"),
                Style::default().fg(MUTED),
            )));
        }
        lines.push("".into());
    }

    Paragraph::new(Text::from(lines))
        .block(Block::default().title(" Plans ").borders(Borders::ALL))
        .wrap(Wrap { trim: false })
}
