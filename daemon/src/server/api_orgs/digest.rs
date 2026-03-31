use crate::server::api_ipc::ensure_ipc_schema;
use crate::server::state::{query_one, query_rows, ApiError, ServerState};
use axum::extract::{Path, State};
use axum::{http::StatusCode, Json};
use serde_json::{json, Value};
use uuid::Uuid;

pub async fn latest_digest(
    State(state): State<ServerState>,
    Path(org_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let digest = query_one(
        &conn,
        "SELECT id, org_id, type, content, created_at
         FROM ipc_org_digests
         WHERE org_id=?1
         ORDER BY created_at DESC, rowid DESC
         LIMIT 1",
        rusqlite::params![org_id],
    )?
    .ok_or_else(|| ApiError::not_found("digest not found"))?;
    Ok(Json(json!({"ok": true, "digest": digest})))
}

pub async fn generate_digest(
    State(state): State<ServerState>,
    Path(org_id): Path<String>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let content = generate_digest_for_org(&conn, &org_id)?;
    let digest_id = format!("digest-{}", Uuid::new_v4().simple());
    conn.execute(
        "INSERT INTO ipc_org_digests(id, org_id, type, content) VALUES (?1, ?2, 'daily', ?3)",
        rusqlite::params![digest_id, org_id, content.to_string()],
    )
    .map_err(|e| ApiError::internal(format!("insert digest failed: {e}")))?;
    let _ = conn.execute(
        "INSERT INTO ipc_messages(id, channel, from_agent, content)
         VALUES (?1, ?2, 'jarvis', ?3)",
        rusqlite::params![
            format!("msg-{}", Uuid::new_v4().simple()),
            format!("org:{org_id}"),
            format!("daily_digest:{content}"),
        ],
    );
    push_digest_telegram(&format!("📊 Digest {org_id}: {}", compact_digest(&content))).await;
    Ok((StatusCode::CREATED, Json(json!({"ok": true, "id": digest_id, "digest": content}))))
}

pub(crate) fn generate_digest_for_org(
    conn: &rusqlite::Connection,
    org_id: &str,
) -> Result<Value, ApiError> {
    let decisions = query_rows(
        conn,
        "SELECT id, decision, rationale, decided_by, created_at
         FROM ipc_decisions WHERE org_id=?1 ORDER BY created_at DESC LIMIT 20",
        rusqlite::params![org_id],
    )?;
    let telemetry = query_one(
        conn,
        "SELECT
            COALESCE(SUM(CAST(json_extract(tags,'$.tokens_in') AS INTEGER)),0) AS tokens_in,
            COALESCE(SUM(CAST(json_extract(tags,'$.tokens_out') AS INTEGER)),0) AS tokens_out,
            COALESCE(SUM(CAST(json_extract(tags,'$.cost') AS REAL)),0.0) AS cost
         FROM ipc_org_telemetry WHERE org_id=?1 AND metric='usage'",
        rusqlite::params![org_id],
    )?
    .unwrap_or_else(|| json!({}));
    let task_stats = query_one(
        conn,
        "SELECT
            COUNT(t.id) AS total,
            SUM(CASE WHEN t.status IN ('submitted','done') THEN 1 ELSE 0 END) AS completed
         FROM tasks t
         JOIN plans p ON p.id = t.plan_id
         WHERE p.project_id = ?1",
        rusqlite::params![org_id],
    )?
    .unwrap_or_else(|| json!({"total": 0, "completed": 0}));
    Ok(json!({
        "org_id": org_id,
        "generated_at": chrono::Utc::now().to_rfc3339(),
        "decisions": decisions,
        "telemetry": telemetry,
        "task_stats": task_stats
    }))
}

fn compact_digest(content: &Value) -> String {
    let total = content["task_stats"]["total"].as_i64().unwrap_or(0);
    let completed = content["task_stats"]["completed"].as_i64().unwrap_or(0);
    let cost = content["telemetry"]["cost"].as_f64().unwrap_or(0.0);
    format!("{completed}/{total} task, costo ${cost:.2}")
}

async fn push_digest_telegram(text: &str) {
    let token = crate::telegram_config::telegram_token();
    let chat_id = crate::telegram_config::telegram_chat_id().ok().flatten();
    if let (Some(token), Some(chat_id)) = (token, chat_id) {
        let _ = crate::kernel::telegram::send_text(&token, chat_id, text, None).await;
    }
}
