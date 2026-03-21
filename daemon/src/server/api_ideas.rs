// api_ideas: router + list/get/create handlers
// update/delete/notes/promote handlers → api_ideas_handlers.rs
use super::api_ideas_handlers::{add_note, delete_idea, list_notes, promote_idea, update_idea};
use super::state::{query_one, query_rows, ApiError, ServerState};
use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/ideas", get(list_ideas).post(create_idea))
        .route(
            "/api/ideas/:id",
            get(get_idea).put(update_idea).delete(delete_idea),
        )
        .route("/api/ideas/:id/notes", get(list_notes).post(add_note))
        .route("/api/ideas/:id/promote", post(promote_idea))
}

async fn list_ideas(
    State(state): State<ServerState>,
    Query(qs): Query<HashMap<String, String>>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let mut conditions: Vec<String> = Vec::new();
    let mut params: Vec<String> = Vec::new();
    if let Some(s) = qs.get("status").filter(|v| !v.is_empty()) {
        conditions.push("status=?".to_string());
        params.push(s.clone());
    }
    if let Some(p) = qs.get("priority").filter(|v| !v.is_empty()) {
        conditions.push("priority=?".to_string());
        params.push(p.clone());
    }
    if let Some(proj) = qs.get("project_id").filter(|v| !v.is_empty()) {
        conditions.push("project_id=?".to_string());
        params.push(proj.clone());
    }
    if let Some(tag) = qs.get("tag").filter(|v| !v.is_empty()) {
        conditions.push("tags LIKE ?".to_string());
        params.push(format!("%{}%", tag));
    }
    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };
    let sql = format!(
        "SELECT id,title,description,tags,priority,status,project_id,links,plan_id,created_at,updated_at FROM ideas{} ORDER BY id DESC",
        where_clause
    );
    let rows = conn
        .prepare(&sql)
        .and_then(|mut stmt| {
            let idx: Vec<&dyn rusqlite::ToSql> =
                params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();
            stmt.query_map(idx.as_slice(), |row| {
                Ok(json!({
                    "id": row.get::<_,i64>(0)?,
                    "title": row.get::<_,Option<String>>(1)?,
                    "description": row.get::<_,Option<String>>(2)?,
                    "tags": row.get::<_,Option<String>>(3)?,
                    "priority": row.get::<_,Option<String>>(4)?,
                    "status": row.get::<_,Option<String>>(5)?,
                    "project_id": row.get::<_,Option<String>>(6)?,
                    "links": row.get::<_,Option<String>>(7)?,
                    "plan_id": row.get::<_,Option<i64>>(8)?,
                    "created_at": row.get::<_,Option<String>>(9)?,
                    "updated_at": row.get::<_,Option<String>>(10)?
                }))
            })
            .and_then(|mapped| mapped.collect::<Result<Vec<_>, _>>())
        })
        .map_err(|e| ApiError::internal(format!("list ideas failed: {e}")))?;
    Ok(Json(Value::Array(rows)))
}

async fn get_idea(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let idea = query_one(
        &conn,
        "SELECT id,title,description,tags,priority,status,project_id,links,plan_id,created_at,updated_at FROM ideas WHERE id=?1",
        rusqlite::params![id],
    )?.ok_or_else(|| ApiError::bad_request(format!("idea {id} not found")))?;
    let notes = query_rows(
        &conn,
        "SELECT id,idea_id,content,created_at FROM idea_notes WHERE idea_id=?1 ORDER BY id",
        rusqlite::params![id],
    )
    .unwrap_or_default();
    Ok(Json(json!({"idea": idea, "notes": notes})))
}

#[derive(Deserialize)]
struct CreateIdeaBody {
    title: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    tags: Option<String>,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    project_id: Option<String>,
    #[serde(default)]
    links: Option<String>,
}

async fn create_idea(
    State(state): State<ServerState>,
    Json(body): Json<CreateIdeaBody>,
) -> Result<Json<Value>, ApiError> {
    let title = body.title.trim().to_string();
    if title.is_empty() {
        return Err(ApiError::bad_request("title is required"));
    }
    let conn = state.get_conn()?;
    conn
        .execute(
            "INSERT INTO ideas (title,description,tags,priority,project_id,links) VALUES (?1,?2,?3,?4,?5,?6)",
            rusqlite::params![title, body.description, body.tags, body.priority, body.project_id, body.links],
        )
        .map_err(|e| ApiError::internal(format!("create idea failed: {e}")))?;
    let id = conn.last_insert_rowid();
    let idea = query_one(
        &conn,
        "SELECT id,title,description,tags,priority,status,project_id,links,plan_id,created_at,updated_at FROM ideas WHERE id=?1",
        rusqlite::params![id],
    )?.unwrap_or_else(|| json!({"id": id}));
    Ok(Json(idea))
}
