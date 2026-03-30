use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

async fn body_json(body: Body) -> Value {
    let bytes = axum::body::to_bytes(body, 131072).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn audit_log_filter_by_agent() {
    let (app, db) = super::api_audit_tests::test_router();
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute_batch(
        "INSERT INTO audit_log (agent, action) VALUES ('alice', 'task_update');
         INSERT INTO audit_log (agent, action) VALUES ('bob', 'task_update');",
    ).unwrap();
    drop(conn);

    let resp = app
        .oneshot(Request::builder().uri("/api/audit/log?agent=alice").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["total"], 1);
    assert_eq!(json["entries"][0]["agent"], "alice");
}
