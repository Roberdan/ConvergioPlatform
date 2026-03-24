// Rich popup system for contextual drill-down overlays.
// Replaces the plain-text detail_text popup with structured sections and action hints.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::tui::widgets::{ACCENT, MUTED, TEXT_PRIMARY, WARN};

/// A labeled section inside a popup (e.g. "Waves" with a list of lines).
#[derive(Debug, Clone)]
pub struct PopupSection {
    pub label: String,
    pub lines: Vec<String>,
}

/// Full content descriptor for a rich popup.
#[derive(Debug, Clone)]
pub struct PopupContent {
    pub title: String,
    pub sections: Vec<PopupSection>,
    /// Action key bindings shown in the bottom bar, e.g. [('p', "Provision"), ('s', "Stop")].
    pub actions: Vec<(char, String)>,
}

impl PopupContent {
    /// Returns the action label for the given key, if any.
    pub fn action_for_key(&self, key: char) -> Option<&str> {
        self.actions
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, label)| label.as_str())
    }
}

/// Renders a rich popup overlay centered on `area`.
///
/// Layout: 70% width, 80% height of terminal area.
/// Border: rounded, DIM color.
/// Title: ACCENT bold, centered at top.
/// Sections: label in WARN bold, content lines in TEXT_PRIMARY.
/// Bottom bar: action hints + [Esc] Close.
/// Content scrolls if it exceeds popup height.
pub fn render_rich_popup(frame: &mut Frame<'_>, area: Rect, content: &PopupContent) {
    let width = (area.width * 70 / 100).max(40).min(area.width);
    let height = (area.height * 80 / 100).max(6).min(area.height);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    let popup_area = Rect { x, y, width, height };

    // Clear background so underlying widgets don't bleed through.
    frame.render_widget(Clear, popup_area);

    // Build all content lines for the scrollable body.
    let mut body_lines: Vec<Line<'static>> = Vec::new();
    for section in &content.sections {
        // Section header in WARN bold.
        body_lines.push(Line::from(Span::styled(
            section.label.clone(),
            Style::default().fg(WARN).add_modifier(Modifier::BOLD),
        )));
        for line in &section.lines {
            body_lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(TEXT_PRIMARY),
            )));
        }
        // Blank separator between sections.
        body_lines.push(Line::from(""));
    }

    // Bottom action bar text.
    let action_bar = build_action_bar(&content.actions);

    // Available lines for content (popup height minus borders minus action bar row).
    let inner_height = height.saturating_sub(3); // top border + title row + bottom bar
    let scroll_offset = if body_lines.len() as u16 > inner_height {
        // Clamp scroll so end of content aligns with bottom when overflow.
        body_lines.len() as u16 - inner_height
    } else {
        0
    };

    let title = format!(" {} ", content.title);
    let block = Block::default()
        .title(Span::styled(
            title,
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
        .title_alignment(ratatui::layout::Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().add_modifier(Modifier::DIM));

    let paragraph = Paragraph::new(body_lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset, 0));
    frame.render_widget(paragraph, popup_area);

    // Render action bar as an overlay on the bottom row of the popup.
    if popup_area.height >= 3 {
        let bar_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + popup_area.height.saturating_sub(2),
            width: popup_area.width.saturating_sub(2),
            height: 1,
        };
        let bar = Paragraph::new(Line::from(Span::styled(
            action_bar,
            Style::default().fg(MUTED),
        )));
        frame.render_widget(bar, bar_area);
    }
}

/// Builds the bottom action bar string, e.g. "[p] Provision  [h] Heartbeat  [Esc] Close".
fn build_action_bar(actions: &[(char, String)]) -> String {
    let mut parts: Vec<String> = actions
        .iter()
        .map(|(k, label)| format!("[{}] {}", k, label))
        .collect();
    parts.push("[Esc] Close".to_string());
    parts.join("  ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn popup_content_creation() {
        let content = PopupContent {
            title: "Node Detail".to_string(),
            sections: vec![
                PopupSection {
                    label: "Status".to_string(),
                    lines: vec!["online".to_string(), "cpu: 42%".to_string()],
                },
                PopupSection {
                    label: "Actions".to_string(),
                    lines: vec!["provision".to_string()],
                },
            ],
            actions: vec![('p', "Provision".to_string()), ('s', "Stop".to_string())],
        };

        assert_eq!(content.title, "Node Detail");
        assert_eq!(content.sections.len(), 2);
        assert_eq!(content.sections[0].label, "Status");
        assert_eq!(content.sections[0].lines.len(), 2);
        assert_eq!(content.actions.len(), 2);
    }

    #[test]
    fn action_for_key_found() {
        let content = PopupContent {
            title: "Test".to_string(),
            sections: vec![],
            actions: vec![
                ('p', "Provision".to_string()),
                ('h', "Heartbeat".to_string()),
                ('s', "Stop".to_string()),
            ],
        };

        assert_eq!(content.action_for_key('p'), Some("Provision"));
        assert_eq!(content.action_for_key('h'), Some("Heartbeat"));
        assert_eq!(content.action_for_key('s'), Some("Stop"));
    }

    #[test]
    fn action_for_key_not_found() {
        let content = PopupContent {
            title: "Test".to_string(),
            sections: vec![],
            actions: vec![('p', "Provision".to_string())],
        };

        assert_eq!(content.action_for_key('x'), None);
        assert_eq!(content.action_for_key('q'), None);
    }

    #[test]
    fn build_action_bar_formats_correctly() {
        let actions = vec![
            ('p', "Provision".to_string()),
            ('h', "Heartbeat".to_string()),
        ];
        let bar = build_action_bar(&actions);
        assert!(bar.contains("[p] Provision"));
        assert!(bar.contains("[h] Heartbeat"));
        assert!(bar.contains("[Esc] Close"));
    }

    #[test]
    fn build_action_bar_empty_actions() {
        let bar = build_action_bar(&[]);
        assert_eq!(bar, "[Esc] Close");
    }

    #[test]
    fn popup_section_clone() {
        let section = PopupSection {
            label: "Waves".to_string(),
            lines: vec!["wave-1: done".to_string()],
        };
        let cloned = section.clone();
        assert_eq!(cloned.label, section.label);
        assert_eq!(cloned.lines, section.lines);
    }
}
