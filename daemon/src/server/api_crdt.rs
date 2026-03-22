// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// HTTP API handlers for CRDT sync status and peer management.
// GET  /api/crdt/status      — CRDT mode, table count, version
// POST /api/crdt/force-sync  — placeholder sync trigger (wired in W3b)
// GET  /api/crdt/peers       — peer list from mesh_sync_stats

use super::state::{ApiError, ServerState};
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/crdt/status", get(handle_crdt_status))
        .route("/api/crdt/force-sync", post(handle_force_sync))
        .route("/api/crdt/peers", get(handle_crdt_peers))
}

async fn handle_crdt_status(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;

    // Count tables that have a crsql_changes shadow — indicates CRSQLite is active
    let crdt_table_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master \
             WHERE type='table' AND name LIKE '%__crsql_clock'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let mode = if crdt_table_count > 0 { "crdt" } else { "wal-only" };

    Ok(Json(json!({
        "mode": mode,
        "tables": crdt_table_count,
        "version": env!("CARGO_PKG_VERSION"),
    })))
}

async fn handle_force_sync(_state: State<ServerState>) -> Result<Json<Value>, ApiError> {
    // Placeholder: actual sync wired in W3b when mesh sync loop is integrated
    Ok(Json(json!({
        "ok": true,
        "message": "sync triggered",
    })))
}

async fn handle_crdt_peers(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;

    let mut stmt = conn
        .prepare(
            "SELECT peer_id, last_sync_at, changes_sent, changes_received, status \
             FROM mesh_sync_stats \
             ORDER BY last_sync_at DESC",
        )
        .map_err(|e| ApiError::internal(format!("prepare failed: {e}")))?;

    let peers: rusqlite::Result<Vec<Value>> = stmt
        .query_map([], |row| {
            Ok(json!({
                "peer_id":           row.get::<_, String>(0)?,
                "last_sync_at":      row.get::<_, Option<String>>(1)?,
                "changes_sent":      row.get::<_, Option<i64>>(2)?,
                "changes_received":  row.get::<_, Option<i64>>(3)?,
                "status":            row.get::<_, Option<String>>(4)?,
            }))
        })
        .map_err(|e| ApiError::internal(format!("query failed: {e}")))?
        .collect();

    let peers = peers.map_err(|e| ApiError::internal(format!("row read failed: {e}")))?;
    Ok(Json(Value::Array(peers)))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use serde_json::Value;
    use tower::ServiceExt;

    const SCHEMA: &str = "
PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS mesh_sync_stats (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    peer_id           TEXT NOT NULL,
    last_sync_at      TEXT,
    changes_sent      INTEGER DEFAULT 0,
    changes_received  INTEGER DEFAULT 0,
    status            TEXT DEFAULT 'idle'
);
CREATE TABLE IF NOT EXISTS plans (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    status TEXT DEFAULT 'todo',
    project_id TEXT
);
CREATE TABLE IF NOT EXISTS daemon_config (
    key TEXT PRIMARY KEY,
    value TEXT
);
CREATE TABLE IF NOT EXISTS agent_activity (
    id INTEGER PRIMARY KEY,
    agent_id TEXT,
    activity TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS host_heartbeats (
    id INTEGER PRIMARY KEY,
    host_id TEXT,
    ts TEXT DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS peer_heartbeats (
    id INTEGER PRIMARY KEY,
    peer_id TEXT,
    ts TEXT DEFAULT (datetime('now'))
);
";

    fn test_router() -> axum::Router {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let tmp = std::env::temp_dir().join(format!(
            "claude-crdt-test-{}-{n}.db",
            std::process::id()
        ));
        let conn = rusqlite::Connection::open(&tmp).expect("open tmp db");
        conn.execute_batch(SCHEMA).expect("schema");
        drop(conn);
        super::super::routes::build_router_with_db(
            std::path::PathBuf::from("/tmp"),
            tmp,
            None,
        )
    }

    async fn get_json(router: axum::Router, path: &str) -> (StatusCode, Value) {
        let resp = router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = resp.status();
        let body = axum::body::to_bytes(resp.into_body(), 1_048_576)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
        (status, json)
    }

    async fn post_json(router: axum::Router, path: &str) -> (StatusCode, Value) {
        let resp = router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(path)
                    .header("Content-Type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = resp.status();
        let body = axum::body::to_bytes(resp.into_body(), 1_048_576)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
        (status, json)
    }

    #[tokio::test]
    async fn test_status() {
        let router = test_router();
        let (status, body) = get_json(router, "/api/crdt/status").await;
        assert_eq!(status, StatusCode::OK, "expected 200: {body}");
        assert!(body.get("mode").is_some(), "response must have 'mode' field: {body}");
        let mode = body["mode"].as_str().unwrap();
        assert!(
            mode == "crdt" || mode == "wal-only",
            "mode must be 'crdt' or 'wal-only', got: {mode}"
        );
        assert!(body.get("tables").is_some(), "response must have 'tables' field: {body}");
        assert!(body.get("version").is_some(), "response must have 'version' field: {body}");
    }

    #[tokio::test]
    async fn test_peers() {
        let router = test_router();
        let (status, body) = get_json(router, "/api/crdt/peers").await;
        assert_eq!(status, StatusCode::OK, "expected 200: {body}");
        assert!(body.is_array(), "response must be a JSON array: {body}");
        // Empty DB → empty array is valid
    }

    #[tokio::test]
    async fn test_peers_with_data() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(100);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let tmp = std::env::temp_dir().join(format!(
            "claude-crdt-test-{}-{n}.db",
            std::process::id()
        ));
        let conn = rusqlite::Connection::open(&tmp).expect("open tmp db");
        conn.execute_batch(SCHEMA).expect("schema");
        conn.execute_batch(
            "INSERT INTO mesh_sync_stats (peer_id, status) VALUES ('peer-alpha', 'active');",
        )
        .expect("seed");
        drop(conn);
        let router = super::super::routes::build_router_with_db(
            std::path::PathBuf::from("/tmp"),
            tmp,
            None,
        );

        let (status, body) = get_json(router, "/api/crdt/peers").await;
        assert_eq!(status, StatusCode::OK, "expected 200: {body}");
        let arr = body.as_array().unwrap();
        assert_eq!(arr.len(), 1, "expected 1 peer: {body}");
        assert_eq!(arr[0]["peer_id"].as_str().unwrap(), "peer-alpha");
        assert_eq!(arr[0]["status"].as_str().unwrap(), "active");
    }

    #[tokio::test]
    async fn test_force_sync() {
        let router = test_router();
        let (status, body) = post_json(router, "/api/crdt/force-sync").await;
        assert_eq!(status, StatusCode::OK, "expected 200: {body}");
        assert_eq!(body["ok"].as_bool().unwrap(), true);
        assert_eq!(body["message"].as_str().unwrap(), "sync triggered");
    }
}
