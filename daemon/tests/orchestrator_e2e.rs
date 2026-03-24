// Orchestrator end-to-end integration tests — in-memory SQLite, no HTTP, no daemon.
use claude_core::ipc::{IpcEngine, IpcResponse, MessageInfo};
use claude_core::orchestrator::actions::emit;
use claude_core::orchestrator::handlers::{
    on_delegation_failed, on_plan_done, on_plan_ready, on_task_done, on_wave_done,
    on_wave_validated,
};
use claude_core::orchestrator::{ALI_AGENT, CHANNEL};
use std::sync::Arc;
use tempfile::NamedTempFile;

const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS plans (
        id INTEGER PRIMARY KEY, name TEXT NOT NULL DEFAULT '',
        status TEXT NOT NULL DEFAULT 'todo', parent_plan_id INTEGER,
        depends_on TEXT, execution_mode TEXT,
        tasks_done INTEGER DEFAULT 0, tasks_total INTEGER DEFAULT 0,
        project_id TEXT DEFAULT 'test'
    );
    CREATE TABLE IF NOT EXISTS waves (
        id INTEGER PRIMARY KEY, plan_id INTEGER NOT NULL,
        status TEXT NOT NULL DEFAULT 'pending'
    );
    CREATE TABLE IF NOT EXISTS tasks (
        id TEXT NOT NULL, plan_id INTEGER NOT NULL,
        wave_id INTEGER, status TEXT NOT NULL DEFAULT 'pending',
        PRIMARY KEY (id, plan_id)
    );
";

fn setup_test_db() -> (NamedTempFile, rusqlite::Connection) {
    let tmp = NamedTempFile::new().unwrap();
    let conn = rusqlite::Connection::open(tmp.path()).unwrap();
    conn.execute_batch(SCHEMA).unwrap();
    (tmp, conn)
}

fn test_engine(db_path: &std::path::Path) -> Arc<IpcEngine> {
    let engine = Arc::new(IpcEngine::new(db_path.to_path_buf()));
    claude_core::ipc::ensure_ipc_schema(&engine.open_conn().unwrap()).unwrap();
    let _ = engine.channel_create(CHANNEL, Some("test"), ALI_AGENT);
    engine
}

fn channel_messages(engine: &Arc<IpcEngine>) -> Vec<MessageInfo> {
    match engine.history(Some(ALI_AGENT), Some(CHANNEL), 50, None).unwrap() {
        IpcResponse::MessageList { messages } => messages,
        _ => vec![],
    }
}

fn has_event(engine: &Arc<IpcEngine>, event: &str) -> bool {
    channel_messages(engine).iter().any(|m| m.content.contains(event))
}

/// Plan with no deps — on_plan_ready emits plan_delegated or need_human (no mesh in tests).
#[tokio::test]
async fn plan_started_with_deps_met_emits_delegate() {
    let (tmp, conn) = setup_test_db();
    conn.execute("INSERT INTO plans (id, name) VALUES (1, 'Solo')", []).unwrap();
    let engine = test_engine(tmp.path());
    on_plan_ready(&engine, &tmp.path().to_path_buf(), 1).await.unwrap();
    let any_outcome = has_event(&engine, "plan_delegated") || has_event(&engine, "need_human");
    assert!(any_outcome, "expected plan_delegated or need_human");
}

/// Plan B depends on A which is not done — should emit plan_blocked.
#[tokio::test]
async fn plan_started_with_deps_blocked_emits_blocked() {
    let (tmp, conn) = setup_test_db();
    conn.execute("INSERT INTO plans (id, name) VALUES (10, 'A')", []).unwrap();
    conn.execute(
        "INSERT INTO plans (id, name, depends_on) VALUES (11, 'B', '10')", [],
    ).unwrap();
    let engine = test_engine(tmp.path());
    on_plan_ready(&engine, &tmp.path().to_path_buf(), 11).await.unwrap();
    assert!(has_event(&engine, "plan_blocked"), "expected plan_blocked");
}

/// Wave with 2 done tasks — triggers wave_done.
#[tokio::test]
async fn task_done_completes_wave() {
    let (tmp, conn) = setup_test_db();
    conn.execute("INSERT INTO plans (id, name, status) VALUES (20, 'P', 'doing')", []).unwrap();
    conn.execute("INSERT INTO waves (id, plan_id, status) VALUES (1, 20, 'in_progress')", []).unwrap();
    conn.execute("INSERT INTO tasks (id, plan_id, wave_id, status) VALUES ('T1', 20, 1, 'done')", []).unwrap();
    conn.execute("INSERT INTO tasks (id, plan_id, wave_id, status) VALUES ('T2', 20, 1, 'done')", []).unwrap();
    let engine = test_engine(tmp.path());
    on_task_done(&engine, &tmp.path().to_path_buf(), "T2", 20).await.unwrap();
    assert!(has_event(&engine, "wave_done"), "expected wave_done");
}

/// wave_done emits wave_needs_validation.
#[test]
fn wave_done_triggers_validation() {
    let (tmp, _conn) = setup_test_db();
    let engine = test_engine(tmp.path());
    on_wave_done(&engine, 5, 30).unwrap();
    assert!(has_event(&engine, "wave_needs_validation"), "expected wave_needs_validation");
}

/// Validating wave 1 with a pending wave 2 emits wave_ready.
#[test]
fn wave_validated_advances_to_next_or_plan_done() {
    let (tmp, conn) = setup_test_db();
    conn.execute("INSERT INTO plans (id, name, status) VALUES (40, 'P', 'doing')", []).unwrap();
    conn.execute("INSERT INTO waves (id, plan_id, status) VALUES (1, 40, 'in_progress')", []).unwrap();
    conn.execute("INSERT INTO waves (id, plan_id, status) VALUES (2, 40, 'pending')", []).unwrap();
    let engine = test_engine(tmp.path());
    on_wave_validated(&engine, tmp.path(), 1, 40).unwrap();
    assert!(has_event(&engine, "wave_ready"), "expected wave_ready for wave 2");
}

/// Completing child A unblocks child B (depends on A) under the same master.
#[test]
fn plan_done_unblocks_sibling() {
    let (tmp, conn) = setup_test_db();
    conn.execute("INSERT INTO plans (id, name, status) VALUES (50, 'Master', 'doing')", []).unwrap();
    conn.execute("INSERT INTO plans (id, name, status, parent_plan_id) VALUES (51, 'A', 'done', 50)", []).unwrap();
    conn.execute("INSERT INTO plans (id, name, status, parent_plan_id, depends_on) VALUES (52, 'B', 'todo', 50, '51')", []).unwrap();
    let engine = test_engine(tmp.path());
    on_plan_done(&engine, tmp.path(), 51).unwrap();
    assert!(has_event(&engine, "plan_ready"), "expected plan_ready for child B");
}

/// delegation_failed with no mesh peers emits need_human.
#[tokio::test]
async fn delegation_failed_with_no_peers_emits_need_human() {
    let (tmp, conn) = setup_test_db();
    conn.execute("INSERT INTO plans (id, name, status) VALUES (60, 'P', 'doing')", []).unwrap();
    let engine = test_engine(tmp.path());
    on_delegation_failed(&engine, &tmp.path().to_path_buf(), 60, "node-1", "SSH timeout").await.unwrap();
    assert!(has_event(&engine, "need_human"), "expected need_human");
}

/// Emit with missing/null fields should not crash.
#[test]
fn malformed_event_does_not_crash() {
    let (tmp, _conn) = setup_test_db();
    let engine = test_engine(tmp.path());
    assert!(emit(&engine, "unknown_event", &serde_json::json!({})).is_ok());
    assert!(emit(&engine, "plan_started", &serde_json::json!({"plan_id": null})).is_ok());
}

/// 10 concurrent task_done events — no panic.
#[tokio::test]
async fn parallel_task_done_events() {
    let (tmp, conn) = setup_test_db();
    conn.execute("INSERT INTO plans (id, name, status) VALUES (70, 'P', 'doing')", []).unwrap();
    conn.execute("INSERT INTO waves (id, plan_id, status) VALUES (10, 70, 'in_progress')", []).unwrap();
    for i in 1..=10 {
        conn.execute(
            &format!("INSERT INTO tasks (id, plan_id, wave_id, status) VALUES ('T{i}', 70, 10, 'pending')"),
            [],
        ).unwrap();
    }
    let engine = test_engine(tmp.path());
    let db_path = tmp.path().to_path_buf();
    let mut handles = vec![];
    for i in 1..=10 {
        let e = engine.clone();
        let p = db_path.clone();
        handles.push(tokio::spawn(async move {
            let conn = rusqlite::Connection::open(&p).unwrap();
            conn.execute(&format!("UPDATE tasks SET status='done' WHERE id='T{i}'"), []).unwrap();
            on_task_done(&e, &p, &format!("T{i}"), 70).await.unwrap();
        }));
    }
    for h in handles { h.await.unwrap(); }
}

/// Sending plan_ready twice for the same plan is idempotent — no crash.
#[tokio::test]
async fn double_plan_started_is_idempotent() {
    let (tmp, conn) = setup_test_db();
    conn.execute("INSERT INTO plans (id, name) VALUES (80, 'P')", []).unwrap();
    let engine = test_engine(tmp.path());
    let db_path = tmp.path().to_path_buf();
    on_plan_ready(&engine, &db_path, 80).await.unwrap();
    on_plan_ready(&engine, &db_path, 80).await.unwrap();
    assert!(!channel_messages(&engine).is_empty(), "expected at least one event");
}
