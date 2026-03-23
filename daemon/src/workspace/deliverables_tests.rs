// Tests for workspace::deliverables — deliverable workspace lifecycle.
// Why: verify Plan B deliverable tracking without git branches (Plan 698).

use super::*;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

fn make_pool() -> ConnPool {
    let manager = SqliteConnectionManager::memory();
    let pool = Pool::builder().max_size(4).build(manager).unwrap();
    let conn = pool.get().unwrap();
    conn.execute_batch(
        "CREATE TABLE projects (
           id TEXT PRIMARY KEY,
           name TEXT NOT NULL,
           output_path TEXT
         );
         CREATE TABLE workspaces (
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           plan_id INTEGER,
           wave_db_id INTEGER,
           workspace_id TEXT UNIQUE NOT NULL,
           path TEXT NOT NULL,
           branch TEXT,
           status TEXT NOT NULL DEFAULT 'active',
           created_at TEXT NOT NULL DEFAULT (datetime('now')),
           deleted_at TEXT
         );
         CREATE TABLE workspace_events (
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           workspace_id TEXT NOT NULL,
           agent TEXT NOT NULL,
           action TEXT NOT NULL,
           file_path TEXT,
           detail TEXT,
           metadata TEXT,
           created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE TABLE deliverables (
           id INTEGER PRIMARY KEY,
           task_id INTEGER,
           project_id TEXT NOT NULL,
           name TEXT NOT NULL,
           output_path TEXT,
           version INTEGER DEFAULT 1,
           status TEXT DEFAULT 'pending',
           output_type TEXT NOT NULL,
           metadata_json TEXT DEFAULT '{}',
           created_at DATETIME DEFAULT CURRENT_TIMESTAMP
         );",
    )
    .unwrap();
    pool
}

#[test]
fn test_create_deliverable_workspace_with_project_output_path() {
    let pool = make_pool();
    pool.get()
        .unwrap()
        .execute(
            "INSERT INTO projects (id, name, output_path) VALUES ('proj-1', 'Test', '/out/proj-1')",
            [],
        )
        .unwrap();

    let ws = create_deliverable_workspace("proj-1", None, &pool).unwrap();

    assert!(!ws.workspace_id.is_empty());
    assert_eq!(ws.path, "/out/proj-1");
    assert!(
        ws.branch.is_none(),
        "deliverable workspace must have no branch"
    );
    assert_eq!(ws.status, "active");
}

#[test]
fn test_create_deliverable_workspace_fallback_path() {
    let pool = make_pool();
    pool.get()
        .unwrap()
        .execute(
            "INSERT INTO projects (id, name) VALUES ('proj-2', 'No Output')",
            [],
        )
        .unwrap();

    let ws = create_deliverable_workspace("proj-2", None, &pool).unwrap();

    assert!(
        ws.path.contains("proj-2"),
        "fallback path should include project_id"
    );
    assert!(ws.branch.is_none());
}

#[test]
fn test_create_deliverable_workspace_unknown_project_uses_fallback() {
    let pool = make_pool();
    // Project does not exist — should still succeed with fallback path
    let ws = create_deliverable_workspace("proj-unknown", None, &pool).unwrap();
    assert!(ws.path.contains("proj-unknown"));
    assert!(ws.branch.is_none());
}

#[test]
fn test_create_deliverable_workspace_with_task_id_records_event() {
    let pool = make_pool();
    pool.get()
        .unwrap()
        .execute(
            "INSERT INTO projects (id, name, output_path) VALUES ('proj-3', 'T', '/out/p3')",
            [],
        )
        .unwrap();

    let ws = create_deliverable_workspace("proj-3", Some(42), &pool).unwrap();

    let conn = pool.get().unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM workspace_events WHERE workspace_id = ?1 AND action = 'workspace_created'",
            params![ws.workspace_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1, "task_id should produce workspace_created event");
}

#[test]
fn test_record_deliverable_event_returns_rowid() {
    let pool = make_pool();
    pool.get()
        .unwrap()
        .execute("INSERT INTO projects (id, name) VALUES ('p4', 'P4')", [])
        .unwrap();
    let ws = create_deliverable_workspace("p4", None, &pool).unwrap();

    let event_id = record_deliverable_event(
        &ws.workspace_id,
        7,
        "file_write",
        "generated report.pdf",
        &pool,
    )
    .unwrap();
    assert!(event_id > 0);
}

#[test]
fn test_list_workspace_deliverables_empty() {
    let pool = make_pool();
    pool.get()
        .unwrap()
        .execute("INSERT INTO projects (id, name) VALUES ('p5', 'P5')", [])
        .unwrap();
    let ws = create_deliverable_workspace("p5", None, &pool).unwrap();

    let result = list_workspace_deliverables(&ws.workspace_id, &pool).unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_list_workspace_deliverables_returns_linked_rows() {
    let pool = make_pool();
    pool.get()
        .unwrap()
        .execute("INSERT INTO projects (id, name) VALUES ('p6', 'P6')", [])
        .unwrap();
    let ws = create_deliverable_workspace("p6", None, &pool).unwrap();

    let conn = pool.get().unwrap();
    conn.execute(
        "INSERT INTO deliverables (id, project_id, name, output_type, status, version) \
         VALUES (100, 'p6', 'Final Report', 'pdf', 'ready', 2)",
        [],
    )
    .unwrap();
    drop(conn);

    record_deliverable_event(
        &ws.workspace_id,
        100,
        "file_write",
        "wrote final report",
        &pool,
    )
    .unwrap();

    let deliverables = list_workspace_deliverables(&ws.workspace_id, &pool).unwrap();
    assert_eq!(deliverables.len(), 1);
    let d = &deliverables[0];
    assert_eq!(d.id, 100);
    assert_eq!(d.name, "Final Report");
    assert_eq!(d.output_type, "pdf");
    assert_eq!(d.status, "ready");
    assert_eq!(d.version, 2);
}

#[test]
fn test_list_workspace_deliverables_deduplicates() {
    let pool = make_pool();
    pool.get()
        .unwrap()
        .execute("INSERT INTO projects (id, name) VALUES ('p7', 'P7')", [])
        .unwrap();
    let ws = create_deliverable_workspace("p7", None, &pool).unwrap();

    let conn = pool.get().unwrap();
    conn.execute(
        "INSERT INTO deliverables (id, project_id, name, output_type) \
         VALUES (200, 'p7', 'Slide Deck', 'pptx')",
        [],
    )
    .unwrap();
    drop(conn);

    // Two events for same deliverable — DISTINCT must collapse to one result
    record_deliverable_event(&ws.workspace_id, 200, "file_write", "draft v1", &pool).unwrap();
    record_deliverable_event(&ws.workspace_id, 200, "file_write", "draft v2", &pool).unwrap();

    let deliverables = list_workspace_deliverables(&ws.workspace_id, &pool).unwrap();
    assert_eq!(
        deliverables.len(),
        1,
        "DISTINCT must deduplicate same deliverable"
    );
}
