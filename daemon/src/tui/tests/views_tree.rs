// Project tree view tests — build_tree_lines assertions.
use super::super::{ProjectTreeData, ProjectTreeNode};
use crate::tui::views::project_tree::build_tree_lines;

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
