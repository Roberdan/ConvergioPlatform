// Cost Center view — token/cost breakdown by model, project, and date.

use ratatui::{
    style::{Style, Stylize},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::tui::{
    widgets::{selected_style, ACCENT, MUTED, TEXT_PRIMARY, TEXT_SECONDARY, WARN},
    CostEntry, TuiData,
};

/// Render the Cost Center view. Returns a `Paragraph` ready for frame rendering.
pub fn cost_center(data: &TuiData, selected: usize) -> Paragraph<'static> {
    let cost = &data.cost;

    if cost.by_model.is_empty() && cost.by_project.is_empty() && cost.by_date.is_empty() {
        return Paragraph::new("No cost data available")
            .block(Block::default().title(" Cost Center ").borders(Borders::ALL))
            .style(Style::default().fg(MUTED));
    }

    let mut lines: Vec<Line<'static>> = Vec::new();

    // Header
    lines.push("COST CENTER".bold().fg(ACCENT).into());

    // Summary line
    let s = &cost.summary;
    lines.push(
        Line::from(format!(
            "Runs: {} | Avg: {:.0}s | Total: ${:.2}",
            s.run_count, s.avg_duration_secs, s.total_cost_usd
        ))
        .style(Style::default().fg(TEXT_SECONDARY)),
    );
    lines.push("".into());

    // BY MODEL section
    lines.push("BY MODEL".bold().fg(WARN).into());
    lines.push(
        Line::from(format!("{:<30} {:>8} {:>10}", "Model", "Calls", "Cost"))
            .style(Style::default().fg(TEXT_SECONDARY)),
    );

    let mut sorted_models: Vec<CostEntry> = cost.by_model.clone();
    sorted_models.sort_by(|a, b| b.cost_usd.partial_cmp(&a.cost_usd).unwrap_or(std::cmp::Ordering::Equal));

    for (i, entry) in sorted_models.iter().enumerate() {
        let style = if i == selected {
            selected_style()
        } else {
            Style::default().fg(TEXT_PRIMARY)
        };
        lines.push(
            Line::from(format!(
                "{:<30} {:>8} {:>9.3}$",
                entry.model, entry.calls, entry.cost_usd
            ))
            .style(style),
        );
    }
    lines.push("".into());

    // BY DATE sparkline (last 7 days)
    if !cost.by_date.is_empty() {
        lines.push("BY DATE (last 7 days)".bold().fg(WARN).into());
        let spark_line = build_date_sparkline(&cost.by_date);
        lines.push(Line::from(spark_line).style(Style::default().fg(ACCENT)));
        // Show dates as labels
        let dates: Vec<&str> = cost
            .by_date
            .iter()
            .map(|d| {
                // Take last 5 chars of date "MM-DD" from "YYYY-MM-DD"
                if d.date.len() >= 5 {
                    &d.date[d.date.len() - 5..]
                } else {
                    d.date.as_str()
                }
            })
            .collect();
        lines.push(
            Line::from(dates.join("  ")).style(Style::default().fg(TEXT_SECONDARY)),
        );
        lines.push("".into());
    }

    // BY PROJECT section
    if !cost.by_project.is_empty() {
        lines.push("BY PROJECT".bold().fg(WARN).into());
        lines.push(
            Line::from(format!("{:<30} {:>8} {:>10}", "Project", "Calls", "Cost"))
                .style(Style::default().fg(TEXT_SECONDARY)),
        );

        let offset = sorted_models.len();
        for (i, entry) in cost.by_project.iter().enumerate() {
            let row_idx = offset + i;
            let style = if row_idx == selected {
                selected_style()
            } else {
                Style::default().fg(TEXT_PRIMARY)
            };
            lines.push(
                Line::from(format!(
                    "{:<30} {:>8} {:>9.3}$",
                    entry.model, entry.calls, entry.cost_usd
                ))
                .style(style),
            );
        }
    }

    Paragraph::new(Text::from(lines))
        .block(Block::default().title(" Cost Center ").borders(Borders::ALL))
        .wrap(Wrap { trim: true })
}

/// Build sparkline string from CostByDate values using block characters.
fn build_date_sparkline(by_date: &[crate::tui::CostByDate]) -> String {
    let levels = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
    let max = by_date
        .iter()
        .map(|d| d.cost_usd)
        .fold(0.0f64, f64::max);

    if max <= 0.0 {
        return levels[0].repeat(by_date.len());
    }

    by_date
        .iter()
        .map(|d| {
            let norm = (d.cost_usd / max).clamp(0.0, 1.0);
            let idx = (norm * (levels.len() - 1) as f64).round() as usize;
            levels[idx.min(levels.len() - 1)]
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::{CostByDate, CostData, CostEntry, CostSummary, TuiData};

    fn sample_data() -> TuiData {
        TuiData {
            cost: CostData {
                by_model: vec![
                    CostEntry { model: "claude-opus-4.6".to_string(), calls: 42, cost_usd: 1.25 },
                    CostEntry { model: "claude-sonnet-4.6".to_string(), calls: 10, cost_usd: 0.30 },
                ],
                by_project: vec![CostEntry {
                    model: "convergio".to_string(),
                    calls: 100,
                    cost_usd: 3.50,
                }],
                by_date: vec![
                    CostByDate { date: "2026-03-17".to_string(), cost_usd: 0.10 },
                    CostByDate { date: "2026-03-23".to_string(), cost_usd: 0.75 },
                ],
                summary: CostSummary {
                    run_count: 42,
                    avg_duration_secs: 1234.5,
                    total_cost_usd: 5.25,
                },
            },
            ..TuiData::default()
        }
    }

    #[test]
    fn cost_center_contains_header() {
        let data = sample_data();
        let p = cost_center(&data, 0);
        // Paragraph renders to Text — inspect via debug representation
        let debug = format!("{:?}", p);
        assert!(debug.contains("COST CENTER"), "Missing COST CENTER header");
    }

    #[test]
    fn cost_center_contains_by_model_section() {
        let data = sample_data();
        let p = cost_center(&data, 0);
        let debug = format!("{:?}", p);
        assert!(debug.contains("BY MODEL"), "Missing BY MODEL section");
        assert!(debug.contains("claude-opus-4.6"), "Missing model entry");
    }

    #[test]
    fn cost_center_shows_no_data_when_empty() {
        let data = TuiData::default();
        let p = cost_center(&data, 0);
        let debug = format!("{:?}", p);
        assert!(
            debug.contains("No cost data available"),
            "Missing no-data message"
        );
    }

    #[test]
    fn cost_center_sorted_by_cost_desc() {
        let data = sample_data();
        let p = cost_center(&data, 0);
        let debug = format!("{:?}", p);
        // opus (1.25) must appear before sonnet (0.30) in output
        let opus_pos = debug.find("claude-opus-4.6").unwrap_or(usize::MAX);
        let sonnet_pos = debug.find("claude-sonnet-4.6").unwrap_or(usize::MAX);
        assert!(
            opus_pos < sonnet_pos,
            "Models not sorted by cost DESC: opus_pos={opus_pos} sonnet_pos={sonnet_pos}"
        );
    }

    #[test]
    fn build_date_sparkline_scales_correctly() {
        let dates = vec![
            CostByDate { date: "2026-03-22".to_string(), cost_usd: 0.0 },
            CostByDate { date: "2026-03-23".to_string(), cost_usd: 1.0 },
        ];
        let result = build_date_sparkline(&dates);
        // First bar should be lowest, last should be highest (█)
        assert!(result.contains("█"), "Max bar missing");
        assert!(result.contains("▁"), "Min bar missing");
    }
}
