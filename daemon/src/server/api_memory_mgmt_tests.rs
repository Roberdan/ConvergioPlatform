//! Tests for memory management API (api_memory_mgmt.rs).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn setup_test_env() -> (axum::Router, std::path::PathBuf, String) {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let base = std::env::temp_dir().join(format!(
        "claude-memmgmt-test-{}",
        std::process::id()
    ));
    let slug = format!("test-project-{n}");
    let mem_dir = base.join(&slug).join("memory");
    fs::create_dir_all(&mem_dir).expect("create mem dir");
    std::env::set_var("CLAUDE_PROJECTS_DIR", base.to_str().unwrap());

    // Seed memory files
    fs::write(
        mem_dir.join("user_role.md"),
        "---\nname: user_role\ntype: user\ndescription: User is a developer\n---\nSenior dev.",
    )
    .unwrap();
    fs::write(
        mem_dir.join("feedback_testing.md"),
        "---\nname: feedback_testing\ntype: feedback\ndescription: Always run tests\n---\nRun tests first.",
    )
    .unwrap();
    fs::write(
        mem_dir.join("project_goal.md"),
        "---\nname: project_goal\ntype: project\ndescription: Current sprint goal\n---\nShip v2.",
    )
    .unwrap();
    fs::write(mem_dir.join("MEMORY.md"), "# Index\n- user_role.md\n").unwrap();

    let db_tmp = std::env::temp_dir().join(format!(
        "claude-memmgmt-db-{n}-{}.db",
        std::process::id()
    ));
    super::middleware::set_dev_mode(true);
    let router =
        super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), db_tmp, None);
    (router, mem_dir, slug.to_string())
}

async fn do_get(router: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let body = axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

async fn do_post(router: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .uri(uri)
        .method("POST")
        .body(Body::empty())
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let body = axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

async fn do_delete(router: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .uri(uri)
        .method("DELETE")
        .body(Body::empty())
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let body = axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

#[tokio::test]
async fn memory_mgmt_list() {
    let (r, _dir, slug) = setup_test_env();
    let (s, j) = do_get(&r, &format!("/api/memory-mgmt/list?slug={slug}")).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    let memories = j["memories"].as_array().expect("array");
    assert_eq!(memories.len(), 3, "3 memory files (excludes MEMORY.md)");
    let types: Vec<&str> = memories
        .iter()
        .filter_map(|m| m["type"].as_str())
        .collect();
    assert!(types.contains(&"user"));
    assert!(types.contains(&"feedback"));
    assert!(types.contains(&"project"));
}

#[tokio::test]
async fn memory_mgmt_stats() {
    let (r, _dir, slug) = setup_test_env();
    let (s, j) = do_get(&r, &format!("/api/memory-mgmt/stats?slug={slug}")).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert_eq!(j["total"], 3);
    assert!(j["estimated_tokens"].as_u64().unwrap() > 0);
    assert!(j["by_type"]["user"].as_u64().unwrap() >= 1);
    assert!(j["by_type"]["feedback"].as_u64().unwrap() >= 1);
}

#[tokio::test]
async fn memory_mgmt_delete() {
    let (r, dir, slug) = setup_test_env();
    assert!(dir.join("project_goal.md").exists());
    let (s, j) = do_delete(
        &r,
        &format!("/api/memory-mgmt/file/project_goal.md?slug={slug}"),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert!(!dir.join("project_goal.md").exists());
}

#[tokio::test]
async fn memory_mgmt_delete_not_found() {
    let (r, _dir, slug) = setup_test_env();
    let (s, _) = do_delete(
        &r,
        &format!("/api/memory-mgmt/file/nonexistent.md?slug={slug}"),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn memory_mgmt_delete_path_traversal() {
    let (r, _dir, slug) = setup_test_env();
    let (s, _) = do_delete(
        &r,
        &format!("/api/memory-mgmt/file/..%2F..%2Fetc%2Fpasswd?slug={slug}"),
    )
    .await;
    assert!(s == StatusCode::BAD_REQUEST || s == StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn memory_mgmt_gc() {
    let (r, _dir, slug) = setup_test_env();
    let (s, j) = do_post(&r, &format!("/api/memory-mgmt/gc?slug={slug}")).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert!(j["deleted"].is_array());
    assert!(j["archived"].is_array());
    assert!(j["kept"].is_array());
}

#[tokio::test]
async fn memory_mgmt_list_missing_dir() {
    let (r, _dir, _slug) = setup_test_env();
    let (s, j) = do_get(&r, "/api/memory-mgmt/list?slug=nonexistent-slug").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert_eq!(j["memories"].as_array().unwrap().len(), 0);
}

#[test]
fn parse_frontmatter_valid() {
    let content = "---\nname: test\ntype: feedback\ndescription: A test memory\n---\nBody here.";
    let (name, t, desc) = super::api_memory_mgmt::parse_frontmatter(content);
    assert_eq!(name.as_deref(), Some("test"));
    assert_eq!(t.as_deref(), Some("feedback"));
    assert_eq!(desc.as_deref(), Some("A test memory"));
}

#[test]
fn parse_frontmatter_missing() {
    let (n, t, d) = super::api_memory_mgmt::parse_frontmatter("No frontmatter here.");
    assert!(n.is_none() && t.is_none() && d.is_none());
}
