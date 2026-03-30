use super::*;

fn open_mem() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();
    conn
}

#[test]
fn test_enqueue_and_list_pending() {
    let conn = open_mem();
    let id = enqueue_validation(&conn, Some(1), Some(10), Some(100)).unwrap();
    assert!(id > 0);
    let pending = get_pending(&conn).unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].task_id, Some(1));
}

#[test]
fn test_enqueue_idempotent() {
    let conn = open_mem();
    let id1 = enqueue_validation(&conn, Some(42), None, None).unwrap();
    let id2 = enqueue_validation(&conn, Some(42), None, None).unwrap();
    assert_eq!(id1, id2, "second enqueue should return existing id");
}

#[test]
fn test_record_verdict_and_get() {
    let conn = open_mem();
    let qid = enqueue_validation(&conn, Some(5), None, None).unwrap();
    record_verdict(&conn, qid, "pass", Some("all gates ok"), Some("thor")).unwrap();

    let v = get_verdict(&conn, 5).unwrap().expect("verdict missing");
    assert_eq!(v.verdict, "pass");
    assert_eq!(v.validator.as_deref(), Some("thor"));

    let queue = list_queue(&conn).unwrap();
    assert_eq!(queue[0].status, "completed");
}

#[test]
fn test_timeout_stale() {
    let conn = open_mem();
    // Insert directly with a past timestamp to simulate stale entry.
    conn.execute(
        "INSERT INTO validation_queue (task_id, status, created_at)
         VALUES (99, 'pending', datetime('now', '-600 seconds'))",
        [],
    )
    .unwrap();
    let reaped = timeout_stale(&conn, 300).unwrap();
    assert_eq!(reaped, 1);
    let pending = get_pending(&conn).unwrap();
    assert_eq!(pending.len(), 0);
}

#[test]
fn test_get_verdict_none() {
    let conn = open_mem();
    let result = get_verdict(&conn, 999).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_migrations_idempotent() {
    let conn = open_mem();
    // Running twice must not fail.
    run_migrations(&conn).unwrap();
}
