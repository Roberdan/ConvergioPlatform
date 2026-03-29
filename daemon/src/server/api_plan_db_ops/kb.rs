// KB search/write handlers extracted from handlers.rs.
// Why: keep handlers.rs ≤250 lines per CONSTITUTION Article V.
use crate::server::state::{query_rows, ApiError, ServerState};
use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
pub struct KbSearchQuery {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    20
}

/// GET /api/plan-db/kb-search?q=term — search knowledge_base table
#[tracing::instrument(skip_all)]
pub async fn handle_kb_search(
    State(state): State<ServerState>,
    Query(params): Query<KbSearchQuery>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;

    // Check if knowledge_base table exists
    let table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master \
             WHERE type='table' AND name='knowledge_base'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(false);

    if !table_exists {
        return Ok(Json(json!({
            "ok": true,
            "results": [],
            "query": params.q,
        })));
    }

    let pattern = format!("%{}%", params.q);
    let results = query_rows(
        conn,
        "SELECT id, domain, title, content, created_at, hit_count \
         FROM knowledge_base \
         WHERE title LIKE ?1 OR content LIKE ?1 OR domain LIKE ?1 \
         ORDER BY hit_count DESC, created_at DESC \
         LIMIT ?2",
        rusqlite::params![pattern, params.limit],
    )?;

    Ok(Json(json!({
        "ok": true,
        "results": results,
        "query": params.q,
        "count": results.len(),
    })))
}

/// POST /api/plan-db/kb-write — insert or update a knowledge_base entry
/// Body: {domain, title, content, tags?, confidence?}
#[tracing::instrument(skip_all)]
pub async fn handle_kb_write(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let domain = body
        .get("domain")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing domain"))?;
    let title = body
        .get("title")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing title"))?;
    let content = body
        .get("content")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing content"))?;

    let conn = state.get_conn()?;

    // Try upsert; fall back to plain INSERT if unique constraint is absent
    conn.execute(
        "INSERT INTO knowledge_base (domain, title, content, created_at, hit_count) \
         VALUES (?1, ?2, ?3, datetime('now'), 0) \
         ON CONFLICT(domain, title) DO UPDATE SET \
           content = excluded.content, \
           hit_count = hit_count + 1",
        rusqlite::params![domain, title, content],
    )
    .or_else(|_| {
        conn.execute(
            "INSERT INTO knowledge_base (domain, title, content, created_at, hit_count) \
             VALUES (?1, ?2, ?3, datetime('now'), 0)",
            rusqlite::params![domain, title, content],
        )
    })
    .map_err(|e| ApiError::internal(format!("kb-write failed: {e}")))?;

    let id: i64 = conn
        .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
        .unwrap_or(0);

    Ok(Json(json!({
        "ok": true,
        "id": id,
        "domain": domain,
        "title": title,
    })))
}
