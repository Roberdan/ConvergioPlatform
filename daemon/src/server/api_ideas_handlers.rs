// api_ideas_handlers: idea update/delete, notes, and promote handlers
use super::state::{query_one, query_rows, ApiError, ServerState};
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
pub struct UpdateIdeaBody {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    tags: Option<String>,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    project_id: Option<String>,
    #[serde(default)]
    links: Option<String>,
}

pub async fn update_idea(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateIdeaBody>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let mut sets: Vec<String> = vec!["updated_at=datetime('now')".to_string()];
    let mut vals: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(v) = body.title {
        sets.push(format!("title=?{}", vals.len() + 1));
        vals.push(Box::new(v));
    }
    if let Some(v) = body.description {
        sets.push(format!("description=?{}", vals.len() + 1));
        vals.push(Box::new(v));
    }
    if let Some(v) = body.tags {
        sets.push(format!("tags=?{}", vals.len() + 1));
        vals.push(Box::new(v));
    }
    if let Some(v) = body.priority {
        sets.push(format!("priority=?{}", vals.len() + 1));
        vals.push(Box::new(v));
    }
    if let Some(v) = body.status {
        sets.push(format!("status=?{}", vals.len() + 1));
        vals.push(Box::new(v));
    }
    if let Some(v) = body.project_id {
        sets.push(format!("project_id=?{}", vals.len() + 1));
        vals.push(Box::new(v));
    }
    if let Some(v) = body.links {
        sets.push(format!("links=?{}", vals.len() + 1));
        vals.push(Box::new(v));
    }
    vals.push(Box::new(id));
    let sql = format!(
        "UPDATE ideas SET {} WHERE id=?{}",
        sets.join(","),
        vals.len()
    );
    let refs: Vec<&dyn rusqlite::ToSql> = vals.iter().map(|v| v.as_ref()).collect();
    conn.execute(&sql, refs.as_slice())
        .map_err(|e| ApiError::internal(format!("update idea failed: {e}")))?;
    let idea = query_one(
        &conn,
        "SELECT id,title,description,tags,priority,status,project_id,links,plan_id,created_at,updated_at FROM ideas WHERE id=?1",
        rusqlite::params![id],
    )?.ok_or_else(|| ApiError::bad_request(format!("idea {id} not found")))?;
    Ok(Json(idea))
}

pub async fn delete_idea(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    conn.execute("DELETE FROM ideas WHERE id=?1", rusqlite::params![id])
        .map_err(|e| ApiError::internal(format!("delete idea failed: {e}")))?;
    Ok(Json(json!({"ok": true, "id": id})))
}

pub async fn list_notes(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let notes = query_rows(
        &conn,
        "SELECT id,idea_id,content,created_at FROM idea_notes WHERE idea_id=?1 ORDER BY id",
        rusqlite::params![id],
    )?;
    Ok(Json(Value::Array(notes)))
}

#[derive(Deserialize)]
pub struct AddNoteBody {
    pub content: String,
}

pub async fn add_note(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
    Json(body): Json<AddNoteBody>,
) -> Result<Json<Value>, ApiError> {
    let content = body.content.trim().to_string();
    if content.is_empty() {
        return Err(ApiError::bad_request("content is required"));
    }
    let conn = state.get_conn()?;
    conn.execute(
        "INSERT INTO idea_notes (idea_id,content) VALUES (?1,?2)",
        rusqlite::params![id, content],
    )
    .map_err(|e| ApiError::internal(format!("add note failed: {e}")))?;
    let note_id = conn.last_insert_rowid();
    let note = query_one(
        &conn,
        "SELECT id,idea_id,content,created_at FROM idea_notes WHERE id=?1",
        rusqlite::params![note_id],
    )?
    .unwrap_or_else(|| json!({"id": note_id}));
    Ok(Json(note))
}

pub async fn promote_idea(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    conn.execute(
        "UPDATE ideas SET status='promoted', updated_at=datetime('now') WHERE id=?1",
        rusqlite::params![id],
    )
    .map_err(|e| ApiError::internal(format!("promote failed: {e}")))?;
    let idea = query_one(
        &conn,
        "SELECT id,title,description,tags,priority,status,project_id,links,plan_id,created_at,updated_at FROM ideas WHERE id=?1",
        rusqlite::params![id],
    )?.ok_or_else(|| ApiError::bad_request(format!("idea {id} not found")))?;
    Ok(Json(idea))
}
