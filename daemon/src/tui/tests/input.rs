// Command mode and key handling tests.
use super::super::app::TuiApp;
use super::super::input::{handle_key, parse_and_apply_command, DrillDownRequest, InteractiveState};
use super::super::MainView;

#[test]
fn handle_command_slash_enters_command_mode() {
    let mut state = InteractiveState::default();
    assert!(!state.command_mode, "should start outside command mode");
    handle_key(
        crossterm::event::KeyCode::Char('/'),
        crossterm::event::KeyModifiers::NONE,
        &mut state,
    );
    assert!(state.command_mode, "slash must enter command mode");
    assert!(state.command_input.is_empty(), "input starts empty");
}

#[test]
fn handle_command_chars_append_in_command_mode() {
    let mut state = InteractiveState::default();
    state.command_mode = true;
    for ch in ['p', 'l', 'a', 'n'] {
        handle_key(
            crossterm::event::KeyCode::Char(ch),
            crossterm::event::KeyModifiers::NONE,
            &mut state,
        );
    }
    assert_eq!(state.command_input, "plan");
}

#[test]
fn handle_command_backspace_removes_last_char() {
    let mut state = InteractiveState {
        command_mode: true,
        command_input: "pla".into(),
        ..Default::default()
    };
    handle_key(
        crossterm::event::KeyCode::Backspace,
        crossterm::event::KeyModifiers::NONE,
        &mut state,
    );
    assert_eq!(state.command_input, "pl");
}

#[test]
fn handle_command_esc_exits_command_mode() {
    let mut state = InteractiveState {
        command_mode: true,
        command_input: "plan 708".into(),
        ..Default::default()
    };
    handle_key(
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyModifiers::NONE,
        &mut state,
    );
    assert!(!state.command_mode, "Esc must exit command mode");
    assert!(state.command_input.is_empty(), "Esc must clear input");
}

#[test]
fn handle_esc_closes_rich_popup() {
    use super::super::views::PopupContent;
    let mut state = InteractiveState {
        popup_open: true,
        popup_content: Some(PopupContent {
            title: "Detail".to_string(),
            sections: vec![],
            actions: vec![],
        }),
        ..Default::default()
    };
    handle_key(
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyModifiers::NONE,
        &mut state,
    );
    assert!(!state.popup_open, "Esc must close rich popup");
    assert!(state.popup_content.is_none(), "Esc must clear popup content");
}

#[test]
fn handle_esc_clears_plan_filter() {
    let mut state = InteractiveState {
        selected_plan_id: Some(708),
        ..Default::default()
    };
    handle_key(
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyModifiers::NONE,
        &mut state,
    );
    assert!(state.selected_plan_id.is_none(), "Esc must clear plan filter");
}

#[test]
fn parse_command_plan_sets_plan_filter() {
    let mut state = InteractiveState::default();
    let mut view = MainView::PlanKanban;
    parse_and_apply_command("plan 708", &mut state, &mut view);
    assert_eq!(state.selected_plan_id, Some(708));
    assert_eq!(view, MainView::TaskPipeline);
}

#[test]
fn parse_command_mesh_switches_view() {
    let mut state = InteractiveState::default();
    let mut view = MainView::PlanKanban;
    parse_and_apply_command("mesh", &mut state, &mut view);
    assert_eq!(view, MainView::MeshStatus);
}

#[test]
fn parse_command_agent_list_switches_view() {
    let mut state = InteractiveState::default();
    let mut view = MainView::PlanKanban;
    parse_and_apply_command("agent list", &mut state, &mut view);
    assert_eq!(view, MainView::AgentOrgChart);
}

#[test]
fn parse_command_refresh_sets_force_refresh() {
    let mut state = InteractiveState::default();
    let mut view = MainView::PlanKanban;
    parse_and_apply_command("refresh", &mut state, &mut view);
    assert!(state.force_refresh, "refresh command must set force_refresh flag");
}

#[test]
fn parse_command_quit_sets_quit_flag() {
    let mut state = InteractiveState::default();
    let mut view = MainView::PlanKanban;
    parse_and_apply_command("quit", &mut state, &mut view);
    assert!(state.quit, "quit command must set quit flag");
    let mut state2 = InteractiveState::default();
    parse_and_apply_command("q", &mut state2, &mut view);
    assert!(state2.quit, "q command must set quit flag");
}

// --- Refresh controls ---

#[test]
fn auto_refresh_defaults_to_true() {
    // TuiApp::parse_auto_refresh_defaults() is a pure fn we can test without I/O
    assert!(TuiApp::default_auto_refresh(), "auto_refresh must default to true");
}

#[test]
fn default_refresh_interval_is_5s() {
    assert_eq!(TuiApp::default_refresh_interval_secs(), 5u64);
}

#[test]
fn shift_r_toggles_auto_refresh() {
    let mut state = InteractiveState::default();
    // InteractiveState tracks auto_refresh toggle request
    // shift+R sends KeyCode::Char('R') with SHIFT modifier
    handle_key(
        crossterm::event::KeyCode::Char('R'),
        crossterm::event::KeyModifiers::SHIFT,
        &mut state,
    );
    assert!(state.toggle_auto_refresh, "shift+R must set toggle_auto_refresh flag");
}

#[test]
fn plus_key_increases_interval() {
    let mut state = InteractiveState::default();
    handle_key(
        crossterm::event::KeyCode::Char('+'),
        crossterm::event::KeyModifiers::NONE,
        &mut state,
    );
    assert!(state.increase_interval, "'+' must set increase_interval flag");
}

#[test]
fn minus_key_decreases_interval() {
    let mut state = InteractiveState::default();
    handle_key(
        crossterm::event::KeyCode::Char('-'),
        crossterm::event::KeyModifiers::NONE,
        &mut state,
    );
    assert!(state.decrease_interval, "'-' must set decrease_interval flag");
}

#[test]
fn interval_steps_are_correct() {
    // Steps: 3, 5, 10, 30, 60 seconds
    assert_eq!(TuiApp::next_interval(3), 5);
    assert_eq!(TuiApp::next_interval(5), 10);
    assert_eq!(TuiApp::next_interval(10), 30);
    assert_eq!(TuiApp::next_interval(30), 60);
    assert_eq!(TuiApp::next_interval(60), 60); // at max, stays at 60
    assert_eq!(TuiApp::prev_interval(60), 30);
    assert_eq!(TuiApp::prev_interval(30), 10);
    assert_eq!(TuiApp::prev_interval(10), 5);
    assert_eq!(TuiApp::prev_interval(5), 3);
    assert_eq!(TuiApp::prev_interval(3), 3); // at min, stays at 3
}

// --- DrillDownRequest unit tests ---

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
    use super::super::{data::{TaskPipelineItem, TuiData}, input::InteractiveState};
    use super::super::refresh_test_helpers::make_app_with_view;

    let mut app = make_app_with_view(MainView::TaskPipeline);
    app.data.pipeline = vec![
        TaskPipelineItem {
            task_id: "T2-01".to_string(),
            title: "Wire Enter key".to_string(),
            status: "in_progress".to_string(),
            agent: "task-executor".to_string(),
        },
    ];
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
    use super::super::data::{MeshNode, TuiData};
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
    use super::super::data::{AgentOrgNode, TuiData};
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
    // Use sample events from the default data (populated by make_app_with_view via sample_data)
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
    // PlanKanban does not set a drill-down (it navigates instead)
    assert!(app.istate.pending_drill_down.is_none(), "PlanKanban Enter must not set pending_drill_down");
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

    // When no plan_id is selected, falls back to plan_id=0
    match &app.istate.pending_drill_down {
        Some(DrillDownRequest::Task(plan_id, _)) => assert_eq!(*plan_id, 0),
        other => panic!("expected Task drill-down, got: {:?}", other),
    }
}
