// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// HTTP API handlers for domain→skill mapping.
// GET  /api/domain/list  — returns all rows from domain_skill_map as JSON array
// POST /api/domain/map   — inserts a new mapping {domain, skill_name, description}

use super::state::{ApiError, ServerState};
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/domain/list", get(list_domains))
        .route("/api/domain/map", post(map_domain))
}

async fn list_domains(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let mut stmt = conn
        .prepare(
            "SELECT id, domain, skill_name, description, created_at \
             FROM domain_skill_map \
             ORDER BY domain, skill_name",
        )
        .map_err(|e| ApiError::internal(format!("prepare failed: {e}")))?;

    let rows: rusqlite::Result<Vec<Value>> = stmt
        .query_map([], |row| {
            Ok(json!({
                "id":          row.get::<_, i64>(0)?,
                "domain":      row.get::<_, String>(1)?,
                "skill_name":  row.get::<_, String>(2)?,
                "description": row.get::<_, Option<String>>(3)?,
                "created_at":  row.get::<_, Option<String>>(4)?,
            }))
        })
        .map_err(|e| ApiError::internal(format!("query failed: {e}")))?
        .collect();

    let rows = rows.map_err(|e| ApiError::internal(format!("row read failed: {e}")))?;
    Ok(Json(json!({ "items": rows })))
}

#[derive(Debug, Deserialize)]
struct MapRequest {
    domain: String,
    skill_name: String,
    description: Option<String>,
}

async fn map_domain(
    State(state): State<ServerState>,
    Json(req): Json<MapRequest>,
) -> Result<Json<Value>, ApiError> {
    if req.domain.trim().is_empty() {
        return Err(ApiError::bad_request("domain must not be empty"));
    }
    if req.skill_name.trim().is_empty() {
        return Err(ApiError::bad_request("skill_name must not be empty"));
    }

    let conn = state.get_conn()?;
    conn.execute(
        "INSERT INTO domain_skill_map (domain, skill_name, description) \
         VALUES (?1, ?2, ?3)",
        rusqlite::params![req.domain, req.skill_name, req.description],
    )
    .map_err(|e| {
        // SQLite unique constraint code = 2067 (SQLITE_CONSTRAINT_UNIQUE)
        if e.to_string().contains("UNIQUE") {
            ApiError::conflict(format!(
                "mapping already exists: {}→{}",
                req.domain, req.skill_name
            ))
        } else {
            ApiError::internal(format!("insert failed: {e}"))
        }
    })?;

    Ok(Json(json!({
        "ok": true,
        "domain": req.domain,
        "skill_name": req.skill_name,
    })))
}
