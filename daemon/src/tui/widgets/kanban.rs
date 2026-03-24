// Modern card-based kanban for the Plan Kanban view.
// Extracted from shared.rs to keep file sizes under 250 lines.

use std::collections::BTreeMap;

use ratatui::{
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::tui::TuiData;

use super::{selected_style, ACCENT, FAIL, MUTED, OK, TEXT_PRIMARY, WARN};

// Section display order and metadata.
const SECTIONS: &[(&str, &str, &str)] = &[
    ("DOING", "◉", "DOING"),
    ("TODO", "○", "TODO"),
    ("BLOCKED", "✕", "BLOCKED"),
    ("DONE", "✓", "DONE"),
];

fn section_color(key: &str) -> ratatui::style::Color {
    match key {
        "DOING" => WARN,
        "DONE" => OK,
        "BLOCKED" => FAIL,
        _ => MUTED,
    }
}

/// Progress icon: ✓ >=100%, ◐ >=50%, ◔ >0%, ○ 0%
fn progress_icon(pct: u16) -> &'static str {
    if pct >= 100 { "✓" } else if pct >= 50 { "◐" } else if pct > 0 { "◔" } else { "○" }
}

/// Colored progress bar string using block chars. Width = number of filled+empty chars.
fn colored_bar(pct: u16, width: usize) -> (String, String, ratatui::style::Color) {
    let color = if pct >= 80 { OK } else if pct >= 50 { WARN } else { FAIL };
    let filled = ((pct as usize) * width / 100).min(width);
    let empty = width - filled;
    ("█".repeat(filled), "░".repeat(empty), color)
}

/// Truncate a string to max_chars, appending … if truncated.
fn truncate(s: &str, max_chars: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_chars {
        s.to_string()
    } else {
        chars[..max_chars.saturating_sub(1)].iter().collect::<String>() + "…"
    }
}

pub fn plan_kanban(data: &TuiData, selected: usize) -> Paragraph<'static> {
    // Bucket plans by status key.
    let mut cols: BTreeMap<&str, Vec<(usize, &crate::tui::data::PlanCard)>> = BTreeMap::new();
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
        cols.entry(key).or_default().push((i, plan));
    }

    let mut lines: Vec<Line<'static>> = vec!["".into()];

    for (key, icon, label) in SECTIONS {
        let items = cols.get(key).map(|v| v.as_slice()).unwrap_or(&[]);
        let count = items.len();
        let col_color = section_color(key);

        // Section header: "  ◉ DOING (3)"
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{} ", icon), Style::default().fg(col_color).bold()),
            Span::styled(format!("{} ", label), Style::default().fg(col_color).bold()),
            Span::styled(format!("({})", count), Style::default().fg(MUTED)),
        ]));

        if items.is_empty() {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("(none)", Style::default().fg(MUTED)),
            ]));
            lines.push("".into());
            continue;
        }

        // Top border of card block.
        lines.push(Line::from(Span::styled(
            "  ┌────────────────────────────────────────────────────────────┐",
            Style::default().fg(MUTED),
        )));

        for (list_idx, (global_idx, plan)) in items.iter().enumerate() {
            let is_selected = *global_idx == selected;
            let pct = if plan.tasks_total > 0 {
                ((plan.tasks_done * 100) / plan.tasks_total) as u16
            } else {
                0
            };
            let name = truncate(&plan.name, 46);
            let id_str = format!("#{:<5}", plan.id);

            // First line: id + name
            let name_pad = 55usize.saturating_sub(id_str.len() + 2 + name.chars().count());
            let row1_text = format!(" {} {}{}│", id_str, name, " ".repeat(name_pad));
            let row1 = if is_selected {
                Line::from(vec![
                    Span::raw("  │"),
                    Span::styled(row1_text, selected_style()),
                ])
            } else {
                Line::from(vec![
                    Span::styled("  │", Style::default().fg(MUTED)),
                    Span::styled(
                        format!(" {} ", id_str),
                        Style::default().fg(ACCENT).bold(),
                    ),
                    Span::styled(name.clone(), Style::default().fg(TEXT_PRIMARY)),
                    Span::raw(" ".repeat(name_pad)),
                    Span::styled("│", Style::default().fg(MUTED)),
                ])
            };
            lines.push(row1);

            // Second line for DOING/BLOCKED: progress bar + fraction + icon.
            // For DONE/TODO compact: just fraction + icon.
            if *key == "DOING" || *key == "BLOCKED" {
                let icon_str = progress_icon(pct);
                let fraction = format!("{}/{}", plan.tasks_done, plan.tasks_total);
                let (filled, empty, bar_color) = colored_bar(pct, 20);
                let pct_str = format!("{:>3}%", pct);
                let tail = format!(
                    " {} {}  {}  ",
                    pct_str, icon_str, fraction
                );
                let tail_pad = 55usize
                    .saturating_sub(1 + filled.chars().count() + empty.chars().count() + tail.chars().count());

                let row2 = if is_selected {
                    Line::from(vec![
                        Span::raw("  │"),
                        Span::styled(
                            format!(" {}{}{}{} │",
                                filled, empty, tail, " ".repeat(tail_pad)),
                            selected_style(),
                        ),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled("  │ ", Style::default().fg(MUTED)),
                        Span::styled(filled, Style::default().fg(bar_color)),
                        Span::styled(empty, Style::default().fg(MUTED)),
                        Span::raw(tail),
                        Span::raw(" ".repeat(tail_pad)),
                        Span::styled("│", Style::default().fg(MUTED)),
                    ])
                };
                lines.push(row2);
            } else {
                // Compact: just fraction + icon, right-aligned.
                let icon_str = progress_icon(pct);
                let fraction = format!("{}/{}", plan.tasks_done, plan.tasks_total);
                let compact = format!(" {} {}  ", icon_str, fraction);
                let pad = 55usize.saturating_sub(compact.chars().count());
                let row2 = if is_selected {
                    Line::from(vec![
                        Span::raw("  │"),
                        Span::styled(format!("{}{} │", " ".repeat(pad), compact), selected_style()),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled("  │", Style::default().fg(MUTED)),
                        Span::raw(" ".repeat(pad)),
                        Span::styled(icon_str, Style::default().fg(col_color)),
                        Span::raw(format!(" {}  ", fraction)),
                        Span::styled("│", Style::default().fg(MUTED)),
                    ])
                };
                lines.push(row2);
            }

            // Separator between cards (not after the last one).
            let is_last = list_idx + 1 == items.len();
            if !is_last {
                lines.push(Line::from(Span::styled(
                    "  ├────────────────────────────────────────────────────────────┤",
                    Style::default().fg(MUTED),
                )));
            }
        }

        // Bottom border.
        lines.push(Line::from(Span::styled(
            "  └────────────────────────────────────────────────────────────┘",
            Style::default().fg(MUTED),
        )));
        lines.push("".into());
    }

    Paragraph::new(Text::from(lines))
        .block(Block::default().title(" Plans ").borders(Borders::ALL))
        .wrap(Wrap { trim: false })
}
