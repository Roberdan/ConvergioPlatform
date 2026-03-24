// Chat view — conversational interface with the Convergio daemon.

use super::chat_render::message_lines;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::tui::{
    widgets::MUTED,
    widgets::TEXT_PRIMARY,
    TuiData,
};

// ── View rendering ──────────────────────────────────────────────────────

/// Renders the full Chat view: messages area + input bar.
pub fn render_chat_view(
    frame: &mut Frame<'_>,
    area: Rect,
    data: &TuiData,
    chat_input: &str,
    chat_sending: bool,
    scroll_offset: u16,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // message scroll area
            Constraint::Length(3), // input bar
        ])
        .split(area);

    render_messages(frame, chunks[0], data, scroll_offset);
    render_input_bar(frame, chunks[1], chat_input, chat_sending);
}

fn render_messages(frame: &mut Frame<'_>, area: Rect, data: &TuiData, scroll_offset: u16) {
    let messages = &data.chat_messages;
    let content_width = area.width.saturating_sub(2); // borders

    let content: Text<'static> = if messages.is_empty() {
        Text::from(vec![
            Line::raw(""),
            Line::from(Span::styled(
                "  Type a message and press Enter to talk to Ali.",
                Style::default().fg(MUTED),
            )),
            Line::raw(""),
            Line::from(Span::styled(
                "  ↑↓ scroll  PgUp/PgDn fast  Home/End top/bottom",
                Style::default().fg(MUTED),
            )),
        ])
    } else {
        let mut lines: Vec<Line<'static>> = vec![Line::raw("")];
        let count = messages.len();
        for (i, msg) in messages.iter().enumerate() {
            lines.extend(message_lines(msg, i == count - 1, content_width));
        }
        Text::from(lines)
    };

    // Lines are pre-wrapped — count is accurate for scroll math.
    let line_count = content.lines.len() as u16;
    let visible = area.height.saturating_sub(2);
    let max_scroll = line_count.saturating_sub(visible);
    let capped_offset = scroll_offset.min(max_scroll);
    let scroll = max_scroll.saturating_sub(capped_offset);

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title(" ◆ Convergio Chat ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(Style::default().fg(MUTED)),
        )
        .scroll((scroll, 0));

    frame.render_widget(paragraph, area);
}

fn render_input_bar(frame: &mut Frame<'_>, area: Rect, chat_input: &str, _sending: bool) {
    let paragraph = Paragraph::new(Line::from(Span::styled(
        format!(" > {chat_input}"),
        Style::default().fg(TEXT_PRIMARY),
    )))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(Style::default().fg(MUTED)),
    );
    frame.render_widget(paragraph, area);
}
