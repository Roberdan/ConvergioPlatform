// Mode badge rendering tests for the project tree view (T2-01).
use super::super::ProjectTreeData;
use crate::tui::views::project_tree::{build_tree_lines, mode_badge_spans};

#[test]
fn mode_badge_sequential_renders_seq() {
    let spans = mode_badge_spans(&Some("sequential".into()));
    let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
    assert!(text.contains("[SEQ]"), "sequential should render [SEQ], got: {text}");
}

#[test]
fn mode_badge_parallel_renders_par() {
    let spans = mode_badge_spans(&Some("parallel".into()));
    let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
    assert!(text.contains("[PAR]"), "parallel should render [PAR], got: {text}");
}

#[test]
fn mode_badge_mixed_renders_mix() {
    let spans = mode_badge_spans(&Some("mixed".into()));
    let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
    assert!(text.contains("[MIX]"), "mixed should render [MIX], got: {text}");
}

#[test]
fn mode_badge_conditional_renders_cnd() {
    let spans = mode_badge_spans(&Some("conditional".into()));
    let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
    assert!(text.contains("[CND]"), "conditional should render [CND], got: {text}");
}

#[test]
fn mode_badge_empty_returns_no_spans() {
    let spans = mode_badge_spans(&None);
    assert!(spans.is_empty(), "None mode should produce no spans");
    let spans_empty = mode_badge_spans(&Some(String::new()));
    assert!(spans_empty.is_empty(), "empty string mode should produce no spans");
}

#[test]
fn tree_master_line_contains_mode_badge_text() {
    use super::super::ProjectTreeNode;
    let tree = ProjectTreeData {
        project_name: "convergio".into(),
        total_tasks: 100,
        done_tasks: 50,
        plans: vec![ProjectTreeNode {
            id: 711,
            name: "Convergio Vision".into(),
            status: "draft".into(),
            is_master: true,
            execution_mode: Some("mixed".into()),
            children: vec![],
            ..Default::default()
        }],
    };
    let (lines, _) = build_tree_lines(&tree, 0, &[]);
    let text: String = lines.iter().map(|l| format!("{l:?}")).collect();
    assert!(text.contains("[MIX]"), "master plan line should contain [MIX] badge: {text}");
}

#[test]
fn mode_badge_sequential_uses_muted_color() {
    use crate::tui::widgets::MUTED;
    let spans = mode_badge_spans(&Some("sequential".into()));
    let badge_span = spans.iter().find(|s| s.content.contains("SEQ"));
    assert!(badge_span.is_some(), "should have a SEQ span");
    let color = badge_span.unwrap().style.fg;
    assert_eq!(color, Some(MUTED), "sequential badge should use MUTED color");
}

#[test]
fn mode_badge_parallel_uses_ok_color() {
    use crate::tui::widgets::OK;
    let spans = mode_badge_spans(&Some("parallel".into()));
    let badge_span = spans.iter().find(|s| s.content.contains("PAR"));
    assert!(badge_span.is_some(), "should have a PAR span");
    let color = badge_span.unwrap().style.fg;
    assert_eq!(color, Some(OK), "parallel badge should use OK color");
}

#[test]
fn mode_badge_mixed_uses_warn_color() {
    use crate::tui::widgets::WARN;
    let spans = mode_badge_spans(&Some("mixed".into()));
    let badge_span = spans.iter().find(|s| s.content.contains("MIX"));
    assert!(badge_span.is_some(), "should have a MIX span");
    let color = badge_span.unwrap().style.fg;
    assert_eq!(color, Some(WARN), "mixed badge should use WARN color");
}

#[test]
fn mode_badge_conditional_uses_accent_color() {
    use crate::tui::widgets::ACCENT;
    let spans = mode_badge_spans(&Some("conditional".into()));
    let badge_span = spans.iter().find(|s| s.content.contains("CND"));
    assert!(badge_span.is_some(), "should have a CND span");
    let color = badge_span.unwrap().style.fg;
    assert_eq!(color, Some(ACCENT), "conditional badge should use ACCENT color");
}
