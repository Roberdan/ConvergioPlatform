// Tests for chat view rendering helpers.
use crate::tui::views::chat_render::{
    dino_frame, message_lines, render_inline, styled_line, wrap_content, INDENT,
};
use crate::tui::widgets::TEXT_SECONDARY;
use crate::tui::ChatMessage;
use ratatui::style::Modifier;

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

#[test]
fn dino_frame_returns_nonempty() {
    let frame = dino_frame();
    assert!(!frame.is_empty());
}
