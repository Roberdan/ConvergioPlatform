// Chat view — conversational interface with the Convergio daemon.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

use crate::tui::{
    widgets::{ACCENT, MUTED, OK, TEXT_PRIMARY, TEXT_SECONDARY},
    ChatMessage, TuiData,
};

/// Returns true if `role` is a user message (as opposed to assistant).
fn is_user(role: &str) -> bool {
    role == "user"
}

/// Build display lines for a single chat message.
fn message_lines(msg: &ChatMessage) -> Vec<Line<'static>> {
    let label = if is_user(&msg.role) {
        Span::styled("  you  ", Style::default().fg(ACCENT).bold())
    } else {
        Span::styled("   ◆   ", Style::default().fg(OK).bold())
    };

    let mut lines: Vec<Line<'static>> = Vec::new();

    // First line: label + first content line
    let content_color = if is_user(&msg.role) {
        TEXT_PRIMARY
    } else {
        TEXT_SECONDARY
    };

    let mut content_lines = msg.content.lines();
    let first = content_lines.next().unwrap_or("");
    lines.push(Line::from(vec![
        label.clone(),
        Span::styled(first.to_string(), Style::default().fg(content_color)),
    ]));

    // Continuation lines indented to match label width (7 spaces).
    for part in content_lines {
        lines.push(Line::from(vec![
            Span::raw("       "),
            Span::styled(part.to_string(), Style::default().fg(content_color)),
        ]));
    }

    // Blank line separator between messages.
    lines.push(Line::raw(""));
    lines
}

/// Renders the full Chat view split into messages area + input bar.
pub fn render_chat_view(
    frame: &mut Frame<'_>,
    area: Rect,
    data: &TuiData,
    chat_input: &str,
    chat_sending: bool,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // message scroll area
            Constraint::Length(3), // input bar
        ])
        .split(area);

    render_messages(frame, chunks[0], data);
    render_input_bar(frame, chunks[1], chat_input, chat_sending);
}

fn render_messages(frame: &mut Frame<'_>, area: Rect, data: &TuiData) {
    let messages = &data.chat_messages;

    let content: Text<'static> = if messages.is_empty() {
        Text::from(vec![
            Line::raw(""),
            Line::from(Span::styled(
                "  Start a conversation — type below and press Enter.",
                Style::default().fg(MUTED),
            )),
        ])
    } else {
        let mut lines: Vec<Line<'static>> = vec![Line::raw("")];
        for msg in messages {
            lines.extend(message_lines(msg));
        }
        Text::from(lines)
    };

    // Scroll to bottom: skip lines so newest message is visible.
    let line_count = content.lines.len() as u16;
    let visible = area.height.saturating_sub(2); // subtract border
    let scroll = line_count.saturating_sub(visible);

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title(" ◆ Convergio Chat ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(Style::default().fg(MUTED)),
        )
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));

    frame.render_widget(paragraph, area);
}

/// Walking dino animation frames — cycles every ~300ms.
const DINO_FRAMES: &[&str] = &[
    "  🦕        ",
    "   🦕       ",
    "    🦕      ",
    "     🦕     ",
    "      🦕    ",
    "       🦕   ",
    "        🦕  ",
    "         🦕 ",
    "        🦕  ",
    "       🦕   ",
    "      🦕    ",
    "     🦕     ",
    "    🦕      ",
    "   🦕       ",
];

fn dino_frame() -> &'static str {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let idx = (ms / 300) as usize % DINO_FRAMES.len();
    DINO_FRAMES[idx]
}

fn render_input_bar(frame: &mut Frame<'_>, area: Rect, chat_input: &str, sending: bool) {
    let display = if sending {
        format!("{}Ali is thinking...", dino_frame())
    } else {
        format!(" > {chat_input}")
    };

    let bar_color = if sending { MUTED } else { TEXT_PRIMARY };

    let paragraph = Paragraph::new(Line::from(Span::styled(display, Style::default().fg(bar_color))))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(Style::default().fg(MUTED)),
        );

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::{ChatMessage, TuiData};

    fn user_msg(content: &str) -> ChatMessage {
        ChatMessage {
            role: "user".to_string(),
            content: content.to_string(),
            timestamp: "2026-03-24T10:00:00Z".to_string(),
        }
    }

    fn assistant_msg(content: &str) -> ChatMessage {
        ChatMessage {
            role: "assistant".to_string(),
            content: content.to_string(),
            timestamp: "2026-03-24T10:00:01Z".to_string(),
        }
    }

    #[test]
    fn message_lines_user_contains_you_label() {
        let msg = user_msg("What is Plan 708?");
        let lines = message_lines(&msg);
        let rendered = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect::<String>();
        assert!(rendered.contains("you"), "user label missing");
        assert!(rendered.contains("What is Plan 708?"), "user content missing");
    }

    #[test]
    fn message_lines_assistant_contains_diamond_label() {
        let msg = assistant_msg("Plan 708 is done.");
        let lines = message_lines(&msg);
        let rendered = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect::<String>();
        assert!(rendered.contains('◆'), "assistant diamond label missing");
        assert!(rendered.contains("Plan 708 is done."), "assistant content missing");
    }

    #[test]
    fn message_lines_multiline_content_indented() {
        let msg = assistant_msg("Line one\nLine two");
        let lines = message_lines(&msg);
        // Line two should appear on a separate line indented with spaces.
        let line_two = lines.iter().find(|l| {
            l.spans
                .iter()
                .any(|s| s.content.as_ref().contains("Line two"))
        });
        assert!(line_two.is_some(), "Line two not found in rendered lines");
        let first_span = &line_two.unwrap().spans[0];
        assert!(
            first_span.content.as_ref().starts_with(' '),
            "Line two not indented"
        );
    }

    #[test]
    fn is_user_returns_true_for_user_role() {
        assert!(is_user("user"));
        assert!(!is_user("assistant"));
        assert!(!is_user(""));
    }
}
