use super::*;
use rusqlite::Connection;

fn in_memory() -> Connection {
    Connection::open_in_memory().expect("in-memory db")
}

#[test]
fn run_is_idempotent() {
    let conn = in_memory();
    run(&conn).expect("first run");
    run(&conn).expect("second run — must be idempotent");
}

#[test]
fn execution_runs_schema_is_correct() {
    let conn = in_memory();
    run(&conn).expect("migration");

    assert!(table_exists(&conn, "execution_runs").unwrap());

    conn.execute(
        "INSERT INTO execution_runs (goal) VALUES (?1)",
        ["verify schema"],
    )
    .expect("insert");

    let status: String = conn
        .query_row(
            "SELECT status FROM execution_runs WHERE goal='verify schema'",
            [],
            |r| r.get(0),
        )
        .expect("select");
    assert_eq!(status, "running");
}

#[test]
fn execution_runs_status_constraint_rejects_invalid() {
    let conn = in_memory();
    run(&conn).expect("migration");

    let result = conn.execute(
        "INSERT INTO execution_runs (goal, status) VALUES (?1, ?2)",
        ["test goal", "invalid_status"],
    );
    assert!(
        result.is_err(),
        "CHECK constraint must reject invalid status"
    );
}

#[test]
fn indexes_exist_after_migration() {
    let conn = in_memory();
    run(&conn).expect("migration");

    for name in &[
        "idx_execution_runs_status",
        "idx_execution_runs_plan_id",
        "idx_execution_runs_started_at",
    ] {
        assert!(
            index_exists(&conn, name).unwrap(),
            "index {name} must exist"
        );
    }
}

#[test]
fn test_domain_skill_map_migration() {
    let conn = in_memory();
    run(&conn).expect("migration must succeed");

    assert!(
        table_exists(&conn, "domain_skill_map").unwrap(),
        "domain_skill_map table must exist"
    );

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM domain_skill_map", [], |r| r.get(0))
        .expect("count seed rows");
    assert_eq!(count, 3, "seed data must contain 3 rows");

    let desc: String = conn
        .query_row(
            "SELECT description FROM domain_skill_map WHERE domain=?1 AND skill_name=?2",
            ["healthcare", "research"],
            |r| r.get(0),
        )
        .expect("healthcare/research seed row must exist");
    assert_eq!(desc, "Medical research and clinical analysis");

    let desc2: String = conn
        .query_row(
            "SELECT description FROM domain_skill_map WHERE domain=?1 AND skill_name=?2",
            ["deploy", "release"],
            |r| r.get(0),
        )
        .expect("deploy/release seed row must exist");
    assert_eq!(desc2, "Deployment and release management");

    let desc3: String = conn
        .query_row(
            "SELECT description FROM domain_skill_map WHERE domain=?1 AND skill_name=?2",
            ["design", "prepare"],
            |r| r.get(0),
        )
        .expect("design/prepare seed row must exist");
    assert_eq!(desc3, "Design preparation and setup");

    let dup = conn.execute(
        "INSERT INTO domain_skill_map (domain, skill_name) VALUES (?1, ?2)",
        ["healthcare", "research"],
    );
    assert!(
        dup.is_err(),
        "UNIQUE(domain, skill_name) must reject duplicate"
    );

    run(&conn).expect("second run must be idempotent");
}

#[test]
fn sync_meta_table_created_by_migration() {
    let conn = in_memory();
    run(&conn).expect("migration");

    assert!(
        table_exists(&conn, "_sync_meta").unwrap(),
        "_sync_meta table must exist after migration"
    );

    // Verify schema: peer TEXT, table_name TEXT, last_sync_at TEXT, PK (peer, table_name)
    conn.execute(
        "INSERT INTO _sync_meta (peer, table_name, last_sync_at) VALUES (?1, ?2, ?3)",
        ["node-alpha", "tasks", "2026-03-28T10:00:00"],
    )
    .expect("insert must succeed");

    let ts: String = conn
        .query_row(
            "SELECT last_sync_at FROM _sync_meta WHERE peer=?1 AND table_name=?2",
            ["node-alpha", "tasks"],
            |r| r.get(0),
        )
        .expect("select");
    assert_eq!(ts, "2026-03-28T10:00:00");
}

#[test]
fn sync_meta_migration_idempotent_with_existing_data() {
    let conn = in_memory();
    run(&conn).expect("first run");

    conn.execute(
        "INSERT INTO _sync_meta (peer, table_name, last_sync_at) VALUES (?1, ?2, ?3)",
        ["node-beta", "plans", "2026-03-28T12:00:00"],
    )
    .expect("insert data");

    run(&conn).expect("second run must not drop existing data");

    let ts: String = conn
        .query_row(
            "SELECT last_sync_at FROM _sync_meta WHERE peer=?1 AND table_name=?2",
            ["node-beta", "plans"],
            |r| r.get(0),
        )
        .expect("data must survive re-migration");
    assert_eq!(ts, "2026-03-28T12:00:00");
}

#[test]
fn sync_meta_rejects_duplicate_peer_table_pair() {
    let conn = in_memory();
    run(&conn).expect("migration");

    conn.execute(
        "INSERT INTO _sync_meta (peer, table_name, last_sync_at) VALUES (?1, ?2, ?3)",
        ["node-a", "tasks", "2026-03-28T10:00:00"],
    )
    .expect("first insert");

    let dup = conn.execute(
        "INSERT INTO _sync_meta (peer, table_name, last_sync_at) VALUES (?1, ?2, ?3)",
        ["node-a", "tasks", "2026-03-28T11:00:00"],
    );
    assert!(
        dup.is_err(),
        "PRIMARY KEY (peer, table_name) must reject duplicate"
    );
}

#[test]
fn schema_loads_without_crsqlite_extension() {
    // Verify that PlanDb::open_path works and migrations run
    // without any crsqlite extension loaded.
    let dir = tempfile::tempdir().expect("tmpdir");
    let db_path = dir.path().join("migration-test.db");
    let db = crate::db::PlanDb::open_path(&db_path).expect("open_path");
    run(db.connection()).expect("migrations must succeed without crsqlite");

    assert!(
        table_exists(db.connection(), "_sync_meta").unwrap(),
        "_sync_meta must exist after migration on file-backed DB"
    );
    assert!(
        table_exists(db.connection(), "execution_runs").unwrap(),
        "execution_runs must exist after migration on file-backed DB"
    );
}
