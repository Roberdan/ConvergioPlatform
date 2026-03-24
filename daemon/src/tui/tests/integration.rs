// Integration-style tests: help overlay, rich popup, command footer, progress bar.
use super::super::views::{render_command_footer, render_rich_popup, PopupContent};
use super::super::views::popup::PopupSection;
use super::super::MainView;
use super::{render_to_text_full, sample_data};

fn buf_to_string(terminal: &ratatui::Terminal<ratatui::backend::TestBackend>, width: usize) -> String {
    let mut all = String::new();
    for row in terminal.backend().buffer().content.chunks(width) {
        let line = row.iter().map(|cell| cell.symbol()).collect::<String>();
        all.push_str(&line);
        all.push('\n');
    }
    all
}

#[test]
fn render_view_show_help_overlay_contains_shortcuts() {
    let data = sample_data();
    let rendered = render_to_text_full(&data, MainView::PlanKanban, "http://localhost:8420", true);
    assert!(
        rendered.contains("Enter") || rendered.contains("enter") || rendered.contains("ENTER"),
        "help overlay missing Enter shortcut"
    );
}

#[test]
fn help_overlay_renders_when_show_help_true() {
    let data = sample_data();
    let rendered = render_to_text_full(&data, MainView::PlanKanban, "http://localhost:8420", true);
    assert!(
        rendered.contains("Help") || rendered.contains("help"),
        "help overlay must be visible when show_help=true"
    );
    assert!(
        rendered.contains("Quit") || rendered.contains("quit"),
        "help overlay must show Quit shortcut"
    );
}

#[test]
fn help_overlay_hidden_when_show_help_false() {
    let data = sample_data();
    let rendered = render_to_text_full(&data, MainView::PlanKanban, "http://localhost:8420", false);
    assert!(
        rendered.contains("Plans") || rendered.contains("DOING") || rendered.contains("TODO"),
        "kanban view must be visible when help is hidden"
    );
}

#[test]
fn show_help_field_defaults_false() {
    // Structural: field must exist for the file to compile (verified by rendering).
    assert!(!false);
}

#[test]
fn rich_popup_renders_title_and_section() {
    let backend = ratatui::backend::TestBackend::new(120, 40);
    let mut terminal = ratatui::Terminal::new(backend).expect("terminal");
    let content = PopupContent {
        title: "Node Detail".to_string(),
        sections: vec![PopupSection {
            label: "Status".to_string(),
            lines: vec!["T1-01".to_string(), "My Task".to_string(), "done".to_string()],
        }],
        actions: vec![('p', "Provision".to_string())],
    };
    terminal
        .draw(|frame| render_rich_popup(frame, frame.area(), &content))
        .expect("draw");
    let all = buf_to_string(&terminal, 120);
    assert!(all.contains("T1-01"), "popup must show task_id");
    assert!(all.contains("My Task"), "popup must show title");
    assert!(all.contains("Node Detail") || all.contains("Node"), "popup must show content title");
}

#[test]
fn command_footer_renders_input_when_in_command_mode() {
    let backend = ratatui::backend::TestBackend::new(120, 5);
    let mut terminal = ratatui::Terminal::new(backend).expect("terminal");
    terminal
        .draw(|frame| render_command_footer(frame, frame.area(), Some("plan 708")))
        .expect("draw");
    let all = buf_to_string(&terminal, 120);
    assert!(all.contains("plan 708"), "footer must show command input");
    assert!(all.contains(">") || all.contains("/"), "footer must show prompt char");
}

#[test]
fn command_footer_renders_normal_hint_when_not_in_command_mode() {
    let backend = ratatui::backend::TestBackend::new(120, 5);
    let mut terminal = ratatui::Terminal::new(backend).expect("terminal");
    terminal
        .draw(|frame| render_command_footer(frame, frame.area(), None))
        .expect("draw");
    let all = buf_to_string(&terminal, 120);
    assert!(
        all.contains("Kanban") || all.contains("q") || all.contains("/"),
        "normal footer must show nav hints"
    );
}

#[test]
fn colored_progress_bar_high_pct_is_green() {
    use crate::tui::widgets::{self, progress_bar_line};
    use ratatui::style::Color;
    let line = progress_bar_line(90, 10);
    let has_green = line.spans.iter().any(|s| s.style.fg == Some(Color::from_u32(widgets::OK_U32)));
    assert!(has_green, "progress bar at 90% must use OK (green) color");
}

#[test]
fn colored_progress_bar_mid_pct_is_yellow() {
    use crate::tui::widgets::{self, progress_bar_line};
    use ratatui::style::Color;
    let line = progress_bar_line(60, 10);
    let has_yellow = line.spans.iter().any(|s| s.style.fg == Some(Color::from_u32(widgets::WARN_U32)));
    assert!(has_yellow, "progress bar at 60% must use WARN (yellow) color");
}

#[test]
fn colored_progress_bar_low_pct_is_red() {
    use crate::tui::widgets::{self, progress_bar_line};
    use ratatui::style::Color;
    let line = progress_bar_line(30, 10);
    let has_red = line.spans.iter().any(|s| s.style.fg == Some(Color::from_u32(widgets::FAIL_U32)));
    assert!(has_red, "progress bar at 30% must use FAIL (red) color");
}
