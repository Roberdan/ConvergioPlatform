// DrillDownRequest unit tests and handle_enter drill-down tests.
use super::super::input::{DrillDownRequest, InteractiveState};
use super::super::MainView;

#[test]
fn drill_down_request_plan_variant_stores_id() {
    let req = DrillDownRequest::Plan(709);
    match req {
        DrillDownRequest::Plan(id) => assert_eq!(id, 709),
        _ => panic!("wrong variant"),
    }
}

#[test]
fn drill_down_request_task_variant_stores_plan_and_index() {
    let req = DrillDownRequest::Task(709, 3);
    match req {
        DrillDownRequest::Task(plan_id, idx) => {
            assert_eq!(plan_id, 709);
            assert_eq!(idx, 3);
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn drill_down_request_mesh_node_stores_name() {
    let req = DrillDownRequest::MeshNode("macProM1".to_string());
    match req {
        DrillDownRequest::MeshNode(name) => assert_eq!(name, "macProM1"),
        _ => panic!("wrong variant"),
    }
}

#[test]
fn drill_down_request_agent_stores_name() {
    let req = DrillDownRequest::Agent("Thor".to_string());
    match req {
        DrillDownRequest::Agent(name) => assert_eq!(name, "Thor"),
        _ => panic!("wrong variant"),
    }
}

#[test]
fn drill_down_request_event_stores_index() {
    let req = DrillDownRequest::Event(5);
    match req {
        DrillDownRequest::Event(idx) => assert_eq!(idx, 5),
        _ => panic!("wrong variant"),
    }
}

#[test]
fn drill_down_request_workspace_stores_id() {
    let req = DrillDownRequest::Workspace("ws-abc".to_string());
    match req {
        DrillDownRequest::Workspace(id) => assert_eq!(id, "ws-abc"),
        _ => panic!("wrong variant"),
    }
}

#[test]
fn drill_down_request_deliverable_stores_id() {
    let req = DrillDownRequest::Deliverable(42);
    match req {
        DrillDownRequest::Deliverable(id) => assert_eq!(id, 42),
        _ => panic!("wrong variant"),
    }
}

#[test]
fn interactive_state_pending_drill_down_defaults_none() {
    let state = InteractiveState::default();
    assert!(state.pending_drill_down.is_none());
}

// --- handle_enter drill-down tests (via TuiApp) ---

#[test]
fn enter_on_task_pipeline_sets_drill_down_task() {
    use super::super::data::TaskPipelineItem;
    use super::super::refresh_test_helpers::make_app_with_view;

    let mut app = make_app_with_view(MainView::TaskPipeline);
    app.data.pipeline = vec![TaskPipelineItem {
        task_id: "T2-01".to_string(),
        title: "Wire Enter key".to_string(),
        status: "in_progress".to_string(),
        agent: "task-executor".to_string(),
    }];
    app.istate.selected_plan_id = Some(709);
    app.selected_index = 0;
    app.handle_enter();

    match &app.istate.pending_drill_down {
        Some(DrillDownRequest::Task(plan_id, idx)) => {
            assert_eq!(*plan_id, 709);
            assert_eq!(*idx, 0);
        }
        other => panic!("expected Task drill-down, got: {:?}", other),
    }
}

#[test]
fn enter_on_mesh_sets_drill_down_mesh_node() {
    use super::super::data::MeshNode;
    use super::super::refresh_test_helpers::make_app_with_view;

    let mut app = make_app_with_view(MainView::MeshStatus);
    app.data.mesh_nodes = vec![MeshNode {
        name: "macProM1".to_string(),
        online: true,
        role: "coordinator".to_string(),
        cpu_percent: 30.0,
    }];
    app.selected_index = 0;
    app.handle_enter();

    match &app.istate.pending_drill_down {
        Some(DrillDownRequest::MeshNode(name)) => assert_eq!(name, "macProM1"),
        other => panic!("expected MeshNode drill-down, got: {:?}", other),
    }
}

#[test]
fn enter_on_agent_sets_drill_down_agent() {
    use super::super::data::AgentOrgNode;
    use super::super::refresh_test_helpers::make_app_with_view;

    let mut app = make_app_with_view(MainView::AgentOrgChart);
    app.data.agents = vec![AgentOrgNode {
        name: "Thor".to_string(),
        role: "validator".to_string(),
        host: "node-a".to_string(),
        active_task: None,
    }];
    app.selected_index = 0;
    app.handle_enter();

    match &app.istate.pending_drill_down {
        Some(DrillDownRequest::Agent(name)) => assert_eq!(name, "Thor"),
        other => panic!("expected Agent drill-down, got: {:?}", other),
    }
}

#[test]
fn enter_on_event_stream_sets_drill_down_event() {
    use super::super::refresh_test_helpers::make_app_with_view;

    let mut app = make_app_with_view(MainView::EventStream);
    app.selected_index = 0;
    app.handle_enter();

    match &app.istate.pending_drill_down {
        Some(DrillDownRequest::Event(idx)) => assert_eq!(*idx, 0),
        other => panic!("expected Event drill-down, got: {:?}", other),
    }
}

#[test]
fn enter_on_workspace_sets_drill_down_workspace() {
    use super::super::refresh_test_helpers::make_app_with_view;

    let mut app = make_app_with_view(MainView::WorkspaceView);
    app.selected_index = 0;
    app.handle_enter();

    match &app.istate.pending_drill_down {
        Some(DrillDownRequest::Workspace(id)) => assert_eq!(id, "ws-1"),
        other => panic!("expected Workspace drill-down, got: {:?}", other),
    }
}

#[test]
fn enter_on_deliverables_sets_drill_down_deliverable() {
    use super::super::refresh_test_helpers::make_app_with_view;

    let mut app = make_app_with_view(MainView::Deliverables);
    app.selected_index = 0;
    app.handle_enter();

    match &app.istate.pending_drill_down {
        Some(DrillDownRequest::Deliverable(id)) => assert_eq!(*id, 1),
        other => panic!("expected Deliverable drill-down, got: {:?}", other),
    }
}

#[test]
fn enter_on_brain_canvas_does_nothing() {
    use super::super::refresh_test_helpers::make_app_with_view;

    let mut app = make_app_with_view(MainView::BrainCanvas);
    app.handle_enter();
    assert!(app.istate.pending_drill_down.is_none(), "BrainCanvas enter must be no-op");
    assert!(!app.istate.popup_open, "BrainCanvas enter must not open popup");
}

#[test]
fn enter_on_plan_kanban_switches_to_task_pipeline() {
    use super::super::refresh_test_helpers::make_app_with_view;

    let mut app = make_app_with_view(MainView::PlanKanban);
    app.selected_index = 0;
    app.handle_enter();

    assert_eq!(app.active_view, MainView::TaskPipeline, "PlanKanban Enter must switch view");
    assert!(app.istate.selected_plan_id.is_some(), "PlanKanban Enter must set selected_plan_id");
    assert!(
        app.istate.pending_drill_down.is_none(),
        "PlanKanban Enter must not set pending_drill_down"
    );
}

#[test]
fn enter_on_task_pipeline_no_plan_id_sets_plan_0() {
    use super::super::data::TaskPipelineItem;
    use super::super::refresh_test_helpers::make_app_with_view;

    let mut app = make_app_with_view(MainView::TaskPipeline);
    app.data.pipeline = vec![TaskPipelineItem {
        task_id: "T2-01".to_string(),
        title: "Test".to_string(),
        status: "done".to_string(),
        agent: "executor".to_string(),
    }];
    app.istate.selected_plan_id = None;
    app.selected_index = 0;
    app.handle_enter();

    match &app.istate.pending_drill_down {
        Some(DrillDownRequest::Task(plan_id, _)) => assert_eq!(*plan_id, 0),
        other => panic!("expected Task drill-down, got: {:?}", other),
    }
}
