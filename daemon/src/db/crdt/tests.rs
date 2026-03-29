use rusqlite::Connection;

use crate::db::PlanDb;
#[cfg(feature = "crsqlite")]
use rusqlite::functions::FunctionFlags;
#[cfg(feature = "crsqlite")]
use std::sync::{Arc, Mutex};

#[cfg(feature = "crsqlite")]
use super::required_crdt_tables;
use super::sync::{record_sync_failure, record_sync_success};

fn setup_mesh_sync_stats(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS mesh_sync_stats (
            peer_name             TEXT PRIMARY KEY,
            last_sync_at          TEXT,
            consecutive_failures  INTEGER DEFAULT 0,
            status                TEXT    DEFAULT 'online'
        );",
    )
    .expect("create mesh_sync_stats");
}

fn insert_peer(conn: &Connection, peer: &str) {
    conn.execute(
        "INSERT OR IGNORE INTO mesh_sync_stats (peer_name) VALUES (?1)",
        [peer],
    )
    .expect("insert peer");
}

fn get_peer_status(conn: &Connection, peer: &str) -> (i64, String) {
    conn.query_row(
        "SELECT consecutive_failures, status FROM mesh_sync_stats WHERE peer_name = ?1",
        [peer],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
    )
    .expect("query peer")
}

#[test]
fn test_unreachable_after_3() {
    let db = PlanDb::open_in_memory().expect("db");
    let conn = db.connection();
    setup_mesh_sync_stats(conn);
    insert_peer(conn, "peer-alpha");

    record_sync_failure(conn, "peer-alpha").expect("failure 1");
    record_sync_failure(conn, "peer-alpha").expect("failure 2");
    record_sync_failure(conn, "peer-alpha").expect("failure 3");

    let (failures, status) = get_peer_status(conn, "peer-alpha");
    assert_eq!(failures, 3);
    assert_eq!(status, "unreachable");
}

#[test]
fn test_reset_on_success() {
    let db = PlanDb::open_in_memory().expect("db");
    let conn = db.connection();
    setup_mesh_sync_stats(conn);
    insert_peer(conn, "peer-beta");

    record_sync_failure(conn, "peer-beta").expect("failure 1");
    record_sync_failure(conn, "peer-beta").expect("failure 2");
    record_sync_failure(conn, "peer-beta").expect("failure 3");

    let (_, status) = get_peer_status(conn, "peer-beta");
    assert_eq!(status, "unreachable");

    record_sync_success(conn, "peer-beta").expect("success");

    let (failures, status) = get_peer_status(conn, "peer-beta");
    assert_eq!(failures, 0);
    assert_eq!(status, "online");
}

