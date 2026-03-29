use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use serde_json::Value;
use std::sync::{Mutex, OnceLock};
use tower::ServiceExt;

use crate::server::sync_runtime_status::SyncRuntimeStatusHolder;

static STATUS_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn status_lock() -> &'static Mutex<()> {
    STATUS_LOCK.get_or_init(|| Mutex::new(()))
}

fn test_router_with_sync_meta(seed_sql: &str) -> axum::Router {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(10_000);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let db_path = std::env::temp_dir()
        .join(format!("claude-sync-status-test-{}-{n}.db", std::process::id()));

    let conn = rusqlite::Connection::open(&db_path).expect("open db");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _sync_meta (
            peer TEXT NOT NULL,
            table_name TEXT NOT NULL,
            last_sync_at TEXT NOT NULL,
            PRIMARY KEY (peer, table_name)
        );",
    )
    .expect("create _sync_meta");
    if !seed_sql.trim().is_empty() {
        conn.execute_batch(seed_sql).expect("seed _sync_meta");
    }
    drop(conn);

    super::super::middleware::set_dev_mode(true);
    super::super::routes::build_router_with_db(
        std::path::PathBuf::from("."),
        db_path,
        None,
    )
}

async fn get_status(router: axum::Router) -> (StatusCode, Value) {
    let resp = router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/sync/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request");
    let status = resp.status();
    let body = axum::body::to_bytes(resp.into_body(), 1_048_576)
        .await
        .expect("body bytes");
    let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    (status, json)
}

#[tokio::test]
async fn sync_status_exposes_daemon_first_policy_and_success_snapshot() {
    let _guard = status_lock().lock().expect("status lock");
    let holder = SyncRuntimeStatusHolder::new_daemon_first();
    holder.reset();
    holder.mark_success("2026-03-30T12:00:00Z");

    let router = test_router_with_sync_meta(
        "INSERT INTO _sync_meta(peer, table_name, last_sync_at) VALUES
         ('peer-a:8420', 'tasks', '2026-03-30T11:59:00Z'),
         ('peer-a:8420', 'plans', '2026-03-30T11:58:00Z');",
    );
    let (status, body) = get_status(router).await;
    assert_eq!(status, StatusCode::OK, "status endpoint failed: {body}");
    assert_eq!(body["healthy"], true);
    assert_eq!(body["last_success_at"], "2026-03-30T12:00:00Z");
    assert_eq!(body["transport_mode"], "daemon-http");
    assert_eq!(body["fallback_policy"], "manual-rsync-only");
    assert_eq!(body["peer_count"], 1);
    assert_eq!(body["table_count"], 2);
}

#[tokio::test]
async fn sync_status_exposes_last_error_when_runtime_unhealthy() {
    let _guard = status_lock().lock().expect("status lock");
    let holder = SyncRuntimeStatusHolder::new_daemon_first();
    holder.reset();
    holder.mark_error("peer query failed: timeout");

    let router = test_router_with_sync_meta("");
    let (status, body) = get_status(router).await;
    assert_eq!(status, StatusCode::OK, "status endpoint failed: {body}");
    assert_eq!(body["healthy"], false);
    assert_eq!(body["last_success_at"], Value::Null);
    assert_eq!(body["last_error"], "peer query failed: timeout");
    assert_eq!(body["transport_mode"], "daemon-http");
    assert_eq!(body["fallback_policy"], "manual-rsync-only");
}
