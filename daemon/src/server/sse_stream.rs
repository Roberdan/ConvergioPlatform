// SSE helper utilities and tests extracted from sse.rs (250-line split).
use super::state::ApiError;
use std::collections::HashMap;

/// Resolve peer name to tailscale_ip from peers.conf
pub fn resolve_peer_ip(home: &str, peer: &str) -> Option<String> {
    let path = format!("{home}/.claude/config/peers.conf");
    let content = std::fs::read_to_string(path).ok()?;
    let mut in_section = false;
    for line in content.lines() {
        let trimmed = line.split('#').next().unwrap_or("").trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let section = trimmed.trim_start_matches('[').trim_end_matches(']');
            in_section = section == peer;
            continue;
        }
        if in_section {
            if let Some((k, v)) = trimmed.split_once('=') {
                if k.trim() == "tailscale_ip" {
                    return Some(v.trim().to_string());
                }
            }
        }
    }
    None
}

pub fn required(qs: &HashMap<String, String>, name: &str) -> Result<String, ApiError> {
    qs.get(name)
        .cloned()
        .filter(|v| !v.is_empty())
        .ok_or_else(|| ApiError::bad_request(format!("missing {name}")))
}

#[cfg(test)]
mod tests {
    use super::super::routes::build_router_with_db;
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    use tower::util::ServiceExt;

    static NEXT_ID: AtomicU64 = AtomicU64::new(1);

    fn test_db_path() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let unique = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("claude-core-sse-{suffix}-{unique}.db"))
    }

    fn seed_db(path: &PathBuf) {
        let conn = rusqlite::Connection::open(path).expect("open db");
        conn.execute_batch(
            "CREATE TABLE plans (id INTEGER PRIMARY KEY, status TEXT, execution_host TEXT);\
             INSERT INTO plans(id,status) VALUES (1,'doing'),(2,'todo');",
        )
        .expect("seed");
    }

    #[tokio::test]
    async fn plan_preflight_requires_plan_id_and_target() {
        let db = test_db_path();
        seed_db(&db);
        let app = build_router_with_db(PathBuf::from("/tmp"), db.clone(), None);

        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/plan/preflight")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("preflight");
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        fs::remove_file(db).ok();
    }

    #[tokio::test]
    async fn plan_start_updates_plan_status_and_emits_done_event() {
        let db = test_db_path();
        seed_db(&db);
        let app = build_router_with_db(PathBuf::from("/tmp"), db.clone(), None);

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/plan/start?plan_id=2&target=local&cli=copilot")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("start");
        assert_eq!(res.status(), StatusCode::OK);
        let body = to_bytes(res.into_body(), usize::MAX).await.expect("body");
        let payload = String::from_utf8_lossy(&body);
        assert!(
            payload.contains("event: done"),
            "missing done event: {payload}"
        );

        let conn = rusqlite::Connection::open(&db).expect("open");
        let status: String = conn
            .query_row("SELECT status FROM plans WHERE id=2", [], |row| row.get(0))
            .expect("status");
        assert_eq!(status, "doing");
        fs::remove_file(db).ok();
    }
}
