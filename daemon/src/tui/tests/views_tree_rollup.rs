// T1-04: rollup progress bar tests for the project tree view.
use super::super::{ProjectTreeData, ProjectTreeNode};
use crate::tui::views::project_tree::build_tree_lines;

// Helper: collapse Line spans into a single string for assertion.
fn lines_to_text(lines: &[ratatui::text::Line<'_>]) -> String {
    lines
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

// Build a master plan with three children whose totals sum to 8/20.
fn tree_with_rollup_children() -> ProjectTreeData {
    ProjectTreeData {
        project_name: "convergio".into(),
        total_tasks: 20,
        done_tasks: 8,
        plans: vec![ProjectTreeNode {
            id: 800,
            name: "Platform Roadmap Q2".into(),
            status: "doing".into(),
            is_master: true,
            execution_mode: None,
            children: vec![
                ProjectTreeNode {
                    id: 801,
                    name: "Auth Hardening".into(),
                    status: "done".into(),
                    tasks_done: 5,
                    tasks_total: 5,
                    ..Default::default()
                },
                ProjectTreeNode {
                    id: 802,
                    name: "Mesh Resilience".into(),
                    status: "doing".into(),
                    tasks_done: 3,
                    tasks_total: 10,
                    ..Default::default()
                },
                ProjectTreeNode {
                    id: 803,
                    name: "Dashboard Refresh".into(),
                    status: "todo".into(),
                    tasks_done: 0,
                    tasks_total: 5,
                    ..Default::default()
                },
            ],
            ..Default::default()
        }],
    }
}

// Rollup: 5+3+0 = 8 done, 5+10+5 = 20 total => 40%
#[test]
fn master_line_shows_percentage() {
    let tree = tree_with_rollup_children();
    let (lines, _) = build_tree_lines(&tree, 0, &[]);
    let master_line = lines
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect::<String>())
        .find(|l| l.contains("Platform Roadmap Q2"))
        .expect("master plan line not found");
    assert!(
        master_line.contains("(40%)"),
        "master plan line must show aggregate percentage (40%): {master_line}"
    );
}

#[test]
fn rollup_line_appears_when_expanded() {
    let tree = tree_with_rollup_children();
    let (lines, _) = build_tree_lines(&tree, 0, &[800]);
    let text = lines_to_text(&lines);
    assert!(
        text.contains("Rollup:"),
        "expanded master must have a Rollup summary line: {text}"
    );
}

#[test]
fn rollup_matches_sum_of_children() {
    let tree = tree_with_rollup_children();
    let (lines, _) = build_tree_lines(&tree, 0, &[800]);
    let rollup_line = lines
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect::<String>())
        .find(|l| l.contains("Rollup:"))
        .expect("Rollup line not found");
    assert!(
        rollup_line.contains("8/20") && rollup_line.contains("40%"),
        "rollup must show 8/20 (40%): {rollup_line}"
    );
}

#[test]
fn master_line_zero_total_shows_zero_percent() {
    let tree = ProjectTreeData {
        project_name: "convergio".into(),
        total_tasks: 0,
        done_tasks: 0,
        plans: vec![ProjectTreeNode {
            id: 900,
            name: "Empty Roadmap".into(),
            status: "todo".into(),
            is_master: true,
            children: vec![],
            ..Default::default()
        }],
    };
    let (lines, _) = build_tree_lines(&tree, 0, &[]);
    let master_line = lines
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect::<String>())
        .find(|l| l.contains("Empty Roadmap"))
        .expect("master plan line not found");
    assert!(
        master_line.contains("(0%)"),
        "master with 0 tasks must show (0%) on master line: {master_line}"
    );
}
