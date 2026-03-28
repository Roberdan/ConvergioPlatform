// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Integration tests for kernel audio routing: active_node DB persistence and resolution.
// Uses in-memory SQLite — no daemon or mesh connection required.

#![cfg(feature = "kernel")]

use convergio_core::kernel::audio::{resolve_active_node, ActiveNode, ActiveNodeSource};

// ── Schema helper ─────────────────────────────────────────────────────────────

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
    .expect("schema creation");
    conn
}

// ── active_node write + read ──────────────────────────────────────────────────

/// Write active_node to in-memory DB, read back with resolve_active_node.
/// Verifies the full write→read round-trip using the kernel_config schema.
#[test]
fn test_active_node_write_read() {
    let conn = in_memory_db();

    // Write active_node set 5 minutes ago (well within the 8 h window).
    conn.execute_batch(
        "INSERT INTO kernel_config(key, value)
         VALUES ('active_node', 'macM5Max');
         INSERT INTO kernel_config(key, value)
         VALUES ('active_node_set_at', datetime('now', '-5 minutes'));",
    )
    .expect("insert active_node");

    let node = resolve_active_node(&conn);

    assert_eq!(node.hostname, "macM5Max", "hostname must match written value");
    assert_eq!(
        node.source,
        ActiveNodeSource::Explicit,
        "source must be Explicit for kernel_config entry"
    );
    assert!(!node.is_local(), "macM5Max must not be treated as local");
    assert_eq!(node.base_url(), "http://macM5Max:8420");
}

/// Write active_node that is expired (> 8 h ago) — must NOT be returned.
#[test]
fn test_active_node_expired_not_returned() {
    let conn = in_memory_db();

    conn.execute_batch(
        "INSERT INTO kernel_config(key, value)
         VALUES ('active_node', 'macM5Max');
         INSERT INTO kernel_config(key, value)
         VALUES ('active_node_set_at', datetime('now', '-10 hours'));",
    )
    .expect("insert expired active_node");

    let node = resolve_active_node(&conn);

    // Expired explicit node must not be returned.
    assert_ne!(
        node.hostname, "macM5Max",
        "expired active_node must not be resolved"
    );
}

// ── resolve_active_node — default fallback ────────────────────────────────────

/// With an empty DB, resolve_active_node returns localhost.
#[test]
fn test_resolve_active_node_default() {
    let conn = in_memory_db();
    let node = resolve_active_node(&conn);

    assert_eq!(
        node.hostname, "localhost",
        "empty DB must fall back to localhost"
    );
    assert_eq!(
        node.source,
        ActiveNodeSource::Localhost,
        "fallback source must be Localhost"
    );
    assert!(node.is_local(), "localhost must be reported as local");
}

// ── last CLI peer fallback ────────────────────────────────────────────────────

/// With no explicit active_node but a peer event in kernel_events,
/// the most recent peer hostname is returned.
#[test]
fn test_resolve_active_node_last_cli_peer() {
    let conn = in_memory_db();

    conn.execute_batch(
        "INSERT INTO kernel_events(source, message)
         VALUES ('macProM1', 'cvg task update 9408 done');",
    )
    .expect("insert kernel_event");

    let node = resolve_active_node(&conn);

    assert_eq!(node.hostname, "macProM1");
    assert_eq!(node.source, ActiveNodeSource::LastCli);
    assert!(!node.is_local());
}

/// Explicit node takes priority over a LastCli peer, even when both present.
#[test]
fn test_resolve_explicit_takes_priority_over_last_cli() {
    let conn = in_memory_db();

    conn.execute_batch(
        "INSERT INTO kernel_config(key, value) VALUES ('active_node', 'macM5Max');
         INSERT INTO kernel_config(key, value)
         VALUES ('active_node_set_at', datetime('now', '-1 minute'));
         INSERT INTO kernel_events(source, message)
         VALUES ('macProM1', 'cvg task update 9408 done');",
    )
    .expect("insert both sources");

    let node = resolve_active_node(&conn);

    assert_eq!(node.hostname, "macM5Max", "explicit must beat last_cli");
    assert_eq!(node.source, ActiveNodeSource::Explicit);
}

// ── ActiveNode helpers ────────────────────────────────────────────────────────

/// ActiveNode::is_local() returns true for localhost variants.
#[test]
fn test_active_node_is_local_variants() {
    for hostname in &["localhost", "127.0.0.1", "::1"] {
        let node = ActiveNode {
            hostname: hostname.to_string(),
            source: ActiveNodeSource::Localhost,
        };
        assert!(node.is_local(), "{hostname} must be local");
    }
}

/// ActiveNode::is_local() returns false for a real hostname.
#[test]
fn test_active_node_remote_is_not_local() {
    let node = ActiveNode {
        hostname: "macProM1".to_string(),
        source: ActiveNodeSource::LastCli,
    };
    assert!(!node.is_local());
    assert_eq!(node.base_url(), "http://macProM1:8420");
}
