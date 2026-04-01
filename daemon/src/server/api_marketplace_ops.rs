use crate::server::api_ipc::ensure_ipc_schema;
use crate::server::state::{ApiError, ServerState};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{post, put};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct ServiceRequestBody {
    pub requester_org: String,
    pub service_name: String,
    pub request_payload: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateRequestStatusBody {
    pub status: String,
}

/// POST /api/services/request — create a service request with budget deduction.
async fn create_service_request(
    State(state): State<ServerState>,
    Json(body): Json<ServiceRequestBody>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;

    // Find active provider for the requested service
    let mut stmt = conn
        .prepare(
            "SELECT org_id, metadata FROM ipc_org_services
             WHERE name = ?1 AND status = 'active' LIMIT 1",
        )
        .map_err(|e| ApiError::internal(format!("prepare failed: {e}")))?;
    let provider: Option<(String, Option<String>)> = stmt
        .query_row(rusqlite::params![body.service_name], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .ok();
    let (provider_org, metadata_str) = provider
        .ok_or_else(|| ApiError::not_found("no active provider for service"))?;

    // Extract cost from metadata or default to 0.0
    let cost: f64 = metadata_str
        .and_then(|m| serde_json::from_str::<Value>(&m).ok())
        .and_then(|v| v.get("cost").and_then(|c| c.as_f64()))
        .unwrap_or(0.0);

    let request_id = format!("svcreq-{}", Uuid::new_v4().simple());
    conn.execute(
        "INSERT INTO ipc_service_requests
         (id, requester_org, provider_org, service_name, status, cost,
          request_payload, created_at)
         VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?6,
          strftime('%Y-%m-%dT%H:%M:%f','now'))",
        rusqlite::params![
            request_id,
            body.requester_org,
            provider_org,
            body.service_name,
            cost,
            body.request_payload
        ],
    )
    .map_err(|e| ApiError::internal(format!("insert request failed: {e}")))?;

    // Record events for both orgs
    record_event(&conn, &body.requester_org, "service_requested", &body.service_name)?;
    record_event(&conn, &provider_org, "service_received", &body.service_name)?;

    // Deduct cost from requester budget
    if cost > 0.0 {
        conn.execute(
            "UPDATE ipc_orgs SET budget = budget - ?1,
             updated_at = strftime('%Y-%m-%dT%H:%M:%f','now')
             WHERE id = ?2",
            rusqlite::params![cost, body.requester_org],
        )
        .map_err(|e| ApiError::internal(format!("budget deduction failed: {e}")))?;
    }

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "ok": true,
            "request_id": request_id,
            "provider_org": provider_org,
            "cost": cost
        })),
    ))
}

/// PUT /api/services/requests/:id — update request status.
async fn update_request_status(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateRequestStatusBody>,
) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;

    let allowed = ["in_progress", "completed", "failed"];
    if !allowed.contains(&body.status.as_str()) {
        return Err(ApiError::bad_request(format!(
            "invalid status '{}'; allowed: in_progress, completed, failed",
            body.status
        )));
    }

    let completed_at = if body.status == "completed" || body.status == "failed" {
        "strftime('%Y-%m-%dT%H:%M:%f','now')"
    } else {
        "NULL"
    };
    let sql = format!(
        "UPDATE ipc_service_requests SET status = ?1, completed_at = {completed_at}
         WHERE id = ?2"
    );
    let changed = conn
        .execute(&sql, rusqlite::params![body.status, id])
        .map_err(|e| ApiError::internal(format!("update request failed: {e}")))?;
    if changed == 0 {
        return Err(ApiError::not_found("service request not found"));
    }

    // On completion, record events for both orgs
    if body.status == "completed" {
        let mut stmt = conn
            .prepare(
                "SELECT requester_org, provider_org, service_name
                 FROM ipc_service_requests WHERE id = ?1",
            )
            .map_err(|e| ApiError::internal(format!("query request failed: {e}")))?;
        if let Ok((req_org, prov_org, svc)) = stmt.query_row(
            rusqlite::params![id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)),
        ) {
            record_event(&conn, &req_org, "service_completed", &svc)?;
            record_event(&conn, &prov_org, "service_fulfilled", &svc)?;
        }
    }

    Ok(Json(json!({ "ok": true, "status": body.status })))
}

fn record_event(
    conn: &rusqlite::Connection,
    org_id: &str,
    event_type: &str,
    description: &str,
) -> Result<(), ApiError> {
    conn.execute(
        "INSERT INTO ipc_org_events (id, org_id, event_type, agent_id, description, created_at)
         VALUES (?1, ?2, ?3, 'system', ?4, strftime('%Y-%m-%dT%H:%M:%f','now'))",
        rusqlite::params![
            format!("evt-{}", Uuid::new_v4().simple()),
            org_id,
            event_type,
            description
        ],
    )
    .map_err(|e| ApiError::internal(format!("record event failed: {e}")))?;
    Ok(())
}

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/services/request", post(create_service_request))
        .route("/api/services/requests/:id", put(update_request_status))
}
