// Chat view — conversational interface with the Convergio daemon.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::tui::{
    widgets::{ACCENT, MUTED, OK, TEXT_PRIMARY, TEXT_SECONDARY, WARN},
    ChatMessage, TuiData,
};

fn is_user(role: &str) -> bool {
    role == "user"
}

// ── Markdown rendering ─────────────────────────────────────────────────

/// Render inline markdown: **bold**, `code`, *italic*.
fn render_inline(text: &str, base: ratatui::style::Color) -> Vec<Span<'static>> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut rest = text.to_string();

    while !rest.is_empty() {
        // Find next marker
        let bold = rest.find("**");
        let code = rest.find('`');
        let next = match (bold, code) {
            (Some(b), Some(c)) if b <= c => Some(("**", b)),
            (_, Some(c)) => Some(("`", c)),
            (Some(b), _) => Some(("**", b)),
            _ => None,
        };

        match next {
            None => {
                spans.push(Span::styled(rest.clone(), Style::default().fg(base)));
                break;
            }
            Some((mk, pos)) => {
                if pos > 0 {
                    spans.push(Span::styled(
                        rest[..pos].to_string(),
                        Style::default().fg(base),
                    ));
                }
                let after = &rest[pos + mk.len()..];
                if let Some(end) = after.find(mk) {
                    let inner = after[..end].to_string();
                    let style = if mk == "**" {
                        Style::default()
                            .fg(TEXT_PRIMARY)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(WARN)
                    };
                    spans.push(Span::styled(inner, style));
                    rest = after[end + mk.len()..].to_string();
                } else {
                    // No closing marker
                    spans.push(Span::styled(rest.clone(), Style::default().fg(base)));
                    break;
                }
            }
        }
    }
    spans
}

/// Convert a content line to styled spans with 7-char indent prefix.
fn styled_line(
    raw: &str,
    base: ratatui::style::Color,
    indent: &str,
) -> Line<'static> {
    let trimmed = raw.trim();

    // ─── separator
    if trimmed == "---" || trimmed == "***" || trimmed == "___" {
        return Line::from(vec![
            Span::raw(indent.to_string()),
            Span::styled(
                "────────────────────────────────────",
                Style::default().fg(MUTED),
            ),
        ]);
    }

    // Headers
    let (header_text, header_mod) = if let Some(t) = trimmed.strip_prefix("### ") {
        (Some(t), Modifier::BOLD)
    } else if let Some(t) = trimmed.strip_prefix("## ") {
        (Some(t), Modifier::BOLD)
    } else if let Some(t) = trimmed.strip_prefix("# ") {
        (Some(t), Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        (None, Modifier::empty())
    };

    if let Some(ht) = header_text {
        return Line::from(vec![
            Span::raw(indent.to_string()),
            Span::styled(
                ht.to_string(),
                Style::default().fg(ACCENT).add_modifier(header_mod),
            ),
        ]);
    }

    // Bullet lists: - item or * item
    if let Some(item) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
        let mut spans = vec![
            Span::raw(indent.to_string()),
            Span::styled("  • ", Style::default().fg(ACCENT)),
        ];
        spans.extend(render_inline(item, base));
        return Line::from(spans);
    }

    // Table rows: | col | col |
    if trimmed.starts_with('|') && trimmed.ends_with('|') {
        // Separator row
        if trimmed.contains("---") {
            return Line::from(vec![
                Span::raw(indent.to_string()),
                Span::styled(trimmed.to_string(), Style::default().fg(MUTED)),
            ]);
        }
        // Data row — render cells with alternating style
        let mut spans = vec![Span::raw(indent.to_string())];
        let cells: Vec<&str> = trimmed.split('|').collect();
        for (i, cell) in cells.iter().enumerate() {
            let c = cell.trim();
            if c.is_empty() && (i == 0 || i == cells.len() - 1) {
                spans.push(Span::styled("│", Style::default().fg(MUTED)));
            } else if c.is_empty() {
                continue;
            } else {
                spans.push(Span::styled("│ ", Style::default().fg(MUTED)));
                spans.extend(render_inline(c, base));
                spans.push(Span::styled(" ", Style::default().fg(base)));
            }
        }
        return Line::from(spans);
    }

    // Plain text with inline formatting
    let mut spans = vec![Span::raw(indent.to_string())];
    spans.extend(render_inline(raw, base));
    Line::from(spans)
}

// ── Message rendering ───────────────────────────────────────────────────

/// Walking dino animation.
const DINO_FRAMES: &[&str] = &[
    "🦕        ", " 🦕       ", "  🦕      ", "   🦕     ",
    "    🦕    ", "     🦕   ", "      🦕  ", "       🦕 ",
    "      🦕  ", "     🦕   ", "    🦕    ", "   🦕     ",
    "  🦕      ", " 🦕       ",
];

fn dino_frame() -> &'static str {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    DINO_FRAMES[(ms / 250) as usize % DINO_FRAMES.len()]
}

const INDENT: &str = "       "; // 7 chars to match label width

/// Build display lines for a single chat message.
fn message_lines(msg: &ChatMessage, is_last: bool, width: u16) -> Vec<Line<'static>> {
    let label = if is_user(&msg.role) {
        Span::styled("  you  ", Style::default().fg(ACCENT).bold())
    } else {
        Span::styled("   ◆   ", Style::default().fg(OK).bold())
    };

    let base = if is_user(&msg.role) {
        TEXT_PRIMARY
    } else {
        TEXT_SECONDARY
    };

    // Dino placeholder while streaming
    if !is_user(&msg.role) && msg.content.is_empty() && is_last {
        return vec![
            Line::from(vec![
                label,
                Span::styled(dino_frame().to_string(), Style::default().fg(MUTED)),
            ]),
            Line::raw(""),
        ];
    }

    let mut lines: Vec<Line<'static>> = Vec::new();
    let usable = (width as usize).saturating_sub(9); // 7 indent + 2 border

    // Pre-wrap long lines manually so scroll math is accurate.
    let wrapped = wrap_content(&msg.content, usable);

    for (i, raw) in wrapped.iter().enumerate() {
        if i == 0 {
            // First line: label + content
            let line = styled_line(raw, base, "");
            let mut spans = vec![label.clone()];
            spans.extend(line.spans);
            lines.push(Line::from(spans));
        } else {
            lines.push(styled_line(raw, base, INDENT));
        }
    }

    // Separator between messages
    lines.push(Line::raw(""));
    lines
}

/// Word-wrap content lines to fit within `max_width` characters.
fn wrap_content(content: &str, max_width: usize) -> Vec<String> {
    let max_width = max_width.max(20);
    let mut result = Vec::new();

    for line in content.lines() {
        if line.len() <= max_width {
            result.push(line.to_string());
            continue;
        }
        // Word wrap
        let mut current = String::new();
        for word in line.split_whitespace() {
            if current.is_empty() {
                current = word.to_string();
            } else if current.len() + 1 + word.len() <= max_width {
                current.push(' ');
                current.push_str(word);
            } else {
                result.push(current);
                current = word.to_string();
            }
        }
        if !current.is_empty() {
            result.push(current);
        }
    }

    if result.is_empty() {
        result.push(String::new());
    }
    result
}

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

// ── Tests ───────────────────────────────────────────────────────────────

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
        let lines = message_lines(&user_msg("What is Plan 708?"), false, 80);
        let rendered: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(rendered.contains("you"));
        assert!(rendered.contains("What is Plan 708?"));
    }

    #[test]
    fn message_lines_assistant_contains_diamond() {
        let lines = message_lines(&assistant_msg("Plan 708 is done."), false, 80);
        let rendered: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(rendered.contains('◆'));
        assert!(rendered.contains("Plan 708 is done."));
    }

    #[test]
    fn message_lines_multiline_indented() {
        let lines = message_lines(&assistant_msg("Line one\nLine two"), false, 80);
        let line_two = lines.iter().find(|l| {
            l.spans.iter().any(|s| s.content.contains("Line two"))
        });
        assert!(line_two.is_some());
    }

    #[test]
    fn is_user_correct() {
        assert!(is_user("user"));
        assert!(!is_user("assistant"));
    }

    #[test]
    fn render_inline_bold() {
        let spans = render_inline("hello **world** end", TEXT_SECONDARY);
        let texts: Vec<&str> = spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(texts, vec!["hello ", "world", " end"]);
        assert!(spans[1].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn render_inline_code() {
        let spans = render_inline("run `curl` now", TEXT_SECONDARY);
        let texts: Vec<&str> = spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(texts, vec!["run ", "curl", " now"]);
    }

    #[test]
    fn styled_line_separator() {
        let line = styled_line("---", TEXT_SECONDARY, INDENT);
        let rendered: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(rendered.contains('─'));
    }

    #[test]
    fn styled_line_header() {
        let line = styled_line("## Status", TEXT_SECONDARY, INDENT);
        let rendered: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(rendered.contains("Status"));
    }

    #[test]
    fn styled_line_bullet() {
        let line = styled_line("- item one", TEXT_SECONDARY, INDENT);
        let rendered: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(rendered.contains('•'));
        assert!(rendered.contains("item one"));
    }

    #[test]
    fn wrap_content_short_lines_unchanged() {
        let result = wrap_content("short line", 80);
        assert_eq!(result, vec!["short line"]);
    }

    #[test]
    fn wrap_content_long_line_wraps() {
        let long = "word ".repeat(20); // 100 chars
        let result = wrap_content(&long, 40);
        assert!(result.len() > 1);
        for line in &result {
            assert!(line.len() <= 40);
        }
    }

    #[test]
    fn wrap_content_empty_returns_one_empty() {
        let result = wrap_content("", 80);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn dino_empty_message_shows_animation() {
        let lines = message_lines(&assistant_msg(""), true, 80);
        let rendered: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(rendered.contains('◆'));
        assert!(rendered.contains('🦕'));
    }

    #[test]
    fn table_row_renders_with_separators() {
        let line = styled_line("| A | B |", TEXT_SECONDARY, INDENT);
        let rendered: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(rendered.contains('│'));
        assert!(rendered.contains('A'));
    }
}
