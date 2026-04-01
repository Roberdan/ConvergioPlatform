use crate::server::api_ipc::ensure_ipc_schema;
use crate::server::state::{query_rows, ApiError, ServerState};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct TimelineQuery {
    pub since: Option<String>,
    pub until: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Deserialize)]
pub struct CreateEventRequest {
    pub event_type: String,
    pub agent_id: Option<String>,
    pub description: String,
    pub metadata_json: Option<Value>,
}

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/orgs/:slug/timeline", get(get_timeline))
        .route("/api/orgs/:slug/events", post(create_event))
}

async fn get_timeline(
    State(state): State<ServerState>,
    Path(slug): Path<String>,
    Query(params): Query<TimelineQuery>,
) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let limit = params.limit.unwrap_or(100).min(500);

    // Build query with optional date filters
    let mut sql = String::from(
        "SELECT id, org_id, event_type, agent_id, description, metadata_json, created_at \
         FROM ipc_org_events WHERE org_id = ?1",
    );
    let mut bind_idx = 2u32;
    let mut bind_since = None;
    let mut bind_until = None;

    if let Some(ref since) = params.since {
        sql.push_str(&format!(" AND created_at >= ?{bind_idx}"));
        bind_since = Some(since.clone());
        bind_idx += 1;
    }
    if let Some(ref until) = params.until {
        sql.push_str(&format!(" AND created_at <= ?{bind_idx}"));
        bind_until = Some(until.clone());
    }
    sql.push_str(&format!(" ORDER BY created_at DESC LIMIT {limit}"));

    let events = match (&bind_since, &bind_until) {
        (Some(s), Some(u)) => query_rows(&conn, &sql, rusqlite::params![slug, s, u])?,
        (Some(s), None) => query_rows(&conn, &sql, rusqlite::params![slug, s])?,
        (None, Some(u)) => query_rows(&conn, &sql, rusqlite::params![slug, u])?,
        (None, None) => query_rows(&conn, &sql, rusqlite::params![slug])?,
    };

    Ok(Json(json!({ "ok": true, "events": events })))
}

async fn create_event(
    State(state): State<ServerState>,
    Path(slug): Path<String>,
    Json(body): Json<CreateEventRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let id = format!("evt-{}", Uuid::new_v4().simple());
    let meta = body.metadata_json.map(|m| m.to_string());

    conn.execute(
        "INSERT INTO ipc_org_events(id, org_id, event_type, agent_id, description, metadata_json) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![id, slug, body.event_type, body.agent_id, body.description, meta],
    )
    .map_err(|e| ApiError::internal(format!("create event failed: {e}")))?;

    Ok((StatusCode::CREATED, Json(json!({ "ok": true, "event_id": id }))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::state::ServerState;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn setup_state() -> ServerState {
        static CTR: AtomicU64 = AtomicU64::new(0);
        let n = CTR.fetch_add(1, Ordering::SeqCst);
        let db = std::env::temp_dir().join(format!(
            "org-timeline-test-{}-{n}.db",
            std::process::id()
        ));
        let state = ServerState::new(db, None);
        ensure_ipc_schema(&state).unwrap();
        let conn = state.get_conn().unwrap();
        conn.execute(
            "INSERT INTO ipc_orgs(id, mission, objectives, ceo_agent, budget) \
             VALUES ('test-org', 'testing', 'test timeline', 'agent-1', 0)",
            [],
        )
        .unwrap();
        state
    }

    #[tokio::test]
    async fn test_create_and_list_events() {
        let state = setup_state();
        let body = CreateEventRequest {
            event_type: "member_joined".into(),
            agent_id: Some("agent-1".into()),
            description: "Agent joined org".into(),
            metadata_json: Some(json!({"role": "engineer"})),
        };
        let (status, json) = create_event(
            State(state.clone()),
            Path("test-org".into()),
            Json(body),
        )
        .await
        .unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(json.0["ok"], true);

        let result = get_timeline(
            State(state.clone()),
            Path("test-org".into()),
            Query(TimelineQuery { since: None, until: None, limit: None }),
        )
        .await
        .unwrap();
        let events = result.0["events"].as_array().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["event_type"], "member_joined");
    }

    #[tokio::test]
    async fn test_timeline_date_filter() {
        let state = setup_state();
        let conn = state.get_conn().unwrap();
        conn.execute(
            "INSERT INTO ipc_org_events(id, org_id, event_type, description, created_at) \
             VALUES ('evt-old', 'test-org', 'old_event', 'ancient', '2024-01-01T00:00:00.000')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO ipc_org_events(id, org_id, event_type, description, created_at) \
             VALUES ('evt-new', 'test-org', 'new_event', 'recent', '2026-06-01T00:00:00.000')",
            [],
        )
        .unwrap();

        let result = get_timeline(
            State(state.clone()),
            Path("test-org".into()),
            Query(TimelineQuery {
                since: Some("2025-01-01T00:00:00.000".into()),
                until: None,
                limit: None,
            }),
        )
        .await
        .unwrap();
        let events = result.0["events"].as_array().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["id"], "evt-new");
    }

    #[tokio::test]
    async fn test_timeline_limit() {
        let state = setup_state();
        let conn = state.get_conn().unwrap();
        for i in 0..5 {
            conn.execute(
                "INSERT INTO ipc_org_events(id, org_id, event_type, description) \
                 VALUES (?1, 'test-org', 'tick', 'event')",
                rusqlite::params![format!("evt-{i}")],
            )
            .unwrap();
        }

        let result = get_timeline(
            State(state.clone()),
            Path("test-org".into()),
            Query(TimelineQuery { since: None, until: None, limit: Some(3) }),
        )
        .await
        .unwrap();
        let events = result.0["events"].as_array().unwrap();
        assert_eq!(events.len(), 3);
    }
}
