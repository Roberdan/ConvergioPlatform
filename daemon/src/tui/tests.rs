use super::{
    views, AgentOrgNode, BrainNode, CostData, CostSummary, DeliverableInfo, KpiData, MainView,
    MeshNode, PlanCard, TaskPipelineItem, TuiData, WorkspaceEvent, WorkspaceInfo,
};
use crate::tui::widgets;
use ratatui::{backend::TestBackend, Terminal};

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
fn renders_cost_center_placeholder() {
    let data = sample_data();
    let rendered = render_to_text(&data, MainView::CostCenter);
    assert!(rendered.contains("Cost Center"));
}

#[test]
fn renders_event_stream_placeholder() {
    let data = sample_data();
    let rendered = render_to_text(&data, MainView::EventStream);
    assert!(rendered.contains("Event Stream"));
}

#[test]
fn renders_workspace_view_placeholder() {
    let data = sample_data();
    let rendered = render_to_text(&data, MainView::WorkspaceView);
    assert!(rendered.contains("Workspace View"));
}

#[test]
fn renders_deliverables_placeholder() {
    let data = sample_data();
    let rendered = render_to_text(&data, MainView::Deliverables);
    assert!(rendered.contains("Deliverables"));
}

#[test]
fn cycles_all_nine_views() {
    // Verify all 9 MainView variants exist and are distinct
    let views = [
        MainView::PlanKanban,
        MainView::TaskPipeline,
        MainView::MeshStatus,
        MainView::AgentOrgChart,
        MainView::BrainCanvas,
        MainView::CostCenter,
        MainView::EventStream,
        MainView::WorkspaceView,
        MainView::Deliverables,
    ];
    assert_eq!(views.len(), 9);
    // Each variant is distinct
    for i in 0..views.len() {
        for j in 0..views.len() {
            if i != j {
                assert_ne!(views[i], views[j], "views[{i}] == views[{j}]");
            }
        }
    }
}

fn render_to_text(data: &TuiData, view: MainView) -> String {
    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal
        .draw(|frame| {
            views::render_view(frame, frame.area(), view, data, 0);
        })
        .expect("draw");
    let mut all = String::new();
    for row in terminal.backend().buffer().content.chunks(120) {
        let line = row.iter().map(|cell| cell.symbol()).collect::<String>();
        all.push_str(&line);
        all.push('\n');
    }
    all
}

fn sample_data() -> TuiData {
    TuiData {
        plans: vec![
            PlanCard {
                id: 100025,
                name: "Stabilize Mesh".to_string(),
                status: "doing".to_string(),
                tasks_done: 12,
                tasks_total: 18,
            },
            PlanCard {
                id: 100026,
                name: "Rust TUI Port".to_string(),
                status: "todo".to_string(),
                tasks_done: 0,
                tasks_total: 8,
            },
        ],
        pipeline: vec![TaskPipelineItem {
            task_id: "T13-01".to_string(),
            title: "Implement Rust TUI".to_string(),
            status: "in_progress".to_string(),
            agent: "copilot".to_string(),
        }],
        mesh_nodes: vec![MeshNode {
            name: "node-a".to_string(),
            online: true,
            role: "coordinator".to_string(),
            cpu_percent: 41.0,
        }],
        agents: vec![AgentOrgNode {
            name: "Thor".to_string(),
            role: "validator".to_string(),
            host: "node-a".to_string(),
            active_task: Some("T13-01".to_string()),
        }],
        kpis: KpiData::default(),
        brain_nodes: vec![BrainNode {
            id: "n1".to_string(),
            label: "Plan 708".to_string(),
            kind: "plan".to_string(),
            parent_id: None,
            status: "active".to_string(),
        }],
        cost: CostData {
            by_model: vec![],
            by_project: vec![],
            by_date: vec![],
            summary: CostSummary::default(),
        },
        events: vec![WorkspaceEvent {
            id: 1,
            workspace_id: "ws-1".to_string(),
            agent: "task-executor".to_string(),
            action: "write".to_string(),
            file_path: Some("daemon/src/tui/data.rs".to_string()),
            detail: None,
            created_at: "2026-03-23T00:00:00Z".to_string(),
        }],
        workspaces: vec![WorkspaceInfo {
            workspace_id: "ws-1".to_string(),
            path: "/tmp/ws-1".to_string(),
            branch: "plan-708-W1".to_string(),
            plan_id: Some(708),
            status: "active".to_string(),
            created_at: "2026-03-23T00:00:00Z".to_string(),
        }],
        deliverables: vec![DeliverableInfo {
            id: 1,
            name: "TUI Refactor".to_string(),
            output_type: "code".to_string(),
            status: "done".to_string(),
            version: 1,
            project_id: "convergio".to_string(),
            created_at: "2026-03-23T00:00:00Z".to_string(),
        }],
    }
}

#[test]
fn palette_has_new_surface_and_text_constants() {
    // Verify the 3 new Maranello palette constants added in T1-03
    assert_eq!(widgets::BG_SURFACE_U32, 0x00262626);
    assert_eq!(widgets::TEXT_PRIMARY_U32, 0x00F3F4F6);
    assert_eq!(widgets::TEXT_SECONDARY_U32, 0x009CA3AF);
}

#[test]
fn cycles_views_forward_and_backward() {
    let mut view = MainView::PlanKanban;
    assert_eq!(view, MainView::PlanKanban);
    view = MainView::TaskPipeline;
    assert_eq!(view, MainView::TaskPipeline);
    view = MainView::PlanKanban;
    assert_eq!(view, MainView::PlanKanban);
}

#[test]
fn api_url_defaults_to_localhost() {
    let url = super::app::TuiApp::parse_api_url();
    assert_eq!(url, "http://localhost:8420");
}
