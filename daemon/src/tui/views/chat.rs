// Chat view — conversational interface with the Convergio daemon.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

use crate::tui::{
    widgets::{ACCENT, MUTED, OK, TEXT_PRIMARY, TEXT_SECONDARY, WARN},
    ChatMessage, TuiData,
};

/// Returns true if `role` is a user message (as opposed to assistant).
fn is_user(role: &str) -> bool {
    role == "user"
}

/// Render a single line of markdown-ish content into styled spans.
/// Supports: **bold**, `code`, # headers, --- separators.
fn render_md_line(line: &str, base_color: ratatui::style::Color) -> Vec<Span<'static>> {
    let trimmed = line.trim();

    // --- separator
    if trimmed == "---" || trimmed == "***" || trimmed == "___" {
        return vec![Span::styled(
            "  ─────────────────────────────",
            Style::default().fg(MUTED),
        )];
    }

    // # Headers
    if let Some(rest) = trimmed.strip_prefix("### ") {
        return vec![Span::styled(
            format!("  {rest}"),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )];
    }
    if let Some(rest) = trimmed.strip_prefix("## ") {
        return vec![Span::styled(
            format!("  {rest}"),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )];
    }
    if let Some(rest) = trimmed.strip_prefix("# ") {
        return vec![Span::styled(
            format!("  {rest}"),
            Style::default()
                .fg(ACCENT)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )];
    }

    // Inline formatting: **bold** and `code`
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut remaining = line.to_string();

    while !remaining.is_empty() {
        // Find the next ** or `
        let bold_pos = remaining.find("**");
        let code_pos = remaining.find('`');

        let next = match (bold_pos, code_pos) {
            (Some(b), Some(c)) => {
                if b <= c {
                    Some(("**", b))
                } else {
                    Some(("`", c))
                }
            }
            (Some(b), None) => Some(("**", b)),
            (None, Some(c)) => Some(("`", c)),
            (None, None) => None,
        };

        match next {
            None => {
                spans.push(Span::styled(remaining.clone(), Style::default().fg(base_color)));
                break;
            }
            Some((marker, pos)) => {
                // Text before marker
                if pos > 0 {
                    let before: String = remaining[..pos].to_string();
                    spans.push(Span::styled(before, Style::default().fg(base_color)));
                }

                let after = &remaining[pos + marker.len()..];
                if let Some(end) = after.find(marker) {
                    let inner: String = after[..end].to_string();
                    let style = if marker == "**" {
                        Style::default()
                            .fg(TEXT_PRIMARY)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(WARN)
                    };
                    spans.push(Span::styled(inner, style));
                    remaining = after[end + marker.len()..].to_string();
                } else {
                    // No closing marker — render as plain text
                    spans.push(Span::styled(
                        remaining.clone(),
                        Style::default().fg(base_color),
                    ));
                    break;
                }
            }
        }
    }

    spans
}

/// Build display lines for a single chat message with markdown rendering.
fn message_lines(msg: &ChatMessage, is_last: bool) -> Vec<Line<'static>> {
    let label = if is_user(&msg.role) {
        Span::styled("  you  ", Style::default().fg(ACCENT).bold())
    } else {
        Span::styled("   ◆   ", Style::default().fg(OK).bold())
    };

    let mut lines: Vec<Line<'static>> = Vec::new();
    let base_color = if is_user(&msg.role) {
        TEXT_PRIMARY
    } else {
        TEXT_SECONDARY
    };

    // Show dino animation when assistant message is empty (still streaming).
    if !is_user(&msg.role) && msg.content.is_empty() && is_last {
        lines.push(Line::from(vec![
            label,
            Span::styled(
                dino_frame().to_string(),
                Style::default().fg(MUTED),
            ),
        ]));
        lines.push(Line::raw(""));
        return lines;
    }

    let mut content_lines = msg.content.lines();
    let first = content_lines.next().unwrap_or("");

    // First line: label + rendered content
    let mut first_spans = vec![label.clone()];
    first_spans.extend(render_md_line(first, base_color));
    lines.push(Line::from(first_spans));

    // Continuation lines indented to match label width (7 spaces).
    for part in content_lines {
        let mut spans = vec![Span::raw("       ")];
        spans.extend(render_md_line(part, base_color));
        lines.push(Line::from(spans));
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

    let content: Text<'static> = if messages.is_empty() {
        Text::from(vec![
            Line::raw(""),
            Line::from(Span::styled(
                "  Start a conversation — type below and press Enter.",
                Style::default().fg(MUTED),
            )),
            Line::raw(""),
            Line::from(Span::styled(
                "  ↑↓ scroll  PgUp/PgDn fast scroll  Enter send",
                Style::default().fg(MUTED),
            )),
        ])
    } else {
        let mut lines: Vec<Line<'static>> = vec![Line::raw("")];
        let count = messages.len();
        for (i, msg) in messages.iter().enumerate() {
            lines.extend(message_lines(msg, i == count - 1));
        }
        Text::from(lines)
    };

    // Scroll to bottom minus manual offset.
    let line_count = content.lines.len() as u16;
    let visible = area.height.saturating_sub(2);
    let max_scroll = line_count.saturating_sub(visible);
    let scroll = max_scroll.saturating_sub(scroll_offset);

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

fn render_input_bar(frame: &mut Frame<'_>, area: Rect, chat_input: &str, _sending: bool) {
    let display = format!(" > {chat_input}");
    let bar_color = TEXT_PRIMARY;

    let paragraph =
        Paragraph::new(Line::from(Span::styled(display, Style::default().fg(bar_color)))).block(
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
    use crate::tui::ChatMessage;

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
        let lines = message_lines(&msg, false);
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
        let lines = message_lines(&msg, false);
        let rendered = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect::<String>();
        assert!(rendered.contains('◆'), "assistant diamond label missing");
        assert!(
            rendered.contains("Plan 708 is done."),
            "assistant content missing"
        );
    }

    #[test]
    fn message_lines_multiline_content_indented() {
        let msg = assistant_msg("Line one\nLine two");
        let lines = message_lines(&msg, false);
        let line_two = lines.iter().find(|l| {
            l.spans
                .iter()
                .any(|s| s.content.as_ref().contains("Line two"))
        });
        assert!(line_two.is_some(), "Line two not found");
        let first_span = &line_two.unwrap().spans[0];
        assert!(first_span.content.as_ref().starts_with(' '), "not indented");
    }

    #[test]
    fn is_user_returns_true_for_user_role() {
        assert!(is_user("user"));
        assert!(!is_user("assistant"));
        assert!(!is_user(""));
    }

    #[test]
    fn render_md_bold_produces_bold_span() {
        let spans = render_md_line("hello **world** end", TEXT_SECONDARY);
        let texts: Vec<&str> = spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(texts, vec!["hello ", "world", " end"]);
        // Middle span should be bold
        assert!(spans[1].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn render_md_code_produces_highlighted_span() {
        let spans = render_md_line("run `curl` now", TEXT_SECONDARY);
        let texts: Vec<&str> = spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(texts, vec!["run ", "curl", " now"]);
        assert_eq!(spans[1].style.fg, Some(WARN));
    }

    #[test]
    fn render_md_separator_produces_line() {
        let spans = render_md_line("---", TEXT_SECONDARY);
        assert!(spans[0].content.contains('─'));
    }

    #[test]
    fn render_md_header_produces_bold_accent() {
        let spans = render_md_line("## Status", TEXT_SECONDARY);
        assert!(spans[0].content.contains("Status"));
        assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
    }
}
