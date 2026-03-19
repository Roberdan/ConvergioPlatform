//! Integration tests for TUI rendering against real API data.
//! Marked #[ignore] — they bind a real TCP listener and fetch live endpoints.

use claude_core::server::routes::build_router_with_db;
use claude_core::tui::{
    views, AgentOrgNode, KpiData, MainView, MeshNode, PlanCard, TaskPipelineItem, TuiData,
};
use ratatui::{backend::TestBackend, Terminal};
use reqwest::Client;
use serde_json::json;
use std::net::SocketAddr;
use tokio::net::TcpListener;

fn seed_db(path: &std::path::Path) {
    let conn = rusqlite::Connection::open(path).expect("open db");
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         CREATE TABLE projects (id TEXT PRIMARY KEY, name TEXT);
         CREATE TABLE plans (id INTEGER PRIMARY KEY, project_id TEXT DEFAULT '',
             name TEXT DEFAULT '', status TEXT DEFAULT 'todo', source_file TEXT,
             description TEXT, human_summary TEXT, tasks_total INTEGER DEFAULT 0,
             tasks_done INTEGER DEFAULT 0, execution_host TEXT, worktree_path TEXT,
             parallel_mode TEXT, created_at TEXT, started_at TEXT, completed_at TEXT,
             updated_at TEXT, cancelled_at TEXT, cancelled_reason TEXT,
             constraints_json TEXT, is_master INTEGER DEFAULT 0);
         CREATE TABLE waves (id INTEGER PRIMARY KEY, plan_id INTEGER,
             project_id TEXT DEFAULT '', wave_id TEXT, name TEXT,
             status TEXT DEFAULT 'pending', tasks_done INTEGER DEFAULT 0,
             tasks_total INTEGER DEFAULT 0, position INTEGER DEFAULT 0,
             depends_on TEXT, estimated_hours INTEGER DEFAULT 8, worktree_path TEXT,
             started_at TEXT, completed_at TEXT, cancelled_at TEXT,
             cancelled_reason TEXT, merge_mode TEXT DEFAULT 'sync', theme TEXT);
         CREATE TABLE tasks (id INTEGER PRIMARY KEY, project_id TEXT DEFAULT '',
             plan_id INTEGER, wave_id_fk INTEGER, wave_id TEXT, task_id TEXT,
             title TEXT, status TEXT DEFAULT 'pending', priority TEXT, type TEXT,
             assignee TEXT, description TEXT, test_criteria TEXT, model TEXT,
             notes TEXT, tokens INTEGER DEFAULT 0, output_data TEXT,
             executor_host TEXT, started_at TEXT, completed_at TEXT,
             validated_at TEXT, validated_by TEXT, validation_report TEXT);
         CREATE TABLE knowledge_base (id INTEGER PRIMARY KEY, domain TEXT,
             title TEXT, content TEXT, created_at TEXT, hit_count INTEGER DEFAULT 0);
         CREATE TABLE agent_activity (id INTEGER PRIMARY KEY AUTOINCREMENT,
             agent_id TEXT NOT NULL, agent_type TEXT NOT NULL DEFAULT 'legacy',
             model TEXT, description TEXT, status TEXT NOT NULL DEFAULT 'running',
             tokens_in INTEGER DEFAULT 0, tokens_out INTEGER DEFAULT 0,
             tokens_total INTEGER DEFAULT 0, cost_usd REAL DEFAULT 0,
             started_at TEXT DEFAULT (datetime('now')), completed_at TEXT,
             duration_s REAL, host TEXT, region TEXT DEFAULT 'prefrontal',
             metadata TEXT, task_db_id INTEGER, plan_id INTEGER, parent_session TEXT);
         CREATE UNIQUE INDEX IF NOT EXISTS uq_aa ON agent_activity(agent_id);
         CREATE TABLE ipc_agents (name TEXT PRIMARY KEY, host TEXT,
             agent_type TEXT, pid INTEGER, metadata TEXT,
             registered_at TEXT, last_seen TEXT);
         CREATE TABLE peer_heartbeats (peer_name TEXT, last_seen INTEGER,
             load_json TEXT);
         INSERT INTO projects (id, name) VALUES ('convergio', 'Convergio');
         INSERT INTO plans (id, project_id, name, status, tasks_total, tasks_done)
             VALUES (671, 'convergio', 'Dashboard Restructure', 'doing', 5, 2);
         INSERT INTO tasks (id, plan_id, wave_id, task_id, title, status)
             VALUES (1, 671, 'W1', 'T1-01', 'Implement brain strip', 'in_progress');
         INSERT INTO tasks (id, plan_id, wave_id, task_id, title, status)
             VALUES (2, 671, 'W1', 'T1-02', 'Wire KPI endpoints', 'done');
         INSERT INTO agent_activity (agent_id, agent_type, model, status, host, region)
             VALUES ('agent-convergio', 'coordinator', 'claude-opus-4-20250514', 'running',
                     'node-m5-master', 'prefrontal');
         INSERT INTO peer_heartbeats (peer_name, last_seen, load_json)
             VALUES ('node-m5-master', strftime('%s','now'),
                     '{\"cpu_percent\":23.5}');",
    )
    .expect("seed");
}

async fn start_server(db_path: std::path::PathBuf) -> SocketAddr {
    let static_dir = db_path.parent().unwrap().join("static");
    std::fs::create_dir_all(&static_dir).expect("mkdir static");
    let app = build_router_with_db(static_dir, db_path, None);
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    tokio::spawn(async move { axum::serve(listener, app).await.expect("serve") });
    addr
}

/// TUI renders plan data fetched from real API into the kanban view.
#[tokio::test]
#[ignore]
async fn tui_renders_real_api_plan_data() {
    let tmp = tempfile::TempDir::new().expect("tmpdir");
    let db_path = tmp.path().join("test.db");
    seed_db(&db_path);
    let addr = start_server(db_path).await;

    let client = Client::new();
    let base = format!("http://{addr}");
    std::env::set_var("CONVERGIO_API_URL", &base);

    // Fetch plans via TUI API function
    let plans = claude_core::tui::api::fetch_plans(&client).await;
    assert!(!plans.is_empty(), "plans should not be empty");
    assert_eq!(plans[0].name, "Dashboard Restructure");
    assert_eq!(plans[0].status, "doing");
    assert_eq!(plans[0].tasks_done, 2);
    assert_eq!(plans[0].tasks_total, 5);

    // Render kanban with fetched data
    let data = TuiData {
        plans,
        ..TuiData::default()
    };
    let rendered = render_to_text(&data, MainView::PlanKanban);
    assert!(rendered.contains("PLAN KANBAN"), "kanban header missing");
    assert!(
        rendered.contains("Dashboard Restructure"),
        "plan name missing"
    );
}

/// TUI KPI strip populated from real /api/overview endpoint.
#[tokio::test]
#[ignore]
async fn tui_kpi_strip_populated_from_api() {
    let tmp = tempfile::TempDir::new().expect("tmpdir");
    let db_path = tmp.path().join("test.db");
    seed_db(&db_path);
    let addr = start_server(db_path).await;

    let client = Client::new();
    std::env::set_var("CONVERGIO_API_URL", format!("http://{addr}"));

    let kpis = claude_core::tui::api::fetch_overview(&client).await;
    // Seeded DB has 1 active plan, 1 running agent, 1 online peer
    assert!(kpis.plans_active >= 1, "expected at least 1 active plan");

    let data = TuiData {
        kpis: kpis.clone(),
        ..TuiData::default()
    };
    let rendered = render_to_text(&data, MainView::PlanKanban);
    assert!(rendered.contains("Plans:"), "KPI strip missing Plans gauge");
    assert!(
        rendered.contains("Agents:"),
        "KPI strip missing Agents gauge"
    );
    assert!(rendered.contains("Mesh:"), "KPI strip missing Mesh gauge");
}

/// All four TUI views cycle correctly and render without panic.
#[tokio::test]
#[ignore]
async fn tui_views_cycle_with_real_data() {
    let tmp = tempfile::TempDir::new().expect("tmpdir");
    let db_path = tmp.path().join("test.db");
    seed_db(&db_path);
    let addr = start_server(db_path).await;

    let client = Client::new();
    std::env::set_var("CONVERGIO_API_URL", format!("http://{addr}"));

    let (kpis, plans, tasks, mesh, agents) = tokio::join!(
        claude_core::tui::api::fetch_overview(&client),
        claude_core::tui::api::fetch_plans(&client),
        claude_core::tui::api::fetch_all_tasks(&client),
        claude_core::tui::api::fetch_mesh(&client),
        claude_core::tui::api::fetch_agents(&client),
    );

    let data = TuiData {
        kpis,
        plans,
        pipeline: tasks,
        mesh_nodes: mesh,
        agents,
    };

    // Every view renders without panic
    for view in [
        MainView::PlanKanban,
        MainView::TaskPipeline,
        MainView::MeshStatus,
        MainView::AgentOrgChart,
    ] {
        let text = render_to_text(&data, view);
        assert!(!text.trim().is_empty(), "view {:?} rendered empty", view);
    }

    // Verify view-specific content
    let kanban = render_to_text(&data, MainView::PlanKanban);
    assert!(kanban.contains("PLAN KANBAN"));

    let pipeline = render_to_text(&data, MainView::TaskPipeline);
    assert!(pipeline.contains("TASK PIPELINE"));

    let mesh_text = render_to_text(&data, MainView::MeshStatus);
    assert!(mesh_text.contains("MESH STATUS"));

    let org = render_to_text(&data, MainView::AgentOrgChart);
    assert!(org.contains("AGENT ORG CHART"));
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
