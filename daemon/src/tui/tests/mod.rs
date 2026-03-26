// Shared helpers and core tests. Sub-modules hold view/input/integration tests.
mod hierarchy;
mod integration;
mod input;
mod input_actions;
mod input_drilldown;
mod notifications;
mod views;
mod views_tree;

use super::{
    AgentOrgNode, BrainNode, ChatMessage, CostData, CostSummary, DeliverableInfo, KpiData,
    MainView, MeshNode, PlanCard, TaskPipelineItem, TuiData, WorkspaceEvent, WorkspaceInfo,
};
use super::views as tui_views;
use ratatui::{backend::TestBackend, Terminal};

// --- shared helpers (pub(crate) so sub-modules can re-export or use directly) ---

pub(crate) fn render_to_text(data: &TuiData, view: MainView) -> String {
    render_to_text_full(data, view, "http://localhost:8420", false)
}

pub(crate) fn render_to_text_full(
    data: &TuiData,
    view: MainView,
    api_url: &str,
    show_help: bool,
) -> String {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal
        .draw(|frame| {
            tui_views::render_view(
                frame, frame.area(), view, data, 0, api_url, show_help, true, 5, "", false, None, false, 0, &[], None,
            );
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

pub(crate) fn render_to_text_with_refresh(
    data: &TuiData,
    view: MainView,
    auto_refresh: bool,
    refresh_interval_secs: u64,
) -> String {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal
        .draw(|frame| {
            tui_views::render_view(
                frame, frame.area(), view, data, 0, "http://localhost:8420",
                false, auto_refresh, refresh_interval_secs, "", false, None, false, 0, &[], None,
            );
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

pub(crate) fn sample_data() -> TuiData {
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
            workspace_id: "ws-1".into(),
            agent: "task-executor".into(),
            action: "write".into(),
            file_path: Some("daemon/src/tui/data.rs".into()),
            detail: None,
            created_at: "2026-03-23T00:00:00Z".into(),
        }],
        workspaces: vec![WorkspaceInfo {
            workspace_id: "ws-1".into(),
            path: "/tmp/ws-1".into(),
            branch: "plan-708-W1".into(),
            plan_id: Some(708),
            status: "active".into(),
            created_at: "2026-03-23T00:00:00Z".into(),
        }],
        deliverables: vec![DeliverableInfo {
            id: 1,
            name: "TUI Refactor".into(),
            output_type: "code".into(),
            status: "done".into(),
            version: 1,
            project_id: "convergio".into(),
            created_at: "2026-03-23T00:00:00Z".into(),
        }],
        chat_messages: vec![
            ChatMessage {
                role: "user".to_string(),
                content: "What is Plan 708?".to_string(),
                timestamp: "2026-03-24T10:00:00Z".to_string(),
            },
            ChatMessage {
                role: "assistant".to_string(),
                content: "Plan 708 is the TUI refactor.".to_string(),
                timestamp: "2026-03-24T10:00:01Z".to_string(),
            },
        ],
        notifications: vec![],
        project_tree: Default::default(),
        projects: vec![],
        delegations: vec![],
        chat_session_id: Some("sess-test-123".to_string()),
    }
}

// --- core tests ---

#[test]
fn cycles_all_ten_views() {
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
        MainView::Chat,
    ];
    assert_eq!(views.len(), 10);
    for i in 0..views.len() {
        for j in 0..views.len() {
            if i != j {
                assert_ne!(views[i], views[j], "views[{i}] == views[{j}]");
            }
        }
    }
}

#[test]
fn palette_has_new_surface_and_text_constants() {
    use crate::tui::widgets;
    assert_eq!(widgets::BG_SURFACE_U32, 0x00262626);
    assert_eq!(widgets::TEXT_PRIMARY_U32, 0x00F3F4F6);
    assert_eq!(widgets::TEXT_SECONDARY_U32, 0x009CA3AF);
}

#[test]
fn api_url_defaults_to_localhost() {
    assert_eq!(super::app::TuiApp::parse_api_url(), "http://localhost:8420");
}
