use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::tui::TuiData;

use super::{ACCENT, MUTED, WARN};

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
            Style::default().fg(super::OK),
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
