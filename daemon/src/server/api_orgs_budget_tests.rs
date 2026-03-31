use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;

fn test_router_with_db() -> (axum::Router, std::path::PathBuf) {
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = CTR.fetch_add(1, Ordering::SeqCst);
    let db = std::env::temp_dir().join(format!("orgs-budget-test-{}-{n}.db", std::process::id()));
    super::middleware::set_dev_mode(true);
    (
        super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), db.clone(), None),
        db,
    )
}

#[tokio::test]
async fn member_action_budget_check_passes_under_daily_budget() {
    let (app, _) = test_router_with_db();
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/orgs")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "id": "org-budget-pass",
                        "mission": "Operate safely",
                        "objectives": "Stay under budget",
                        "ceo_agent": "kai",
                        "budget": 1000.0
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("create");

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/orgs/org-budget-pass/members")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"agent":"bob","role":"engineer","department":"platform"}).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("member");
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn member_action_budget_circuit_breaker_suspends_org_and_broadcasts_alert() {
    let (app, db_path) = test_router_with_db();
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/orgs")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "id": "org-budget-breaker",
                        "mission": "Operate safely",
                        "objectives": "Trip breaker",
                        "ceo_agent": "kai",
                        "budget": 1000.0
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("create");

    {
        let conn = rusqlite::Connection::open(&db_path).expect("open db");
        conn.execute(
            "UPDATE ipc_orgs SET daily_budget_tokens = 10 WHERE id = 'org-budget-breaker'",
            [],
        )
        .expect("set daily budget");
        conn.execute(
            "INSERT INTO ipc_org_telemetry(id, org_id, metric, value, tags)
             VALUES ('telemetry-breaker', 'org-budget-breaker', 'tokens_consumed', 31, '{}')",
            [],
        )
        .expect("seed telemetry");
    }

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/orgs/org-budget-breaker/members")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"agent":"ivy","role":"ops","department":"platform"}).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("member");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let conn = rusqlite::Connection::open(&db_path).expect("open db");
    let status: String = conn
        .query_row(
            "SELECT status FROM ipc_orgs WHERE id = 'org-budget-breaker'",
            [],
            |row| row.get(0),
        )
        .expect("org status");
    assert_eq!(status, "suspended");

    let alerts: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ipc_messages
             WHERE channel = 'org:org-budget-breaker'
               AND content LIKE '%budget_circuit_breaker%'",
            [],
            |row| row.get(0),
        )
        .expect("alert count");
    assert_eq!(alerts, 1);
}
