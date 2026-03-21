use crate::db::PlanDb;
use rusqlite::{functions::FunctionFlags, Connection};
use std::sync::{Arc, Mutex};

use super::required_crdt_tables;

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

#[test]
fn crdt_marks_required_tables() {
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

#[test]
fn crdt_changes_converge_between_two_nodes() {
    let left = PlanDb::open_in_memory().expect("left db");
    let right = PlanDb::open_in_memory().expect("right db");
    seed_change_schema(&left);
    seed_change_schema(&right);
    left.connection().execute(
        r#"INSERT INTO crsql_changes ("table",pk,cid,val,col_version,db_version,site_id,cl,seq)
           VALUES ('tasks','id=1','title','left',1,1,'left',1,1)"#,
        [],
    )
    .expect("left change");
    right.connection().execute(
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

#[test]
fn crdt_avoids_format_sql_for_dynamic_identifiers() {
    // Verify that migration.rs (the file that does actual SQL) avoids raw format! for SQL identifiers
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
        assert!(
            !source.contains(pattern),
            "found non-parameterized SQL pattern: {pattern}"
        );
    }
}
