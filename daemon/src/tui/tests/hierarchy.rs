// T1-03: Plan hierarchy context tests — TDD RED → GREEN.
// Tests that handle_tree_enter populates hierarchy_context with master name and sibling info.

use super::super::data::{PlanHierarchyContext, ProjectTreeData, ProjectTreeNode};
use super::super::input::InteractiveState;
use super::super::MainView;

/// Build a realistic tree: master "Convergio Vision Master" with two sub-plans.
fn make_tree() -> ProjectTreeData {
    ProjectTreeData {
        project_name: "convergio".into(),
        total_tasks: 15,
        done_tasks: 3,
        plans: vec![
            ProjectTreeNode {
                id: 711,
                name: "Convergio Vision Master".into(),
                status: "doing".into(),
                is_master: true,
                execution_mode: Some("sequential".into()),
                children: vec![
                    ProjectTreeNode {
                        id: 719,
                        name: "Plan H0 Foundation".into(),
                        status: "doing".into(),
                        tasks_done: 3,
                        tasks_total: 8,
                        depends_on: None,
                        ..Default::default()
                    },
                    ProjectTreeNode {
                        id: 712,
                        name: "Plan H Adapters".into(),
                        status: "draft".into(),
                        tasks_done: 0,
                        tasks_total: 7,
                        depends_on: Some("719".into()),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
        ],
    }
}

#[test]
fn handle_tree_enter_child_populates_hierarchy_context() {
    use super::super::refresh_test_helpers::make_app_with_view;

    let mut app = make_app_with_view(MainView::PlanKanban);
    app.data.project_tree = make_tree();
    // Master is at index 0; expand it.
    app.istate.expanded_masters.push(711);
    // First child is at index 1.
    app.selected_index = 1;
    app.handle_tree_enter();

    assert_eq!(app.active_view, MainView::TaskPipeline, "should switch to TaskPipeline");
    assert_eq!(app.istate.selected_plan_id, Some(719));

    let ctx = app.istate.hierarchy_context.as_ref()
        .expect("hierarchy_context must be populated after drill-in");
    assert_eq!(ctx.master_name, "Convergio Vision Master");
    assert_eq!(ctx.master_id, 711);
    assert_eq!(ctx.siblings.len(), 2, "both children must be in siblings list");
}

#[test]
fn hierarchy_context_marks_current_sibling() {
    use super::super::refresh_test_helpers::make_app_with_view;

    let mut app = make_app_with_view(MainView::PlanKanban);
    app.data.project_tree = make_tree();
    app.istate.expanded_masters.push(711);
    // Drill into second child (index 2).
    app.selected_index = 2;
    app.handle_tree_enter();

    let ctx = app.istate.hierarchy_context.as_ref().unwrap();
    assert_eq!(ctx.siblings[0].is_current, false, "first child is not current");
    assert_eq!(ctx.siblings[1].is_current, true, "second child is current");
    assert_eq!(ctx.siblings[1].id, 712);
}

#[test]
fn hierarchy_context_reflects_depends_on() {
    use super::super::refresh_test_helpers::make_app_with_view;

    let mut app = make_app_with_view(MainView::PlanKanban);
    app.data.project_tree = make_tree();
    app.istate.expanded_masters.push(711);
    // Drill into second child (depends_on = "719").
    app.selected_index = 2;
    app.handle_tree_enter();

    let ctx = app.istate.hierarchy_context.as_ref().unwrap();
    // Second sibling has depends_on = Some("719")
    assert_eq!(ctx.siblings[1].depends_on, Some("719".to_string()));
    // First sibling has no dependency.
    assert!(ctx.siblings[0].depends_on.is_none());
}

#[test]
fn esc_clears_hierarchy_context() {
    use crossterm::event::{KeyCode, KeyModifiers};
    use super::super::input::handle_key;

    let ctx = PlanHierarchyContext {
        master_name: "Convergio Vision Master".into(),
        master_id: 711,
        siblings: vec![],
    };
    let mut state = InteractiveState {
        selected_plan_id: Some(719),
        hierarchy_context: Some(ctx),
        ..Default::default()
    };

    handle_key(KeyCode::Esc, KeyModifiers::NONE, &mut state);

    assert!(state.hierarchy_context.is_none(), "Esc must clear hierarchy_context");
    assert!(state.selected_plan_id.is_none(), "Esc must clear selected_plan_id");
}

#[test]
fn hierarchy_context_sibling_counts_and_tasks() {
    use super::super::refresh_test_helpers::make_app_with_view;

    let mut app = make_app_with_view(MainView::PlanKanban);
    app.data.project_tree = make_tree();
    app.istate.expanded_masters.push(711);
    app.selected_index = 1;
    app.handle_tree_enter();

    let ctx = app.istate.hierarchy_context.as_ref().unwrap();
    let first = &ctx.siblings[0];
    assert_eq!(first.id, 719);
    assert_eq!(first.name, "Plan H0 Foundation");
    assert_eq!(first.tasks_done, 3);
    assert_eq!(first.tasks_total, 8);
    assert_eq!(first.status, "doing");
}

#[test]
fn hierarchy_bar_renders_master_name_in_task_pipeline() {
    use ratatui::{backend::TestBackend, Terminal};
    use crate::tui::data::{PlanHierarchyContext, SiblingPlanInfo};
    use crate::tui::views::hierarchy_bar::render_hierarchy_bar;

    let ctx = PlanHierarchyContext {
        master_name: "Convergio Vision Master".into(),
        master_id: 711,
        siblings: vec![
            SiblingPlanInfo {
                id: 719,
                name: "Plan H0 Foundation".into(),
                status: "doing".into(),
                tasks_done: 3,
                tasks_total: 8,
                is_current: true,
                depends_on: None,
            },
            SiblingPlanInfo {
                id: 712,
                name: "Plan H Adapters".into(),
                status: "draft".into(),
                tasks_done: 0,
                tasks_total: 7,
                is_current: false,
                depends_on: Some("719".into()),
            },
        ],
    };

    let backend = TestBackend::new(120, 4);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|frame| {
        render_hierarchy_bar(frame, frame.area(), &ctx);
    }).expect("draw");

    let mut text = String::new();
    for row in terminal.backend().buffer().content.chunks(120) {
        let line = row.iter().map(|c| c.symbol()).collect::<String>();
        text.push_str(&line);
        text.push('\n');
    }

    assert!(text.contains("Convergio Vision Master"), "bar must show master name: {text}");
    assert!(text.contains("Plan H0 Foundation") || text.contains("H0"), "bar must show current sibling: {text}");
}
