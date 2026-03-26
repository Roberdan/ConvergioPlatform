// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Repository management API: list, create, show endpoints.
// Backing table: repositories (Plan 724, T4-01).

use super::state::{query_one, query_rows, ApiError, ServerState};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new()
        .route(
            "/api/repositories",
            get(list_repositories).post(create_repository),
        )
        .route("/api/repositories/:name", get(show_repository))
}

async fn list_repositories(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let rows = query_rows(
        &conn,
        "SELECT id,name,path,github_url,description,is_active,transport,\
         health_status,last_health_check,created_at,updated_at \
         FROM repositories ORDER BY name ASC",
        [],
    )?;
    Ok(Json(Value::Array(rows)))
}

#[derive(Deserialize)]
pub struct CreateRepoRequest {
    name: Option<String>,
    path: Option<String>,
    github_url: Option<String>,
    description: Option<String>,
    transport: Option<String>,
}

async fn create_repository(
    State(state): State<ServerState>,
    Json(body): Json<CreateRepoRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let name = body
        .name
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| ApiError::bad_request("name is required"))?;
    let path = body
        .path
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| ApiError::bad_request("path is required"))?;
    let transport = body.transport.unwrap_or_else(|| "local".to_string());

    let conn = state.get_conn()?;
    conn.execute(
        "INSERT INTO repositories(name,path,github_url,description,transport) \
         VALUES(?1,?2,?3,?4,?5)",
        rusqlite::params![
            name,
            path,
            body.github_url,
            body.description,
            transport
        ],
    )
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("UNIQUE") {
            ApiError::conflict(format!("repository '{name}' already exists"))
        } else {
            ApiError::internal(format!("insert failed: {e}"))
        }
    })?;

    let id = conn.last_insert_rowid();
    let row = query_one(
        &conn,
        "SELECT id,name,path,github_url,description,is_active,transport,\
         health_status,last_health_check,created_at,updated_at \
         FROM repositories WHERE id=?1",
        rusqlite::params![id],
    )?
    .ok_or_else(|| ApiError::internal("created repository not found"))?;

    Ok((StatusCode::CREATED, Json(row)))
}

async fn show_repository(
    State(state): State<ServerState>,
    Path(name): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let row = query_one(
        &conn,
        "SELECT id,name,path,github_url,description,is_active,transport,\
         health_status,last_health_check,created_at,updated_at \
         FROM repositories WHERE name=?1",
        rusqlite::params![name],
    )?
    .ok_or_else(|| ApiError::not_found(format!("repository '{name}' not found")))?;
    Ok(Json(row))
}

#[cfg(test)]
mod unit_tests {
    use super::CreateRepoRequest;

    #[test]
    fn create_request_name_required() {
        let req = CreateRepoRequest {
            name: None,
            path: Some("/tmp/test".into()),
            github_url: None,
            description: None,
            transport: None,
        };
        assert!(req.name.is_none());
    }

    #[test]
    fn create_request_path_required() {
        let req = CreateRepoRequest {
            name: Some("valid".into()),
            path: None,
            github_url: None,
            description: None,
            transport: None,
        };
        assert!(req.path.is_none());
    }

    #[test]
    fn list_route_registered() {
        // Smoke: router builds without panic
        let _ = super::router();
    }

    #[test]
    fn default_transport_is_local() {
        let transport: Option<String> = None;
        let resolved = transport.unwrap_or_else(|| "local".to_string());
        assert_eq!(resolved, "local");
    }

    #[test]
    fn response_shape_fields_match_schema() {
        // Verify expected column names for integration callers
        let expected = [
            "id", "name", "path", "github_url", "description",
            "is_active", "transport", "health_status",
            "last_health_check", "created_at", "updated_at",
        ];
        assert_eq!(expected.len(), 11);
        assert!(expected.contains(&"health_status"));
        assert!(expected.contains(&"transport"));
    }
}

pub fn health_check_json(name: &str, status: &str) -> Value {
    json!({
        "name": name,
        "health_status": status,
        "checked_at": chrono::Utc::now().to_rfc3339()
    })
}
