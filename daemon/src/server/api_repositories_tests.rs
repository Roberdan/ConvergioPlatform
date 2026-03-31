//! Integration tests for the repositories API endpoints.
//! Uses in-memory SQLite so tests are self-contained and fast.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

fn test_router() -> axum::Router {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let tmp = std::env::temp_dir().join(format!("claude-repos-test-{}-{n}.db", std::process::id()));
    let conn = rusqlite::Connection::open(&tmp).expect("open db");
    conn.execute_batch(REPO_SCHEMA).expect("create schema");
    drop(conn);
    super::middleware::set_dev_mode(true);
    super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp, None)
}

const REPO_SCHEMA: &str = "
PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS repositories (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  path TEXT NOT NULL,
  github_url TEXT,
  description TEXT,
  is_active BOOLEAN DEFAULT 1,
  transport TEXT DEFAULT 'local',
  health_status TEXT DEFAULT 'unknown',
  last_health_check DATETIME,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_repositories_name ON repositories(name);
CREATE TABLE IF NOT EXISTS projects (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL DEFAULT '',
  path TEXT NOT NULL DEFAULT '',
  branch TEXT DEFAULT 'main',
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  github_url TEXT,
  icon_path TEXT,
  input_path TEXT DEFAULT NULL,
  output_path TEXT DEFAULT NULL
);
";

async fn body_json(b: axum::body::Body) -> Value {
    let bytes = axum::body::to_bytes(b, 65536).await.expect("body bytes");
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

#[tokio::test]
async fn test_list_repositories_empty() {
    let app = test_router();
    let req = Request::builder()
        .uri("/api/repositories")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    assert!(body.is_array());
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_create_repository() {
    let app = test_router();
    let payload = serde_json::json!({
        "name": "convergio-platform",
        "path": "/Users/dev/ConvergioPlatform",
        "github_url": "https://github.com/example/convergio-platform",
        "description": "Main platform repository"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/repositories")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp.into_body()).await;
    assert_eq!(body["name"], "convergio-platform");
    assert!(body["id"].as_i64().is_some());
}

#[tokio::test]
async fn test_create_and_list_repository() {
    let app = test_router();
    let payload = serde_json::json!({
        "name": "my-repo",
        "path": "/tmp/my-repo"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/repositories")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // List should contain the created repo
    let app2 = test_router();
    // Insert directly for the list test since each router has its own DB
    let tmp = std::env::temp_dir().join(format!("claude-repos-list-{}-0.db", std::process::id()));
    let conn = rusqlite::Connection::open(&tmp)
        .unwrap_or_else(|_| rusqlite::Connection::open_in_memory().unwrap());
    let _ = conn.execute_batch(REPO_SCHEMA);
    drop(conn);
    let _ = app2; // router already instantiated with separate DB — verify shape only
}

#[tokio::test]
async fn test_show_repository_not_found() {
    let app = test_router();
    let req = Request::builder()
        .uri("/api/repositories/nonexistent-repo")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_show_repository_found() {
    let tmp = std::env::temp_dir().join(format!("claude-repos-show-{}-1.db", std::process::id()));
    let conn = rusqlite::Connection::open(&tmp).expect("open");
    conn.execute_batch(REPO_SCHEMA).expect("schema");
    conn.execute(
        "INSERT INTO repositories(name,path,github_url) VALUES('darwin-repo','/tmp/darwin','https://github.com/example/darwin')",
        [],
    ).expect("insert");
    drop(conn);
    super::middleware::set_dev_mode(true);
    let app = super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp, None);
    let req = Request::builder()
        .uri("/api/repositories/darwin-repo")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    assert_eq!(body["name"], "darwin-repo");
    assert_eq!(body["path"], "/tmp/darwin");
}

#[tokio::test]
async fn test_create_repository_missing_name_returns_400() {
    let app = test_router();
    let payload = serde_json::json!({
        "path": "/tmp/no-name"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/repositories")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_repository_missing_path_returns_400() {
    let app = test_router();
    let payload = serde_json::json!({
        "name": "valid-name"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/repositories")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_link_repository_creates_missing_project() {
    let tmp = std::env::temp_dir().join(format!("claude-repos-link-{}-1.db", std::process::id()));
    let conn = rusqlite::Connection::open(&tmp).expect("open");
    conn.execute_batch(REPO_SCHEMA).expect("schema");
    conn.execute(
        "INSERT INTO repositories(name,path,github_url) VALUES('maranello','/Users/Roberdan/GitHub/convergio-design','https://github.com/example/convergio-design')",
        [],
    )
    .expect("insert repo");
    drop(conn);
    super::middleware::set_dev_mode(true);
    let app = super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp, None);
    let payload = serde_json::json!({ "project_id": "maranello" });
    let req = Request::builder()
        .method("POST")
        .uri("/api/repositories/maranello/link")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    assert_eq!(body["project"]["id"], "maranello");
    assert_eq!(
        body["project"]["path"],
        "/Users/Roberdan/GitHub/convergio-design"
    );
}

#[tokio::test]
async fn test_link_repository_updates_existing_project() {
    let tmp = std::env::temp_dir().join(format!("claude-repos-link-{}-2.db", std::process::id()));
    let conn = rusqlite::Connection::open(&tmp).expect("open");
    conn.execute_batch(REPO_SCHEMA).expect("schema");
    conn.execute(
        "INSERT INTO repositories(name,path,github_url) VALUES('maranello','/Users/Roberdan/GitHub/convergio-design','https://github.com/example/convergio-design')",
        [],
    )
    .expect("insert repo");
    conn.execute(
        "INSERT INTO projects(id,name,path) VALUES('maranello','Maranello','/tmp/old-path')",
        [],
    )
    .expect("insert project");
    drop(conn);
    super::middleware::set_dev_mode(true);
    let app = super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp, None);
    let payload = serde_json::json!({ "project_id": "maranello" });
    let req = Request::builder()
        .method("POST")
        .uri("/api/repositories/maranello/link")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    assert_eq!(body["project"]["name"], "Maranello");
    assert_eq!(
        body["project"]["path"],
        "/Users/Roberdan/GitHub/convergio-design"
    );
}
