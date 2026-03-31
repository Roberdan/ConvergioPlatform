use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;

fn test_router_with_db() -> (axum::Router, std::path::PathBuf) {
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = CTR.fetch_add(1, Ordering::SeqCst);
    let db = std::env::temp_dir().join(format!("orgs-integ-{}-{n}.db", std::process::id()));
    super::super::middleware::set_dev_mode(true);
    (
        super::super::routes::build_router_with_db(std::path::PathBuf::from("."), db.clone(), None),
        db,
    )
}

async fn body_json(body: Body) -> Value {
    let bytes = to_bytes(body, 1_000_000).await.expect("body bytes");
    serde_json::from_slice(&bytes).expect("json body")
}

async fn post(app: &axum::Router, uri: &str, payload: Value) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .expect("request"),
        )
        .await
        .expect("response");
    let status = resp.status();
    let body = body_json(resp.into_body()).await;
    (status, body)
}

async fn get(app: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).expect("request"))
        .await
        .expect("response");
    let status = resp.status();
    let body = body_json(resp.into_body()).await;
    (status, body)
}

#[tokio::test]
async fn full_agent_network_flow_end_to_end() {
    let (app, db_path) = test_router_with_db();

    let (s, _) = post(
        &app,
        "/api/orgs",
        json!({"id":"org-alpha","mission":"m1","objectives":"o1","ceo_agent":"alpha-ceo","budget":1000.0}),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED);
    let (s, _) = post(
        &app,
        "/api/orgs",
        json!({"id":"org-beta","mission":"m2","objectives":"o2","ceo_agent":"beta-ceo","budget":900.0}),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED);

    let (s, _) = post(
        &app,
        "/api/orgs/org-alpha/members",
        json!({"agent":"dan","role":"lead","department":"engineering"}),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED);
    let (s, _) = post(
        &app,
        "/api/orgs/org-alpha/members",
        json!({"agent":"rex","role":"reviewer","department":"engineering"}),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED);
    let (s, _) = post(
        &app,
        "/api/orgs/org-alpha/services",
        json!({"name":"planner","endpoint":"https://svc.local/planner","status":"active"}),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED);

    let (s, _) = post(
        &app,
        "/api/ipc/send-direct",
        json!({"from":"dan","to":"rex","content":"status update?"}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (s, _) = post(
        &app,
        "/api/ipc/send-direct",
        json!({"from":"rex","to":"dan","content":"ready to ship"}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);

    let (s, j) = get(&app, "/api/ipc/messages?to_agent=rex&limit=10").await;
    assert_eq!(s, StatusCode::OK);
    assert!(
        j["messages"]
            .as_array()
            .expect("messages")
            .iter()
            .any(|m| m["from_agent"] == "dan" && m["to_agent"] == "rex")
    );
    let (s, j) = get(&app, "/api/ipc/messages?to_agent=dan&limit=10").await;
    assert_eq!(s, StatusCode::OK);
    assert!(
        j["messages"]
            .as_array()
            .expect("messages")
            .iter()
            .any(|m| m["from_agent"] == "rex" && m["to_agent"] == "dan")
    );

    let (s, _) = post(
        &app,
        "/api/ipc/send",
        json!({"sender_name":"alpha-ceo","channel":"inter-org:org-alpha:org-beta","content":"sync request"}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (s, j) = get(&app, "/api/ipc/messages?channel=inter-org:org-alpha:org-beta&limit=10").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["messages"][0]["from_agent"], "alpha-ceo");

    let (s, _) = post(
        &app,
        "/api/orgs/org-alpha/decisions",
        json!({"decision":"bootstrap complete","rationale":"ceo bootstrap","made_by":"alpha-ceo","refs":["boot-1"]}),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED);
    let (s, j) = get(&app, "/api/orgs/org-alpha/decisions").await;
    assert_eq!(s, StatusCode::OK);
    assert!(
        j["decisions"]
            .as_array()
            .expect("decisions")
            .iter()
            .any(|d| d["decision"] == "bootstrap complete")
    );

    let (s, _) = post(
        &app,
        "/api/orgs/org-alpha/telemetry",
        json!({"agent":"dan","tokens_in":120,"tokens_out":80,"cost":0.42,"period":"day"}),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED);
    let (s, j) = get(&app, "/api/orgs/org-alpha/telemetry?period=day").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["aggregate"]["tokens_in"], 120);
    assert_eq!(j["aggregate"]["tokens_out"], 80);

    let stream_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/ipc/stream?agent=rex")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("stream response");
    assert_eq!(stream_resp.status(), StatusCode::OK);
    assert_eq!(
        stream_resp.headers().get("content-type").and_then(|v| v.to_str().ok()),
        Some("text/event-stream")
    );

    let (s, j) = get(&app, "/api/orgs/org-alpha").await;
    assert_eq!(s, StatusCode::OK);
    assert!(j["org"]["id"] == "org-alpha");
    assert!(
        j["members"]
            .as_array()
            .expect("members")
            .iter()
            .any(|m| m["agent"] == "dan")
    );
    assert!(
        j["services"]
            .as_array()
            .expect("services")
            .iter()
            .any(|svc| svc["name"] == "planner")
    );

    let conn = rusqlite::Connection::open(db_path).expect("open db");
    conn.execute("UPDATE ipc_orgs SET daily_budget_tokens = 10 WHERE id = 'org-alpha'", [])
        .expect("set budget");
    conn.execute(
        "INSERT INTO ipc_org_telemetry(id, org_id, metric, value, tags)
         VALUES ('telemetry-breaker-alpha', 'org-alpha', 'tokens_consumed', 31, '{}')",
        [],
    )
    .expect("seed telemetry");
    drop(conn);

    let (s, _) = post(
        &app,
        "/api/orgs/org-alpha/members",
        json!({"agent":"ivy","role":"ops","department":"platform"}),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
}
