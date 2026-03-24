// Chat rendering helpers: inline markdown, word-wrap, and message-line building.
// Called from chat.rs (render_chat_view).

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::tui::{
    widgets::{ACCENT, MUTED, OK, TEXT_PRIMARY, TEXT_SECONDARY, WARN},
    ChatMessage,
};

fn is_user(role: &str) -> bool {
    role == "user"
}

// ── Markdown rendering ─────────────────────────────────────────────────

/// Render inline markdown: **bold**, `code`, *italic*.
pub(crate) fn render_inline(text: &str, base: ratatui::style::Color) -> Vec<Span<'static>> {
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
pub(crate) fn styled_line(
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

pub(crate) fn dino_frame() -> &'static str {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    DINO_FRAMES[(ms / 250) as usize % DINO_FRAMES.len()]
}

pub(crate) const INDENT: &str = "       "; // 7 chars to match label width

/// Build display lines for a single chat message.
pub(crate) fn message_lines(msg: &ChatMessage, is_last: bool, width: u16) -> Vec<Line<'static>> {
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
pub(crate) fn wrap_content(content: &str, max_width: usize) -> Vec<String> {
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
