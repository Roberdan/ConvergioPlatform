// Tests for workspace::events — EventLogger record/query lifecycle.
// Why: verify event persistence and query filtering (Plan 698).

use super::*;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

fn make_pool() -> ConnPool {
    let manager = SqliteConnectionManager::memory();
    let pool = Pool::builder().max_size(4).build(manager).unwrap();
    pool.get()
        .unwrap()
        .execute_batch(
            "CREATE TABLE workspace_events (id INTEGER PRIMARY KEY AUTOINCREMENT, \
         workspace_id TEXT NOT NULL, agent TEXT NOT NULL, action TEXT NOT NULL, \
         file_path TEXT, detail TEXT, metadata TEXT, \
         created_at TEXT NOT NULL DEFAULT (datetime('now')));",
        )
        .unwrap();
    pool
}

#[test]
fn test_action_display_and_from_str_round_trip() {
    use WorkspaceAction::*;
    let actions = [
        FileRead,
        FileWrite,
        FileEdit,
        GitCommit,
        GitPush,
        PrCreated,
        PrMerged,
        QualityGatePass,
        QualityGateFail,
        WorkspaceCreated,
        WorkspaceDeleted,
    ];
    for action in &actions {
        let s = action.to_string();
        let parsed: WorkspaceAction = s.parse().unwrap();
        assert_eq!(*action, parsed, "round-trip failed for {s}");
    }
}

#[test]
fn test_action_from_str_unknown() {
    assert!("unknown_action".parse::<WorkspaceAction>().is_err());
}

#[test]
fn test_record_and_query_round_trip() {
    let logger = EventLogger::new(make_pool());
    let id = logger
        .record_event(
            "ws-abc",
            "task-executor",
            WorkspaceAction::FileWrite,
            Some("src/main.rs"),
            Some("wrote 42 lines"),
            None,
        )
        .unwrap();
    assert!(id > 0);

    let events = logger.query_events("ws-abc", None, None).unwrap();
    assert_eq!(events.len(), 1);
    let ev = &events[0];
    assert_eq!(ev.workspace_id, "ws-abc");
    assert_eq!(ev.agent, "task-executor");
    assert_eq!(ev.action, "file_write");
    assert_eq!(ev.file_path.as_deref(), Some("src/main.rs"));
    assert_eq!(ev.detail.as_deref(), Some("wrote 42 lines"));
    assert!(ev.metadata.is_none());
}

#[test]
fn test_query_with_limit() {
    let logger = EventLogger::new(make_pool());
    for i in 0..5 {
        logger
            .record_event(
                "ws-limit",
                "agent",
                WorkspaceAction::FileRead,
                Some(&format!("file{i}.rs")),
                None,
                None,
            )
            .unwrap();
    }
    let events = logger.query_events("ws-limit", Some(3), None).unwrap();
    assert_eq!(events.len(), 3);
}

#[test]
fn test_query_by_agent() {
    let logger = EventLogger::new(make_pool());
    logger
        .record_event(
            "ws-1",
            "alice",
            WorkspaceAction::GitCommit,
            None,
            None,
            None,
        )
        .unwrap();
    logger
        .record_event(
            "ws-2",
            "alice",
            WorkspaceAction::PrCreated,
            None,
            None,
            None,
        )
        .unwrap();
    logger
        .record_event("ws-3", "bob", WorkspaceAction::FileWrite, None, None, None)
        .unwrap();

    let alice = logger.query_events_by_agent("alice", None).unwrap();
    assert_eq!(alice.len(), 2);
    assert!(alice.iter().all(|e| e.agent == "alice"));

    let bob = logger.query_events_by_agent("bob", Some(10)).unwrap();
    assert_eq!(bob.len(), 1);
    assert_eq!(bob[0].action, "file_write");
}
