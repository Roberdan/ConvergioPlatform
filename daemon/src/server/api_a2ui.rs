// A2UI — Agent-to-UI block push protocol (handlers).
// Agents POST blocks; frontend receives via SSE stream (see api_a2ui_sse.rs).

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use rusqlite::params;
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::info;

use super::state::ServerState;

#[derive(Debug, Deserialize)]
pub struct BlockPushRequest {
    pub agent_id: String,
    pub target: Option<BlockTarget>,
    pub block: Value,
    pub priority: Option<String>,
    pub ttl_seconds: Option<i64>,
    pub replaces: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BlockTarget {
    pub page: Option<String>,
    pub position: Option<String>,
}

pub fn router(state: ServerState) -> Router<ServerState> {
    let cleanup_state = state.clone();
    tokio::spawn(super::api_a2ui_sse::ttl_cleanup_loop(cleanup_state));
    Router::new()
        .route("/api/a2ui/blocks", get(list_blocks))
        .route("/api/a2ui/blocks/{id}", get(get_block))
        .route("/api/a2ui/push", post(push_block))
        .route("/api/a2ui/dismiss/{id}", post(dismiss_block))
        .route("/api/a2ui/stream", get(super::api_a2ui_sse::a2ui_stream))
}

async fn push_block(
    State(state): State<ServerState>,
    Json(req): Json<BlockPushRequest>,
) -> impl IntoResponse {
    let block_id = uuid::Uuid::new_v4().to_string();
    let block_type = req.block.get("type").and_then(|v| v.as_str())
        .unwrap_or("unknown").to_string();
    let block_json = serde_json::to_string(&req.block).unwrap_or_default();
    let target = req.target.unwrap_or(BlockTarget {
        page: None, position: Some("top".into()),
    });
    let page = target.page.clone();
    let position = target.position.unwrap_or_else(|| "top".into());
    let priority = req.priority.unwrap_or_else(|| "normal".into());

    if let Some(ref old_id) = req.replaces {
        if let Ok(conn) = state.get_conn() {
            let _ = conn.execute(
                "UPDATE a2ui_blocks SET status = 'replaced' WHERE id = ?1",
                params![old_id],
            );
        }
    }

    let insert_ok = match state.get_conn() {
        Ok(conn) => conn.execute(
            "INSERT INTO a2ui_blocks (id, agent_id, target_page, target_position, \
             block_type, block_json, priority, ttl_seconds) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![block_id, req.agent_id, page, position,
                    block_type, block_json, priority, req.ttl_seconds],
        ).is_ok(),
        Err(_) => false,
    };

    if !insert_ok {
        return Json(json!({"ok": false, "error": "db insert failed"}));
    }

    let _ = state.ws_tx.send(json!({
        "type": "a2ui_block",
        "block_id": block_id,
        "agent_id": req.agent_id,
        "target": { "page": page, "position": position },
        "block": req.block,
        "priority": priority,
        "ttl_seconds": req.ttl_seconds,
        "replaces": req.replaces,
    }));

    info!(block_id = %block_id, agent = %req.agent_id, "A2UI block pushed");
    Json(json!({"ok": true, "block_id": block_id}))
}

async fn list_blocks(State(state): State<ServerState>) -> impl IntoResponse {
    let conn = match state.get_conn() {
        Ok(c) => c,
        Err(_) => return Json(json!({"ok": false, "blocks": []})),
    };
    let mut stmt = match conn.prepare(
        "SELECT id, agent_id, target_page, target_position, block_type, \
         block_json, priority, ttl_seconds, status, created_at \
         FROM a2ui_blocks WHERE status = 'active' ORDER BY created_at DESC",
    ) {
        Ok(s) => s,
        Err(_) => return Json(json!({"ok": true, "blocks": []})),
    };
    let blocks: Vec<Value> = stmt
        .query_map([], |row| {
            let bj: String = row.get(5)?;
            let block: Value = serde_json::from_str(&bj).unwrap_or(json!({}));
            Ok(json!({
                "block_id": row.get::<_, String>(0)?,
                "agent_id": row.get::<_, String>(1)?,
                "target": {
                    "page": row.get::<_, Option<String>>(2)?,
                    "position": row.get::<_, String>(3)?
                },
                "block": block,
                "priority": row.get::<_, String>(6)?,
                "ttl_seconds": row.get::<_, Option<i64>>(7)?,
                "status": row.get::<_, String>(8)?,
                "created_at": row.get::<_, String>(9)?,
            }))
        })
        .ok()
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default();

    Json(json!({"ok": true, "blocks": blocks}))
}

async fn get_block(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let block = state.get_conn().ok().and_then(|conn| {
        conn.query_row(
            "SELECT id, agent_id, block_json, status FROM a2ui_blocks WHERE id = ?1",
            params![id],
            |row| {
                let bj: String = row.get(2)?;
                Ok(json!({
                    "block_id": row.get::<_, String>(0)?,
                    "agent_id": row.get::<_, String>(1)?,
                    "block": serde_json::from_str::<Value>(&bj).unwrap_or(json!({})),
                    "status": row.get::<_, String>(3)?,
                }))
            },
        ).ok()
    });
    match block {
        Some(b) => Json(json!({"ok": true, "block": b})),
        None => Json(json!({"ok": false, "error": "not found"})),
    }
}

async fn dismiss_block(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let updated = state.get_conn().ok()
        .map(|conn| conn.execute(
            "UPDATE a2ui_blocks SET status = 'dismissed', \
             dismissed_at = datetime('now') WHERE id = ?1",
            params![id],
        ).unwrap_or(0))
        .unwrap_or(0);

    if updated > 0 {
        let _ = state.ws_tx.send(json!({"type": "a2ui_dismiss", "block_id": id}));
    }
    Json(json!({"ok": updated > 0}))
}
