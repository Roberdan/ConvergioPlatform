// View rendering tests — all renders_* and tab/status bar assertions.
use super::super::{BrainNode, MainView};
use super::{render_to_text, render_to_text_full, sample_data};

#[test]
fn renders_plan_kanban_view() {
    let data = sample_data();
    let rendered = render_to_text(&data, MainView::PlanKanban);
    assert!(rendered.contains("PLAN KANBAN"));
    assert!(rendered.contains("Stabilize Mesh"));
}

#[test]
fn renders_task_pipeline_view() {
    let data = sample_data();
    let rendered = render_to_text(&data, MainView::TaskPipeline);
    assert!(rendered.contains("TASK PIPELINE"));
    assert!(rendered.contains("T13-01"));
}

#[test]
fn renders_mesh_status_view() {
    let data = sample_data();
    let rendered = render_to_text(&data, MainView::MeshStatus);
    assert!(rendered.contains("MESH STATUS"));
    assert!(rendered.contains("node-a"));
}

#[test]
fn renders_agent_org_chart_view() {
    let data = sample_data();
    let rendered = render_to_text(&data, MainView::AgentOrgChart);
    assert!(rendered.contains("AGENT ORG CHART"));
    assert!(rendered.contains("Thor"));
}

#[test]
fn renders_brain_canvas_placeholder() {
    let data = sample_data();
    let rendered = render_to_text(&data, MainView::BrainCanvas);
    assert!(rendered.contains("Brain Canvas"));
}

#[test]
fn renders_brain_canvas_with_nodes() {
    let mut data = sample_data();
    data.brain_nodes = vec![
        BrainNode {
            id: "s1".to_string(),
            label: "session-alpha".to_string(),
            kind: "session".to_string(),
            parent_id: None,
            status: "running".to_string(),
        },
        BrainNode {
            id: "a1".to_string(),
            label: "agent-thor".to_string(),
            kind: "agent".to_string(),
            parent_id: Some("s1".to_string()),
            status: "running".to_string(),
        },
        BrainNode {
            id: "t1".to_string(),
            label: "task-T2-02".to_string(),
            kind: "task".to_string(),
            parent_id: Some("a1".to_string()),
            status: "submitted".to_string(),
        },
    ];
    let rendered = render_to_text(&data, MainView::BrainCanvas);
    assert!(rendered.contains("BRAIN CANVAS"), "missing BRAIN CANVAS header");
    assert!(rendered.contains("session-alpha"), "missing session node label");
    assert!(rendered.contains("agent-thor"), "missing agent node label");
    assert!(rendered.contains("task-T2-02"), "missing task node label");
}

#[test]
fn renders_brain_canvas_empty_state() {
    let mut data = sample_data();
    data.brain_nodes = vec![];
    let rendered = render_to_text(&data, MainView::BrainCanvas);
    assert!(rendered.contains("BRAIN CANVAS"), "missing BRAIN CANVAS header");
    assert!(rendered.contains("No brain data"), "missing empty-state message");
}

#[test]
fn renders_cost_center_placeholder() {
    let data = sample_data();
    let rendered = render_to_text(&data, MainView::CostCenter);
    assert!(rendered.contains("Cost Center"));
}

#[test]
fn renders_event_stream_with_events() {
    let data = sample_data();
    let rendered = render_to_text(&data, MainView::EventStream);
    assert!(rendered.contains("EVENTS"), "Missing EVENTS header");
}

#[test]
fn renders_event_stream_empty_state() {
    let mut data = sample_data();
    data.events = vec![];
    let rendered = render_to_text(&data, MainView::EventStream);
    assert!(rendered.contains("No events"), "Missing empty state message");
}

#[test]
fn renders_workspace_view_placeholder() {
    let data = sample_data();
    let rendered = render_to_text(&data, MainView::WorkspaceView);
    assert!(
        rendered.contains("Workspaces") || rendered.contains("WS"),
        "workspace view must render tab or content header"
    );
}

#[test]
fn renders_deliverables_placeholder() {
    let data = sample_data();
    let rendered = render_to_text(&data, MainView::Deliverables);
    assert!(rendered.contains("Deliverables"));
}

#[test]
fn tab_bar_shows_all_nine_views() {
    let data = sample_data();
    let rendered = render_to_text(&data, MainView::PlanKanban);
    assert!(rendered.contains("Kanban"), "missing Kanban tab");
    assert!(rendered.contains("Pipeline"), "missing Pipeline tab");
    assert!(rendered.contains("Mesh"), "missing Mesh tab");
    assert!(rendered.contains("Agents"), "missing Agents tab");
    assert!(rendered.contains("Brain"), "missing Brain tab");
    assert!(rendered.contains("Cost"), "missing Cost tab");
    assert!(rendered.contains("Events"), "missing Events tab");
    assert!(rendered.contains("WS"), "missing WS tab");
    assert!(rendered.contains("Deliv"), "missing Deliv tab");
}

#[test]
fn status_bar_shows_api_url() {
    let data = sample_data();
    let rendered = render_to_text_full(&data, MainView::PlanKanban, "http://testhost:9000", false);
    assert!(rendered.contains("testhost:9000"), "status bar must show api_url host");
}

#[test]
fn status_bar_shows_navigate_hint() {
    let data = sample_data();
    let rendered = render_to_text(&data, MainView::PlanKanban);
    assert!(
        rendered.contains("Navigate") || rendered.contains("navigate"),
        "status bar must show navigate hint"
    );
}

#[test]
fn render_view_accepts_api_url_and_show_help_params() {
    let data = sample_data();
    let rendered = render_to_text_full(&data, MainView::PlanKanban, "http://localhost:8420", false);
    assert!(rendered.contains("PLAN KANBAN"));
}
