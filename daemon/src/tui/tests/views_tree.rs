// Project tree view tests — build_tree_lines assertions.
use super::super::{ProjectTreeData, ProjectTreeNode};
use crate::tui::views::project_tree::{build_tree_lines, mode_badge_spans};

fn sample_tree() -> ProjectTreeData {
    ProjectTreeData {
        project_name: "convergio".into(),
        total_tasks: 100,
        done_tasks: 50,
        plans: vec![
            ProjectTreeNode {
                id: 711, name: "Convergio Vision".into(), status: "draft".into(),
                is_master: true, execution_mode: Some("mixed".into()),
                children: vec![
                    ProjectTreeNode {
                        id: 719, name: "Plan H0".into(), status: "doing".into(),
                        tasks_done: 3, tasks_total: 8, ..Default::default()
                    },
                    ProjectTreeNode {
                        id: 712, name: "Plan H".into(), status: "draft".into(),
                        tasks_done: 0, tasks_total: 7,
                        depends_on: Some("719".into()), ..Default::default()
                    },
                ],
                ..Default::default()
            },
            ProjectTreeNode {
                id: 123, name: "Old Plan".into(), status: "done".into(),
                tasks_done: 5, tasks_total: 5, ..Default::default()
            },
        ],
    }
}

fn tree_with_three_children() -> ProjectTreeData {
    ProjectTreeData {
        project_name: "alpha".into(),
        total_tasks: 30,
        done_tasks: 10,
        plans: vec![ProjectTreeNode {
            id: 1,
            name: "Master Alpha".into(),
            status: "doing".into(),
            is_master: true,
            children: vec![
                ProjectTreeNode {
                    id: 2, name: "Child One".into(), status: "done".into(),
                    tasks_done: 5, tasks_total: 5, ..Default::default()
                },
                ProjectTreeNode {
                    id: 3, name: "Child Two".into(), status: "doing".into(),
                    tasks_done: 3, tasks_total: 10,
                    depends_on: Some("Child One".into()), ..Default::default()
                },
                ProjectTreeNode {
                    id: 4, name: "Child Three".into(), status: "blocked".into(),
                    tasks_done: 0, tasks_total: 5, ..Default::default()
                },
            ],
            ..Default::default()
        }],
    }
}

// Helper: collapse Line spans into a single string for assertion.
fn lines_to_text(lines: &[ratatui::text::Line<'_>]) -> String {
    lines
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

// ---- existing tests (kept) ----

#[test]
fn tree_counts_selectable_items_collapsed() {
    let tree = sample_tree();
    let (_lines, count) = build_tree_lines(&tree, 0, &[]);
    assert_eq!(count, 2); // master + orphan
}

#[test]
fn tree_counts_selectable_items_expanded() {
    let tree = sample_tree();
    let (_lines, count) = build_tree_lines(&tree, 0, &[711]);
    assert_eq!(count, 4); // master + 2 children + orphan
}

#[test]
fn tree_empty_shows_placeholder() {
    let tree = ProjectTreeData::default();
    let (lines, count) = build_tree_lines(&tree, 0, &[]);
    assert_eq!(count, 0);
    let text: String = lines.iter().map(|l| format!("{l:?}")).collect();
    assert!(text.contains("No project data"), "should show placeholder: {text}");
}

// ---- new tests for T1-02 requirements ----

/// Master with 3 children shows ├── for middle children and └── for the last.
#[test]
fn tree_three_children_uses_branch_characters() {
    let tree = tree_with_three_children();
    let (lines, _) = build_tree_lines(&tree, 0, &[1]);
    let text = lines_to_text(&lines);
    assert!(text.contains("\u{251c}\u{2500}\u{2500}"), "should contain ├── for non-last child: {text}");
    assert!(text.contains("\u{2514}\u{2500}\u{2500}"), "should contain └── for last child: {text}");
}

/// depends_on label renders as `→ depends on: <name>`.
#[test]
fn tree_depends_on_label_format() {
    let tree = sample_tree();
    let (lines, _) = build_tree_lines(&tree, 0, &[711]);
    let text = lines_to_text(&lines);
    assert!(
        text.contains("\u{2192} depends on: 719"),
        "depends_on should render as '→ depends on: <name>': {text}"
    );
}

/// Status colors: done=OK, doing=WARN, blocked=FAIL verified via status_color logic.
#[test]
fn tree_status_icons_reflect_state() {
    let tree = tree_with_three_children();
    let (lines, _) = build_tree_lines(&tree, 0, &[1]);
    let text = lines_to_text(&lines);
    assert!(text.contains("\u{2713}"), "done child should show ✓ icon");
    assert!(text.contains("\u{25c9}"), "doing child should show ◉ icon");
    assert!(text.contains("\u{2715}"), "blocked child should show ✕ icon");
}

/// Tasks progress appears as [N/M] bracket notation next to each child node.
#[test]
fn tree_child_progress_shown_as_bracketed_fraction() {
    let tree = tree_with_three_children();
    let (lines, _) = build_tree_lines(&tree, 0, &[1]);
    let text = lines_to_text(&lines);
    assert!(text.contains("[5/5]"), "child progress should use [N/M] format: {text}");
    assert!(text.contains("[3/10]"), "child progress should use [N/M] format: {text}");
}

/// A master plan with no children renders only the master node (count=1).
#[test]
fn tree_master_with_no_children_renders_just_root() {
    let tree = ProjectTreeData {
        project_name: "solo".into(),
        total_tasks: 5,
        done_tasks: 1,
        plans: vec![ProjectTreeNode {
            id: 99,
            name: "Lone Master".into(),
            status: "doing".into(),
            is_master: true,
            children: vec![],
            ..Default::default()
        }],
    };
    let (lines, count) = build_tree_lines(&tree, 0, &[99]);
    assert_eq!(count, 1, "only master is selectable");
    let text = lines_to_text(&lines);
    assert!(text.contains("Lone Master"), "master name should be present: {text}");
    assert!(
        !text.contains("\u{251c}\u{2500}\u{2500}") && !text.contains("\u{2514}\u{2500}\u{2500}"),
        "no branch chars expected with empty children: {text}"
    );
}
