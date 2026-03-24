use super::*;
use crate::ipc::IpcEngine;
use std::sync::Arc;
use tempfile::NamedTempFile;

fn setup_test_db() -> (NamedTempFile, rusqlite::Connection) {
    let tmp = NamedTempFile::new().unwrap();
    let conn = rusqlite::Connection::open(tmp.path()).unwrap();
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS plans (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'todo',
            parent_plan_id INTEGER,
            depends_on TEXT,
            execution_mode TEXT,
            tasks_done INTEGER DEFAULT 0,
            tasks_total INTEGER DEFAULT 0,
            project_id TEXT DEFAULT 'test'
        );
        CREATE TABLE IF NOT EXISTS waves (
            id INTEGER PRIMARY KEY,
            plan_id INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending'
        );
        CREATE TABLE IF NOT EXISTS tasks (
            id TEXT NOT NULL,
            plan_id INTEGER NOT NULL,
            wave_id INTEGER,
            status TEXT NOT NULL DEFAULT 'pending',
            PRIMARY KEY (id, plan_id)
        );",
    )
    .unwrap();
    (tmp, conn)
}

fn test_engine(db_path: &Path) -> Arc<IpcEngine> {
    let engine = Arc::new(IpcEngine::new(db_path.to_path_buf()));
    crate::ipc::ensure_ipc_schema(&engine.open_conn().unwrap()).unwrap();
    let _ = engine.channel_create(
        super::super::CHANNEL,
        Some("test"),
        super::super::ALI_AGENT,
    );
    engine
}

#[test]
fn on_plan_done_with_no_parent_succeeds() {
    let (tmp, conn) = setup_test_db();
    conn.execute(
        "INSERT INTO plans (id, name, status) VALUES (100, 'Solo Plan', 'done')",
        [],
    )
    .unwrap();
    let engine = test_engine(tmp.path());
    let result = on_plan_done(&engine, tmp.path(), 100);
    assert!(result.is_ok());
}

#[test]
fn task_done_detects_wave_completion() {
    let (tmp, conn) = setup_test_db();
    conn.execute(
        "INSERT INTO plans (id, name, status) VALUES (200, 'Test Plan', 'doing')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO waves (id, plan_id, status) VALUES (1, 200, 'in_progress')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO tasks (id, plan_id, wave_id, status) VALUES ('T1-01', 200, 1, 'done')",
        [],
    )
    .unwrap();

    let engine = test_engine(tmp.path());

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let result = rt.block_on(on_task_done(&engine, &tmp.path().to_path_buf(), "T1-01", 200));
    assert!(result.is_ok());

    // Should have broadcast wave_done since all tasks in wave 1 are done
    let history = engine
        .history(
            Some(super::super::ALI_AGENT),
            Some(super::super::CHANNEL),
            10,
            None,
        )
        .unwrap();
    if let crate::ipc::IpcResponse::MessageList { messages } = history {
        let wave_done = messages.iter().any(|m| m.content.contains("wave_done"));
        assert!(wave_done, "expected wave_done event in channel");
    }
}

#[test]
fn wave_done_emits_validation_request() {
    let (tmp, _conn) = setup_test_db();
    let engine = test_engine(tmp.path());
    let result = on_wave_done(&engine, 1, 300);
    assert!(result.is_ok());

    let history = engine
        .history(
            Some(super::super::ALI_AGENT),
            Some(super::super::CHANNEL),
            10,
            None,
        )
        .unwrap();
    if let crate::ipc::IpcResponse::MessageList { messages } = history {
        let has_validation = messages
            .iter()
            .any(|m| m.content.contains("wave_needs_validation"));
        assert!(has_validation, "expected wave_needs_validation event");
    }
}
