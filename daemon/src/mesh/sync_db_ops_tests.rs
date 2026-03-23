// Tests for mesh sync DB operations: schema, stats, error recording,
// collect_changes, apply_delta_frame, validate_peer_name.
use super::*;
use rusqlite::Connection;

fn full_schema_conn() -> Connection {
    let conn = Connection::open_in_memory().expect("conn");
    conn.execute_batch(
        "CREATE TABLE crsql_changes (\"table\" TEXT NOT NULL, pk BLOB NOT NULL, \
         cid TEXT NOT NULL, val TEXT, col_version INTEGER NOT NULL, \
         db_version INTEGER NOT NULL, site_id BLOB NOT NULL, cl INTEGER NOT NULL, \
         seq INTEGER NOT NULL); \
         CREATE TABLE crsql_site_id (site_id BLOB); \
         INSERT INTO crsql_site_id VALUES (X'6C6F63616C'); \
         CREATE TABLE tasks__crsql_clock (id INTEGER PRIMARY KEY); \
         CREATE TABLE plans__crsql_clock (id INTEGER PRIMARY KEY);",
    )
    .expect("seed schema");
    conn
}

// === validate_peer_name (via conn-based functions) ===

#[test]
fn record_sent_stats_accepts_max_length_peer_name() {
    let conn = Connection::open_in_memory().expect("conn");
    let name = "p".repeat(256);
    // Should not error on validate_peer_name
    record_sent_stats_with_conn(&conn, &name, 0, 0).expect("256 chars ok");
}

#[test]
fn record_sent_stats_rejects_empty_peer_name() {
    let conn = Connection::open_in_memory().expect("conn");
    let err = record_sent_stats_with_conn(&conn, "", 0, 0).expect_err("empty");
    assert!(err.to_string().contains("invalid peer name length"));
}

#[test]
fn record_sent_stats_rejects_257_char_peer_name() {
    let conn = Connection::open_in_memory().expect("conn");
    let name = "x".repeat(257);
    let err = record_sent_stats_with_conn(&conn, &name, 0, 0).expect_err("too long");
    assert!(err.to_string().contains("invalid peer name length: 257"));
}

// === ensure_sync_schema_pub — idempotency ===

#[test]
fn ensure_sync_schema_creates_table_idempotently() {
    let conn = Connection::open_in_memory().expect("conn");
    ensure_sync_schema_pub(&conn).expect("first call");
    ensure_sync_schema_pub(&conn).expect("second call — idempotent");
    // Verify table exists by inserting
    conn.execute(
        "INSERT INTO mesh_sync_stats(peer_name) VALUES(?1)",
        ["peer-a"],
    )
    .expect("insert into created table");
}

// === record_sent_stats_with_conn — accumulation ===

#[test]
fn record_sent_stats_accumulates_total_sent() {
    let conn = Connection::open_in_memory().expect("conn");
    record_sent_stats_with_conn(&conn, "peer-a", 10, 5).expect("first");
    record_sent_stats_with_conn(&conn, "peer-a", 20, 10).expect("second");
    let total: i64 = conn
        .query_row(
            "SELECT total_sent FROM mesh_sync_stats WHERE peer_name='peer-a'",
            [],
            |r| r.get(0),
        )
        .expect("query");
    assert_eq!(total, 30, "total_sent should accumulate");
}

#[test]
fn record_sent_stats_tracks_max_db_version() {
    let conn = Connection::open_in_memory().expect("conn");
    record_sent_stats_with_conn(&conn, "peer-b", 5, 100).expect("first");
    record_sent_stats_with_conn(&conn, "peer-b", 5, 50).expect("second lower");
    let version: i64 = conn
        .query_row(
            "SELECT last_db_version FROM mesh_sync_stats WHERE peer_name='peer-b'",
            [],
            |r| r.get(0),
        )
        .expect("query");
    assert_eq!(version, 100, "last_db_version should keep the MAX");
}

// === current_db_version_with_conn ===

#[test]
fn current_db_version_empty_table_returns_zero() {
    let conn = full_schema_conn();
    let v = current_db_version_with_conn(&conn).expect("query");
    assert_eq!(v, 0);
}

#[test]
fn current_db_version_returns_max() {
    let conn = full_schema_conn();
    conn.execute(
        "INSERT INTO crsql_changes VALUES('tasks', X'01', 'title', 'a', 1, 5, X'01', 1, 1)",
        [],
    )
    .expect("insert");
    conn.execute(
        "INSERT INTO crsql_changes VALUES('tasks', X'02', 'title', 'b', 1, 12, X'01', 1, 2)",
        [],
    )
    .expect("insert");
    let v = current_db_version_with_conn(&conn).expect("query");
    assert_eq!(v, 12);
}

// === collect_changes_with_conn ===

#[test]
fn collect_changes_with_conn_returns_only_local_changes() {
    let conn = full_schema_conn();
    // local site_id is X'6C6F63616C'
    conn.execute(
        "INSERT INTO crsql_changes VALUES('tasks', X'01', 'title', 'local', 1, 5, X'6C6F63616C', 1, 1)",
        [],
    )
    .expect("local insert");
    conn.execute(
        "INSERT INTO crsql_changes VALUES('tasks', X'02', 'title', 'remote', 1, 6, X'72656D6F7465', 1, 2)",
        [],
    )
    .expect("remote insert");
    let (changes, max_v) = collect_changes_with_conn(&conn, 0).expect("collect");
    // collect_changes_with_conn calls read_local_changes_since which filters by site_id
    assert_eq!(changes.len(), 1, "should only return local changes");
    assert_eq!(changes[0].val, Some("local".into()));
    assert_eq!(max_v, 5);
}

#[test]
fn collect_changes_with_conn_empty_returns_cursor() {
    let conn = full_schema_conn();
    let (changes, max_v) = collect_changes_with_conn(&conn, 0).expect("collect");
    assert!(changes.is_empty());
    assert_eq!(max_v, 0, "max should be the passed cursor when empty");
}

// === get_crr_table_allowlist ===

#[test]
fn allowlist_empty_db_returns_empty_set() {
    let conn = Connection::open_in_memory().expect("conn");
    let allowlist = get_crr_table_allowlist(&conn);
    assert!(allowlist.is_empty());
}

#[test]
fn allowlist_ignores_tables_without_crsql_clock_suffix() {
    let conn = Connection::open_in_memory().expect("conn");
    conn.execute_batch("CREATE TABLE regular_table (id INTEGER);")
        .expect("create");
    let allowlist = get_crr_table_allowlist(&conn);
    assert!(!allowlist.contains("regular_table"));
}

// === apply_changes_to_conn — mixed allowed/disallowed ===

#[test]
fn apply_skips_all_disallowed_table_changes() {
    let conn = full_schema_conn();
    let changes = vec![DeltaChange {
        table_name: "unknown_table".into(),
        pk: b"id=1".to_vec(),
        cid: "col".into(),
        val: Some("bad".into()),
        col_version: 1,
        db_version: 1,
        site_id: b"p1".to_vec(),
        cl: 1,
        seq: 1,
    }];
    let applied = apply_changes_to_conn(&conn, &changes).expect("apply");
    assert_eq!(applied, 0);
}

#[test]
fn apply_mixed_tables_counts_only_allowed() {
    let conn = full_schema_conn();
    let changes = vec![
        DeltaChange {
            table_name: "tasks".into(),
            pk: b"1".to_vec(),
            cid: "t".into(),
            val: Some("ok".into()),
            col_version: 1,
            db_version: 1,
            site_id: b"p".to_vec(),
            cl: 1,
            seq: 1,
        },
        DeltaChange {
            table_name: "evil".into(),
            pk: b"2".to_vec(),
            cid: "t".into(),
            val: Some("bad".into()),
            col_version: 1,
            db_version: 2,
            site_id: b"p".to_vec(),
            cl: 1,
            seq: 2,
        },
        DeltaChange {
            table_name: "plans".into(),
            pk: b"3".to_vec(),
            cid: "t".into(),
            val: Some("ok2".into()),
            col_version: 1,
            db_version: 3,
            site_id: b"p".to_vec(),
            cl: 1,
            seq: 3,
        },
    ];
    let applied = apply_changes_to_conn(&conn, &changes).expect("apply");
    assert_eq!(applied, 2, "tasks + plans allowed, evil blocked");
}

#[test]
fn read_changes_ordered_by_db_version_then_seq() {
    let conn = full_schema_conn();
    conn.execute(
        "INSERT INTO crsql_changes VALUES('tasks',X'02','c','second',1,5,X'01',1,2)", [],
    ).expect("insert");
    conn.execute(
        "INSERT INTO crsql_changes VALUES('tasks',X'01','c','first',1,3,X'01',1,1)", [],
    ).expect("insert");
    let changes = read_changes_since_from_conn(&conn, 0).expect("read");
    assert_eq!(changes.len(), 2);
    assert_eq!(changes[0].db_version, 3, "lower db_version first");
    assert_eq!(changes[1].db_version, 5);
}
