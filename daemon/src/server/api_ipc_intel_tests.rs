// IPC intelligence endpoint integration tests (Plan 742 T5-03)
// Tests: budget, models, skills, auth-status, route-history, metrics, logs
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;

/// Intelligence tables not created by default migrations. Seed before router.
const INTEL_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS ipc_model_registry (
    id INTEGER PRIMARY KEY, host TEXT, provider TEXT, model TEXT,
    size_gb REAL, quantization TEXT, last_seen TEXT, UNIQUE(host,provider,model)
);
CREATE TABLE IF NOT EXISTS ipc_subscriptions (
    name TEXT PRIMARY KEY, provider TEXT, plan TEXT,
    budget_usd REAL, reset_day INTEGER, models TEXT
);
CREATE TABLE IF NOT EXISTS ipc_budget_log (
    id INTEGER PRIMARY KEY, subscription TEXT, date TEXT,
    tokens_in INTEGER, tokens_out INTEGER, estimated_cost_usd REAL,
    model TEXT, task_ref TEXT
);
CREATE TABLE IF NOT EXISTS ipc_agent_skills (
    id INTEGER PRIMARY KEY, agent TEXT, host TEXT, skill TEXT,
    confidence REAL DEFAULT 0.5, last_used TEXT, UNIQUE(agent,host,skill)
);
CREATE TABLE IF NOT EXISTS ipc_node_capabilities (
    host TEXT PRIMARY KEY, provider TEXT, models TEXT, updated_at TEXT
);
CREATE TABLE IF NOT EXISTS session_state (key TEXT PRIMARY KEY, value TEXT);
CREATE TABLE IF NOT EXISTS ipc_auth_tokens (
    id INTEGER PRIMARY KEY, service TEXT, host TEXT, token_hash TEXT,
    status TEXT DEFAULT 'valid', last_checked TEXT, updated_at TEXT,
    UNIQUE(service, host)
);
";

fn make_db_path() -> std::path::PathBuf {
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = CTR.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!(
        "claude-ipc-intel-{}-{n}.db",
        std::process::id()
    ))
}

/// Create router with pre-seeded intelligence tables.
fn test_router() -> axum::Router {
    let tmp = make_db_path();
    // Seed intel tables BEFORE pool/router, so all connections see them
    let conn = rusqlite::Connection::open(&tmp).expect("open seed db");
    conn.execute_batch("PRAGMA journal_mode=WAL;").ok();
    conn.execute_batch(INTEL_SCHEMA).expect("intel schema");
    drop(conn);
    super::middleware::set_dev_mode(true);
    super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp, None)
}

/// Create router with subscription data for budget testing.
fn test_router_with_subscription() -> axum::Router {
    let tmp = make_db_path();
    let conn = rusqlite::Connection::open(&tmp).expect("open");
    conn.execute_batch("PRAGMA journal_mode=WAL;").ok();
    conn.execute_batch(INTEL_SCHEMA).expect("schema");
    conn.execute(
        "INSERT INTO ipc_subscriptions (name, provider, plan, budget_usd, reset_day, models)
         VALUES ('anthropic-pro', 'anthropic', 'pro', 200.0, 1, '[\"claude-opus-4-6\"]')",
        [],
    )
    .expect("seed sub");
    drop(conn);
    super::middleware::set_dev_mode(true);
    super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp, None)
}

async fn get(router: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let resp = router
        .clone()
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

// --- Budget ---

#[tokio::test]
async fn ipc_budget_returns_empty_budgets() {
    let app = test_router();
    let (s, j) = get(&app, "/api/ipc/budget").await;
    assert_eq!(s, StatusCode::OK);
    assert!(j["budgets"].is_array());
    assert!(j["budgets"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn ipc_budget_with_subscription_returns_status() {
    let app = test_router_with_subscription();
    let (s, j) = get(&app, "/api/ipc/budget").await;
    assert_eq!(s, StatusCode::OK);
    let budgets = j["budgets"].as_array().unwrap();
    assert_eq!(budgets.len(), 1);
    assert_eq!(budgets[0]["subscription"], "anthropic-pro");
    assert_eq!(budgets[0]["provider"], "anthropic");
}

// --- Models ---

#[tokio::test]
async fn ipc_models_returns_empty_registry() {
    let app = test_router();
    let (s, j) = get(&app, "/api/ipc/models").await;
    assert_eq!(s, StatusCode::OK);
    assert!(j["models"].is_array());
    assert!(j["capabilities"].is_array());
}

// --- Skills ---

#[tokio::test]
async fn ipc_skills_returns_empty_pool() {
    let app = test_router();
    let (s, j) = get(&app, "/api/ipc/skills").await;
    assert_eq!(s, StatusCode::OK);
    assert!(j["skills"].is_array());
    assert!(j["skills"].as_array().unwrap().is_empty());
}

// --- Auth status ---

#[tokio::test]
async fn ipc_auth_status_returns_health() {
    let app = test_router();
    let (s, j) = get(&app, "/api/ipc/auth-status").await;
    assert_eq!(s, StatusCode::OK);
    assert!(!j["health"].is_null());
    assert!(j["tokens"].is_array());
}

// --- Route history ---

#[tokio::test]
async fn ipc_route_history_returns_empty() {
    let app = test_router();
    let (s, j) = get(&app, "/api/ipc/route-history").await;
    assert_eq!(s, StatusCode::OK);
    assert!(j["history"].is_array());
    assert!(j["history"].as_array().unwrap().is_empty());
}

// --- Metrics ---

#[tokio::test]
async fn ipc_metrics_returns_all_counters() {
    let app = test_router();
    let (s, j) = get(&app, "/api/ipc/metrics").await;
    assert_eq!(s, StatusCode::OK);
    assert!(j["model_count"].is_number());
    assert!(j["agent_count"].is_number());
    assert!(j["ipc_message_rate_1d"].is_number());
    assert!(j["budget_usage"].is_number());
    assert!(j["skill_requests_active"].is_number());
    assert_eq!(j["model_count"], 0);
    assert_eq!(j["agent_count"], 0);
}

// --- Logs ---

#[tokio::test]
async fn ipc_logs_returns_empty_buffer() {
    let app = test_router();
    let (s, j) = get(&app, "/api/ipc/logs").await;
    assert_eq!(s, StatusCode::OK);
    assert!(j["logs"].is_array());
    assert!(j["count"].is_number());
}

#[tokio::test]
async fn ipc_logs_respects_limit_param() {
    let app = test_router();
    let (s, j) = get(&app, "/api/ipc/logs?limit=5").await;
    assert_eq!(s, StatusCode::OK);
    assert!(j["logs"].is_array());
}

#[tokio::test]
async fn ipc_log_buffer_captures_entries() {
    for i in 0..3 {
        super::api_ipc::ipc_log("info", "test", &format!("entry-{i}"));
    }
    let app = test_router();
    let (s, j) = get(&app, "/api/ipc/logs?limit=100").await;
    assert_eq!(s, StatusCode::OK);
    assert!(j["count"].as_i64().unwrap() >= 3);
}
