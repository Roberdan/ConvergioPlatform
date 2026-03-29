// crsqlite-feature-gated CRDT tests extracted from tests.rs.
// Why: keep tests.rs ≤250 lines per CONSTITUTION Article V.
use crate::db::PlanDb;

#[cfg(feature = "crsqlite")]
fn seed_change_schema(db: &PlanDb) {
    db.connection()
        .execute_batch(
            r#"
            CREATE TABLE crsql_changes (
              "table" TEXT NOT NULL,
              pk TEXT NOT NULL,
              cid TEXT NOT NULL,
              val TEXT,
              col_version INTEGER NOT NULL,
              db_version INTEGER NOT NULL,
              site_id TEXT NOT NULL,
              cl INTEGER NOT NULL,
              seq INTEGER NOT NULL
            );
            "#,
        )
        .expect("schema");
}

#[cfg(feature = "crsqlite")]
#[test]
fn crdt_marks_required_tables() {
    use rusqlite::functions::FunctionFlags;
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};
    use super::required_crdt_tables;

    let conn = Connection::open_in_memory().expect("conn");
    let called = Arc::new(Mutex::new(Vec::<String>::new()));
    let sink = Arc::clone(&called);
    for table in required_crdt_tables() {
        conn.execute(
            &format!("CREATE TABLE \"{table}\" (id TEXT PRIMARY KEY)"),
            [],
        )
        .expect("create table");
    }
    conn.create_scalar_function("crsql_as_crr", 1, FunctionFlags::SQLITE_UTF8, move |ctx| {
        sink.lock().expect("lock").push(
            ctx.get::<String>(0)
                .expect("table argument for crsql_as_crr"),
        );
        Ok(1_i64)
    })
    .expect("register function");
    super::migration::mark_required_tables(&conn).expect("mark tables");
    assert_eq!(
        called.lock().expect("lock").clone(),
        required_crdt_tables()
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>()
    );
}

#[cfg(feature = "crsqlite")]
#[test]
fn crdt_changes_converge_between_two_nodes() {
    let left = PlanDb::open_in_memory().expect("left db");
    let right = PlanDb::open_in_memory().expect("right db");
    seed_change_schema(&left);
    seed_change_schema(&right);
    left.connection()
        .execute(
            r#"INSERT INTO crsql_changes ("table",pk,cid,val,col_version,db_version,site_id,cl,seq)
           VALUES ('tasks','id=1','title','left',1,1,'left',1,1)"#,
            [],
        )
        .expect("left change");
    right
        .connection()
        .execute(
            r#"INSERT INTO crsql_changes ("table",pk,cid,val,col_version,db_version,site_id,cl,seq)
           VALUES ('tasks','id=2','title','right',1,1,'right',1,1)"#,
            [],
        )
        .expect("right change");
    let left_changes = left.export_changes().expect("left export");
    let right_changes = right.export_changes().expect("right export");
    left.apply_changes(&right_changes).expect("left apply");
    right.apply_changes(&left_changes).expect("right apply");
    assert_eq!(left.export_changes().expect("left final").len(), 2);
    assert_eq!(right.export_changes().expect("right final").len(), 2);
}

#[cfg(feature = "crsqlite")]
#[test]
fn crdt_apply_is_idempotent() {
    let db = PlanDb::open_in_memory().expect("db");
    seed_change_schema(&db);

    let changes = vec![crate::db::crdt::CrdtChange {
        table_name: "tasks".into(),
        pk: "id=42".into(),
        cid: "title".into(),
        val: Some("Buy milk".into()),
        col_version: 3,
        db_version: 1,
        site_id: "node-a".into(),
        cl: 1,
        seq: 0,
    }];

    let n1 = db.apply_changes(&changes).expect("first apply");
    assert_eq!(n1, 1, "first apply should insert one change");

    let n2 = db.apply_changes(&changes).expect("second apply");
    assert_eq!(n2, 0, "duplicate apply must be skipped");

    let count: i64 = db
        .connection()
        .query_row("SELECT COUNT(*) FROM crsql_changes", [], |r| r.get(0))
        .expect("count");
    assert_eq!(count, 1, "table must hold exactly one row after duplicate apply");
}

#[cfg(feature = "crsqlite")]
#[test]
fn crdt_apply_newer_version_replaces_older() {
    let db = PlanDb::open_in_memory().expect("db");
    seed_change_schema(&db);

    let base = crate::db::crdt::CrdtChange {
        table_name: "tasks".into(),
        pk: "id=7".into(),
        cid: "status".into(),
        val: Some("pending".into()),
        col_version: 1,
        db_version: 1,
        site_id: "node-b".into(),
        cl: 1,
        seq: 0,
    };
    db.apply_changes(&[base.clone()]).expect("base insert");

    let newer = crate::db::crdt::CrdtChange { col_version: 5, val: Some("done".into()), ..base };
    let n = db.apply_changes(&[newer]).expect("newer apply");
    assert_eq!(n, 1, "newer version must be applied");

    let val: Option<String> = db
        .connection()
        .query_row(
            r#"SELECT val FROM crsql_changes WHERE "table" = 'tasks' AND pk = 'id=7'"#,
            [],
            |r| r.get(0),
        )
        .expect("query val");
    assert_eq!(val.as_deref(), Some("done"), "col value must be updated to newer version");
}

#[cfg(feature = "crsqlite")]
#[test]
fn crdt_avoids_format_sql_for_dynamic_identifiers() {
    let source = include_str!("migration.rs");
    let banned_patterns = [
        "format!(\"DROP TABLE IF EXISTS \\\"{tmp}\\\"\")",
        "format!(\"DROP VIEW IF EXISTS \\\"{name}\\\"\")",
        "format!(\"DROP TRIGGER IF EXISTS \\\"{name}\\\"\")",
        "format!(\"SELECT crsql_as_crr('{table}')\")",
        "format!(\"DROP INDEX IF EXISTS \\\"{idx}\\\"\")",
        "format!(\"PRAGMA table_info(\\\"{}\\\")\", table)",
        "format!(\"PRAGMA foreign_key_list(\\\"{}\\\")\", table)",
    ];
    for pattern in banned_patterns {
        assert!(!source.contains(pattern), "found non-parameterized SQL pattern: {pattern}");
    }
}
