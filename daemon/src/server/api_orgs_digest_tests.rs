use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;

fn test_router_with_db() -> (axum::Router, std::path::PathBuf) {
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = CTR.fetch_add(1, Ordering::SeqCst);
    let db = std::env::temp_dir().join(format!("org-digest-test-{}-{n}.db", std::process::id()));
    super::middleware::set_dev_mode(true);
    (
        super::routes::build_router_with_db(std::path::PathBuf::from("."), db.clone(), None),
        db,
    )
}

async fn body_json(body: Body) -> Value {
    let bytes = axum::body::to_bytes(body, 131072).await.expect("body");
    serde_json::from_slice(&bytes).expect("json")
}

async fn create_org(app: &axum::Router, id: &str) {
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/orgs")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"id": id, "mission": "m", "objectives": "o", "ceo_agent": "ceo", "budget": 100.0})
                        .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("create org");
}

#[tokio::test]
async fn generate_digest_persists_and_returns_latest() {
    let (app, _) = test_router_with_db();
    create_org(&app, "org-digest").await;

    let generated = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/orgs/org-digest/digest/generate")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("generate");
    assert_eq!(generated.status(), StatusCode::CREATED);

    let latest = app
        .oneshot(
            Request::builder()
                .uri("/api/orgs/org-digest/digest")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("latest digest");
    assert_eq!(latest.status(), StatusCode::OK);
    let json = body_json(latest.into_body()).await;
    assert_eq!(json["digest"]["org_id"], "org-digest");
}

#[tokio::test]
async fn morning_digest_aggregates_active_orgs() {
    let (app, _) = test_router_with_db();
    create_org(&app, "org-a").await;
    create_org(&app, "org-b").await;

    for org in ["org-a", "org-b"] {
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/orgs/{org}/digest/generate"))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("generate digest");
    }

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/digest/morning")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("morning");
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["org_count"], 2);
}

#[tokio::test]
async fn morning_digest_detects_pending_escalations() {
    let (app, db_path) = test_router_with_db();
    create_org(&app, "org-escalate").await;
    let conn = rusqlite::Connection::open(&db_path).expect("open db");
    conn.execute(
        "INSERT INTO ipc_decisions(id, org_id, decision, rationale, decided_by, created_at)
         VALUES ('dec-old', 'org-escalate', 'pending contract', 'needs approval', 'ceo',
                 datetime('now','-8 hours'))",
        [],
    )
    .expect("insert old decision");

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/digest/morning")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("morning");
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["escalations"].as_array().map(|x| x.len()), Some(1));
}
