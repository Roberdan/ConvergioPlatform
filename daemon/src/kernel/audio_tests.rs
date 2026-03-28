// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Tests for kernel/audio.rs — resolve_active_node, ActiveNode helpers.
//
// Included by audio.rs via:
//   #[cfg(test)] #[path = "audio_tests.rs"] mod tests;
// so `super::` refers to the audio module.

use super::*;

fn in_memory_db() -> rusqlite::Connection {
    let conn = rusqlite::Connection::open_in_memory().expect("in-memory db");
    conn.execute_batch(
        "CREATE TABLE kernel_config (
             key TEXT PRIMARY KEY NOT NULL,
             value TEXT NOT NULL DEFAULT '',
             updated_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE TABLE kernel_events (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             timestamp TEXT NOT NULL DEFAULT (datetime('now')),
             severity TEXT NOT NULL DEFAULT 'ok',
             source TEXT NOT NULL DEFAULT '',
             message TEXT NOT NULL DEFAULT '',
             action_taken TEXT NOT NULL DEFAULT ''
         );",
    )
    .expect("schema");
    conn
}

#[test]
fn fallback_to_localhost_when_no_data() {
    let conn = in_memory_db();
    let node = resolve_active_node(&conn);
    assert_eq!(node.hostname, "localhost");
    assert_eq!(node.source, ActiveNodeSource::Localhost);
}

#[test]
fn explicit_node_fresh() {
    let conn = in_memory_db();
    // Insert explicit active_node set 1 minute ago
    conn.execute_batch(
        "INSERT INTO kernel_config(key, value) VALUES ('active_node', 'macM5Max');
         INSERT INTO kernel_config(key, value) VALUES
           ('active_node_set_at', datetime('now', '-1 minute'));",
    )
    .unwrap();
    let node = resolve_active_node(&conn);
    assert_eq!(node.hostname, "macM5Max");
    assert_eq!(node.source, ActiveNodeSource::Explicit);
    assert!(!node.is_local());
}

#[test]
fn explicit_node_expired_falls_back() {
    let conn = in_memory_db();
    // Insert an expired active_node (10 hours ago)
    conn.execute_batch(
        "INSERT INTO kernel_config(key, value) VALUES ('active_node', 'macM5Max');
         INSERT INTO kernel_config(key, value) VALUES
           ('active_node_set_at', datetime('now', '-10 hours'));",
    )
    .unwrap();
    let node = resolve_active_node(&conn);
    // Should NOT return macM5Max — expired
    assert_ne!(node.hostname, "macM5Max");
}

#[test]
fn last_cli_peer_detected() {
    let conn = in_memory_db();
    conn.execute_batch(
        "INSERT INTO kernel_events(source, message) VALUES ('macProM1', 'cvg task update');",
    )
    .unwrap();
    let node = resolve_active_node(&conn);
    assert_eq!(node.hostname, "macProM1");
    assert_eq!(node.source, ActiveNodeSource::LastCli);
}

#[test]
fn explicit_takes_priority_over_last_cli() {
    let conn = in_memory_db();
    conn.execute_batch(
        "INSERT INTO kernel_config(key, value) VALUES ('active_node', 'macM5Max');
         INSERT INTO kernel_config(key, value) VALUES
           ('active_node_set_at', datetime('now', '-1 minute'));
         INSERT INTO kernel_events(source, message) VALUES ('macProM1', 'cvg task update');",
    )
    .unwrap();
    let node = resolve_active_node(&conn);
    assert_eq!(node.hostname, "macM5Max");
    assert_eq!(node.source, ActiveNodeSource::Explicit);
}

#[test]
fn localhost_is_local() {
    let node =
        ActiveNode { hostname: "localhost".to_string(), source: ActiveNodeSource::Localhost };
    assert!(node.is_local());
}

#[test]
fn remote_node_is_not_local() {
    let node =
        ActiveNode { hostname: "macM5Max".to_string(), source: ActiveNodeSource::LastCli };
    assert!(!node.is_local());
    assert_eq!(node.base_url(), "http://macM5Max:8420");
}

#[test]
fn temp_wav_path_contains_pid() {
    let path = temp_wav_path();
    let pid = std::process::id();
    assert!(path.to_string_lossy().contains(&pid.to_string()));
}
