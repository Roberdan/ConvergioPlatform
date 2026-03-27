// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Integration tests: monitor (detect failure), verify (block no evidence),
// recover (dry_run), audio (resolve_active_node).
// All tests require #[cfg(feature = "kernel")].

#![cfg(feature = "kernel")]

use claude_core::kernel::{
    audio::{resolve_active_node, ActiveNodeSource},
    monitor::{check_daemon_reachable, classify_and_store},
    recover::{recover, RecoveryConfig, Severity},
    verify::{check_evidence, EvidenceReport},
};
use claude_core::kernel::engine::{KernelConfig, KernelEngine};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_pool() -> Pool<SqliteConnectionManager> {
    let manager = SqliteConnectionManager::memory();
    let pool = Pool::new(manager).expect("pool");
    let conn = pool.get().expect("conn");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS kernel_events (
             id           INTEGER PRIMARY KEY AUTOINCREMENT,
             timestamp    TEXT NOT NULL DEFAULT (datetime('now')),
             severity     TEXT NOT NULL DEFAULT 'ok',
             source       TEXT NOT NULL DEFAULT '',
             message      TEXT NOT NULL DEFAULT '',
             action_taken TEXT NOT NULL DEFAULT 'none'
         );",
    )
    .expect("schema");
    pool
}

fn make_conn_with_verify_schema() -> Connection {
    let conn = Connection::open_in_memory().expect("in-memory db");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS kernel_verifications (
             id             INTEGER PRIMARY KEY AUTOINCREMENT,
             task_id        INTEGER,
             timestamp      TEXT NOT NULL DEFAULT (datetime('now')),
             checks_json    TEXT NOT NULL DEFAULT '[]',
             passed         INTEGER NOT NULL DEFAULT 1,
             blocked_reason TEXT
         );",
    )
    .expect("schema");
    conn
}

fn make_audio_conn() -> Connection {
    let conn = Connection::open_in_memory().expect("in-memory db");
    conn.execute_batch(
        "CREATE TABLE kernel_config (
             key        TEXT PRIMARY KEY NOT NULL,
             value      TEXT NOT NULL DEFAULT '',
             updated_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE TABLE kernel_events (
             id           INTEGER PRIMARY KEY AUTOINCREMENT,
             timestamp    TEXT NOT NULL DEFAULT (datetime('now')),
             severity     TEXT NOT NULL DEFAULT 'ok',
             source       TEXT NOT NULL DEFAULT '',
             message      TEXT NOT NULL DEFAULT '',
             action_taken TEXT NOT NULL DEFAULT 'none'
         );",
    )
    .expect("schema");
    conn
}

fn make_engine() -> KernelEngine {
    KernelEngine::new(KernelConfig::default())
}

// ---------------------------------------------------------------------------
// test_monitor_detect_failure
// WHY: verify that a daemon returning HTTP 500 triggers a CRITICAL event stored
// in kernel_events with severity = 'critical'.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_monitor_detect_failure() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let pool = make_pool();
    let daemon_url = server.uri();

    let result = check_daemon_reachable(&daemon_url).await;
    assert!(!result.ok, "HTTP 500 must be a failed check");

    // classify_and_store writes the event and returns true for critical checks
    let critical = classify_and_store(&pool, &[result]);
    assert!(critical, "daemon_health failure must be classified as CRITICAL");

    // Verify the event is stored in kernel_events with severity = 'critical'
    let conn = pool.get().expect("conn");
    let (severity, source): (String, String) = conn
        .query_row(
            "SELECT severity, source FROM kernel_events ORDER BY id DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("event row must exist");

    assert_eq!(severity, "critical");
    assert_eq!(source, "daemon_health");
}

// ---------------------------------------------------------------------------
// test_verify_block_no_evidence
// WHY: tasks marked done without output files must be blocked (passed = false)
// and the record persisted in kernel_verifications.
// ---------------------------------------------------------------------------

#[test]
fn test_verify_block_no_evidence() {
    let conn = make_conn_with_verify_schema();
    let engine = make_engine();

    // Pass an output file path that does NOT exist — simulates no evidence
    let missing = "/tmp/nonexistent_kernel_test_output_file_abc123.rs";
    let report: EvidenceReport =
        check_evidence(&conn, &engine, 1001, "done", None, &[missing]);

    // The file does not exist → output_file_exists check fails → gate blocked
    let failed = report.failed_checks();
    assert!(
        failed.iter().any(|c| c.name == "output_file_exists"),
        "output_file_exists check must fail for missing file"
    );

    // Verify a row was persisted
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM kernel_verifications WHERE task_id = 1001",
            [],
            |r| r.get(0),
        )
        .expect("count query");
    assert_eq!(count, 1, "verification record must be written to kernel_verifications");
}

// ---------------------------------------------------------------------------
// test_recover_dry_run
// WHY: dry_run=true must skip the real checkpoint command but still return Ok
// (the function-level contract). The caller log path exercises the code path
// without side effects — safe for CI.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_recover_dry_run() {
    let cfg = RecoveryConfig {
        ntfy_topic: "test-topic".to_string(),
        channels: vec![],   // no channels → communicate() is a no-op
        dry_run: true,
        db_path: None,
    };

    // Critical + dry_run → checkpoint/SSH/reap skipped, must return Ok
    let result = recover(Severity::Critical, None, &cfg).await;
    assert!(result.is_ok(), "dry_run critical recovery must return Ok, got: {result:?}");
}

// ---------------------------------------------------------------------------
// test_audio_routing_resolve
// WHY: when active_node is set in kernel_config and is within 8 hours,
// resolve_active_node must return that hostname with source=Explicit.
// ---------------------------------------------------------------------------

#[test]
fn test_audio_routing_resolve() {
    let conn = make_audio_conn();

    // Insert active_node set 2 minutes ago (well within 8-hour window)
    conn.execute_batch(
        "INSERT INTO kernel_config(key, value)
             VALUES ('active_node', 'macProM1');
         INSERT INTO kernel_config(key, value)
             VALUES ('active_node_set_at', datetime('now', '-2 minutes'));",
    )
    .expect("seed kernel_config");

    let node = resolve_active_node(&conn);

    assert_eq!(node.hostname, "macProM1");
    assert_eq!(node.source, ActiveNodeSource::Explicit);
    assert!(!node.is_local(), "macProM1 is not localhost");
}
