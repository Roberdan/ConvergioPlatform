//! Integration tests for all dashboard API endpoints.
//! Uses temp SQLite with seeded data to verify response shapes match frontend JS.
mod plan_tests;
mod resource_tests;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

pub(super) fn test_router() -> axum::Router {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let tmp = std::env::temp_dir().join(format!("claude-test-{}-{n}.db", std::process::id()));
    let conn = rusqlite::Connection::open(&tmp).expect("open");
    conn.execute_batch(CORE_SCHEMA).expect("core schema");
    conn.execute_batch(SEED_DATA).expect("seed data");
    drop(conn);
    super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp, None)
}

pub(super) const CORE_SCHEMA: &str = "
PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS projects (
  id TEXT PRIMARY KEY, name TEXT NOT NULL, path TEXT NOT NULL,
  branch TEXT DEFAULT 'main', created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS plans (
  id INTEGER PRIMARY KEY AUTOINCREMENT, project_id TEXT NOT NULL,
  name TEXT NOT NULL, source_file TEXT, status TEXT NOT NULL DEFAULT 'todo'
    CHECK(status IN ('todo','doing','done','cancelled')),
  tasks_total INTEGER DEFAULT 0, tasks_done INTEGER DEFAULT 0,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP, started_at DATETIME,
  completed_at DATETIME, execution_host TEXT, human_summary TEXT,
  parallel_mode TEXT DEFAULT 'standard', lines_added INTEGER,
  lines_removed INTEGER, cancelled_at DATETIME, description TEXT,
  FOREIGN KEY (project_id) REFERENCES projects(id)
);
CREATE TABLE IF NOT EXISTS waves (
  id INTEGER PRIMARY KEY AUTOINCREMENT, plan_id INTEGER NOT NULL,
  project_id TEXT, wave_id TEXT NOT NULL, name TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending'
    CHECK(status IN ('pending','in_progress','done','blocked','merging','cancelled')),
  tasks_done INTEGER DEFAULT 0, tasks_total INTEGER DEFAULT 0,
  position INTEGER DEFAULT 0, started_at DATETIME, completed_at DATETIME,
  cancelled_at DATETIME, pr_number INTEGER, pr_url TEXT,
  theme TEXT, depends_on TEXT,
  FOREIGN KEY (plan_id) REFERENCES plans(id)
);
CREATE TABLE IF NOT EXISTS tasks (
  id INTEGER PRIMARY KEY AUTOINCREMENT, project_id TEXT NOT NULL,
  wave_id TEXT NOT NULL, task_id TEXT NOT NULL, title TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending'
    CHECK(status IN ('pending','in_progress','submitted','done','blocked','skipped','cancelled')),
  tokens INTEGER DEFAULT 0, validated_at DATETIME, validated_by TEXT,
  notes TEXT, wave_id_fk INTEGER, plan_id INTEGER REFERENCES plans(id),
  model TEXT DEFAULT 'haiku', output_data TEXT, executor_agent TEXT,
  executor_host TEXT, started_at DATETIME, completed_at DATETIME,
  validation_report TEXT, duration_minutes REAL,
  FOREIGN KEY (project_id) REFERENCES projects(id)
);
CREATE TABLE IF NOT EXISTS peer_heartbeats (
  peer_name TEXT PRIMARY KEY, last_seen INTEGER NOT NULL,
  load_json TEXT, capabilities TEXT,
  updated_at TEXT DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS token_usage (
  id INTEGER PRIMARY KEY AUTOINCREMENT, project_id TEXT, plan_id INTEGER,
  wave_id TEXT, task_id TEXT, agent TEXT, model TEXT,
  input_tokens INTEGER DEFAULT 0, output_tokens INTEGER DEFAULT 0,
  cost_usd REAL DEFAULT 0, created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  execution_host TEXT
);
CREATE TABLE IF NOT EXISTS notifications (
  id INTEGER PRIMARY KEY AUTOINCREMENT, project_id TEXT,
  type TEXT NOT NULL, title TEXT NOT NULL, message TEXT NOT NULL,
  source TEXT, link TEXT, link_type TEXT, is_read INTEGER DEFAULT 0,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP, read_at DATETIME
);
CREATE TABLE IF NOT EXISTS nightly_jobs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  run_id TEXT, job_name TEXT DEFAULT 'guardian',
  started_at DATETIME DEFAULT CURRENT_TIMESTAMP, finished_at DATETIME,
  host TEXT, status TEXT NOT NULL
    CHECK(status IN ('running','ok','action_required','failed')),
  sentry_unresolved INTEGER DEFAULT 0, github_open_issues INTEGER DEFAULT 0,
  processed_items INTEGER DEFAULT 0, fixed_items INTEGER DEFAULT 0,
  branch_name TEXT, pr_url TEXT, summary TEXT, report_json TEXT,
  log_stdout TEXT, log_stderr TEXT, log_file_path TEXT, duration_sec INTEGER,
  config_snapshot TEXT, exit_code INTEGER, error_detail TEXT,
  trigger_source TEXT DEFAULT 'scheduled', parent_run_id TEXT
);
CREATE TABLE IF NOT EXISTS nightly_job_definitions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id TEXT DEFAULT 'mirrorbuddy',
  name TEXT NOT NULL UNIQUE, description TEXT,
  schedule TEXT NOT NULL DEFAULT '0 3 * * *',
  script_path TEXT NOT NULL, target_host TEXT DEFAULT 'local',
  enabled INTEGER NOT NULL DEFAULT 1,
  run_fixes INTEGER DEFAULT 1, timeout_sec INTEGER DEFAULT 5400,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS mesh_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT, event_type TEXT NOT NULL,
  plan_id INTEGER, source_peer TEXT NOT NULL, payload TEXT,
  status TEXT DEFAULT 'pending', created_at INTEGER DEFAULT (unixepoch()),
  delivered_at INTEGER
);
";

pub(super) const SEED_DATA: &str = "
INSERT INTO projects(id,name,path) VALUES('proj1','TestProject','/tmp/test');
INSERT INTO projects(id,name,path) VALUES('proj2','AnotherProject','/tmp/other');

-- Active plan (doing) with waves and tasks
INSERT INTO plans(id,project_id,name,status,tasks_done,tasks_total,human_summary,execution_host)
  VALUES(1,'proj1','Active Plan Alpha','doing',2,5,'Test plan summary','mac-worker-2');
INSERT INTO waves(id,plan_id,project_id,wave_id,name,status,tasks_done,tasks_total,position,completed_at)
  VALUES(10,1,'proj1','W0','Foundation','done',2,2,1,'2026-03-01 12:00:00');
INSERT INTO waves(id,plan_id,project_id,wave_id,name,status,tasks_done,tasks_total,position)
  VALUES(11,1,'proj1','W1','Core Logic','in_progress',0,3,2);
INSERT INTO tasks(id,project_id,plan_id,wave_id_fk,wave_id,task_id,title,status,executor_agent,model,validated_at)
  VALUES(100,'proj1',1,10,'W0','T0-01','Setup monorepo','done','claude','opus','2026-03-01 12:00:00');
INSERT INTO tasks(id,project_id,plan_id,wave_id_fk,wave_id,task_id,title,status,executor_agent,model,validated_at)
  VALUES(101,'proj1',1,10,'W0','T0-02','Add CI pipeline','done','copilot','gpt-5.3','2026-03-01 12:30:00');
INSERT INTO tasks(id,project_id,plan_id,wave_id_fk,wave_id,task_id,title,status,model)
  VALUES(102,'proj1',1,11,'W1','T1-01','Implement auth','pending','haiku');
INSERT INTO tasks(id,project_id,plan_id,wave_id_fk,wave_id,task_id,title,status,model)
  VALUES(103,'proj1',1,11,'W1','T1-02','Implement API','in_progress','opus');
INSERT INTO tasks(id,project_id,plan_id,wave_id_fk,wave_id,task_id,title,status,model)
  VALUES(104,'proj1',1,11,'W1','T1-03','Add tests','blocked','haiku');

-- Done plan
INSERT INTO plans(id,project_id,name,status,tasks_done,tasks_total,completed_at,lines_added,lines_removed)
  VALUES(2,'proj2','Completed Plan Beta','done',10,10,datetime('now','-3 hours'),500,120);
INSERT INTO waves(id,plan_id,project_id,wave_id,name,status,tasks_done,tasks_total,position,completed_at)
  VALUES(20,2,'proj2','W0','All work','done',10,10,1,datetime('now','-3 hours'));

-- Done test plan (must be excluded from recent missions)
INSERT INTO plans(id,project_id,name,status,tasks_done,tasks_total,completed_at)
  VALUES(6,'proj1','Test Mission Zeta','done',1,1,datetime('now','-2 hours'));

INSERT INTO projects(id,name,path) VALUES('proj3','HyperDemo','/tmp/hyperdemo');
INSERT INTO plans(id,project_id,name,status,tasks_done,tasks_total,completed_at)
  VALUES(7,'proj3','HyperDemo Launch','done',3,3,datetime('now','-1 hours'));

-- Cancelled plan (parking lot)
INSERT INTO plans(id,project_id,name,status,tasks_done,tasks_total,cancelled_at)
  VALUES(3,'proj1','Cancelled Plan Gamma','cancelled',0,8,datetime('now','-12 hours'));
INSERT INTO waves(id,plan_id,project_id,wave_id,name,status,tasks_done,tasks_total,position)
  VALUES(30,3,'proj1','W0','Never started','cancelled',0,4,1);
INSERT INTO tasks(id,project_id,plan_id,wave_id_fk,wave_id,task_id,title,status)
  VALUES(300,'proj1',3,30,'W0','T0-01','Task A','cancelled');

-- Todo plan
INSERT INTO plans(id,project_id,name,status,tasks_done,tasks_total)
  VALUES(4,'proj1','Pipeline Plan Delta','todo',0,12);

-- Submitted task (for Thor validate test)
INSERT INTO plans(id,project_id,name,status,tasks_done,tasks_total)
  VALUES(5,'proj1','Thor Test Plan','doing',0,1);
INSERT INTO waves(id,plan_id,project_id,wave_id,name,status,tasks_done,tasks_total,position)
  VALUES(50,5,'proj1','W0','Wave','in_progress',0,1,1);
INSERT INTO tasks(id,project_id,plan_id,wave_id_fk,wave_id,task_id,title,status)
  VALUES(500,'proj1',5,50,'W0','T0-01','Submitted task','submitted');

-- Peers
INSERT INTO peer_heartbeats(peer_name,last_seen,load_json,capabilities)
  VALUES('mac-worker-2',strftime('%s','now'),'{\"cpu\":15.2,\"mem_total_gb\":36,\"mem_used_gb\":22}','claude,copilot');
INSERT INTO peer_heartbeats(peer_name,last_seen,load_json,capabilities)
  VALUES('linux-worker',strftime('%s','now')-600,'{\"cpu\":5.0,\"mem_total_gb\":16,\"mem_used_gb\":8}','claude,ollama');

-- Tokens
INSERT INTO token_usage(project_id,plan_id,agent,model,input_tokens,output_tokens,cost_usd)
  VALUES('proj1',1,'claude','opus',50000,10000,1.25);
INSERT INTO token_usage(project_id,plan_id,agent,model,input_tokens,output_tokens,cost_usd)
  VALUES('proj1',2,'copilot','gpt-5.3',30000,8000,0.50);

-- Notifications
INSERT INTO notifications(type,title,message,is_read)
  VALUES('info','Test notification','Hello world',0);

-- Nightly jobs
INSERT INTO nightly_jobs(
  id, run_id, job_name, started_at, finished_at, host, status,
  sentry_unresolved, github_open_issues, processed_items, fixed_items,
  summary, trigger_source, parent_run_id, report_json
) VALUES
  (1, 'mirrorbuddy-nightly-20260308-030000', 'guardian', '2026-03-08 03:00:00', '2026-03-08 03:20:00', 'local', 'ok',
   1, 2, 3, 1, 'Nightly run completed', 'scheduled', NULL, '{\"status\":\"ok\"}');

INSERT INTO nightly_job_definitions(
  id, project_id, name, description, schedule, script_path, target_host, enabled, run_fixes, timeout_sec
) VALUES
  (1, 'mirrorbuddy', 'guardian-main', 'Primary nightly guardian', '0 3 * * *', '/tmp/guardian.sh', 'local', 1, 1, 5400),
  (2, 'mirrorbuddy', 'guardian-secondary', 'Secondary nightly guardian', '30 3 * * *', '/tmp/guardian-secondary.sh', 'remote', 0, 0, 7200),
  (3, 'proj1', 'proj1-guardian', 'Project specific guardian', '0 1 * * *', '/tmp/proj1-guardian.sh', 'local', 1, 1, 1800);
";

pub(super) async fn get(router: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let body = axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    (status, json)
}

pub(super) async fn post(router: &axum::Router, uri: &str, payload: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .uri(uri)
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let body = axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    (status, json)
}

pub(super) async fn put(router: &axum::Router, uri: &str, payload: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .uri(uri)
        .method("PUT")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let body = axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    (status, json)
}
