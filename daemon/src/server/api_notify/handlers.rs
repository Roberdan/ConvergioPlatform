use crate::server::state::{query_rows, ApiError, ServerState};
use axum::extract::State;
use axum::{Json, Router};
use axum::routing::{get, post};
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/notify", post(handle_notify))
        .route("/api/notify/queue", get(handle_queue))
        .route("/api/notify/deliver", post(handle_deliver))
}

/// POST /api/notify — create notification, attempt native delivery + mesh relay
/// Body: {severity, title, message, plan_id?, link?}
pub async fn handle_notify(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let title = body
        .get("title")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing title"))?;
    let message = body.get("message").and_then(Value::as_str).unwrap_or("");
    let severity = body
        .get("severity")
        .and_then(Value::as_str)
        .unwrap_or("info");
    let plan_id = body.get("plan_id").and_then(Value::as_i64);
    let link = body.get("link").and_then(Value::as_str);

    let conn = state.get_conn()?;
    let conn = &conn;

    // Insert into notification_queue
    conn.execute(
        "INSERT INTO notification_queue (severity, title, message, plan_id, link, status) \
         VALUES (?1, ?2, ?3, ?4, ?5, 'pending')",
        rusqlite::params![severity, title, message, plan_id, link],
    )
    .map_err(|e| ApiError::internal(format!("notify insert failed: {e}")))?;

    let notif_id: i64 = conn
        .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
        .map_err(|e| ApiError::internal(format!("rowid failed: {e}")))?;

    // Attempt native notification (non-blocking)
    let native_ok = try_native_notify(title, message, severity);

    // Mark as delivered if native succeeded
    if native_ok {
        let _ = conn.execute(
            "UPDATE notification_queue SET status = 'delivered', \
             delivered_at = datetime('now') WHERE id = ?1",
            rusqlite::params![notif_id],
        );
    }

    // Broadcast via WebSocket for real-time dashboard updates
    let _ = state.ws_tx.send(json!({
        "type": "notification",
        "id": notif_id,
        "severity": severity,
        "title": title,
        "message": message,
    }));

    Ok(Json(json!({
        "ok": true,
        "id": notif_id,
        "native_delivered": native_ok,
        "status": if native_ok { "delivered" } else { "pending" },
    })))
}

/// GET /api/notify/queue — list pending notifications
pub async fn handle_queue(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let notifications = query_rows(
        &conn,
        "SELECT id, severity, title, message, plan_id, link, status, created_at \
         FROM notification_queue \
         WHERE status = 'pending' \
         ORDER BY created_at DESC LIMIT 50",
        [],
    )?;

    Ok(Json(json!({
        "ok": true,
        "notifications": notifications,
        "count": notifications.len(),
    })))
}

/// POST /api/notify/deliver — mark notifications as delivered
/// Body: {ids: [1, 2, 3]}
pub async fn handle_deliver(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let ids = body
        .get("ids")
        .and_then(Value::as_array)
        .ok_or_else(|| ApiError::bad_request("missing ids array"))?;

    let conn = state.get_conn()?;
    let conn = &conn;
    let mut delivered = 0usize;

    for id_val in ids {
        if let Some(id) = id_val.as_i64() {
            let changed = conn
                .execute(
                    "UPDATE notification_queue SET status = 'delivered', \
                     delivered_at = datetime('now') WHERE id = ?1 AND status = 'pending'",
                    rusqlite::params![id],
                )
                .unwrap_or(0);
            delivered += changed;
        }
    }

    Ok(Json(json!({
        "ok": true,
        "delivered": delivered,
    })))
}

/// Try to send a native OS notification (non-blocking, best-effort)
pub fn try_native_notify(title: &str, message: &str, severity: &str) -> bool {
    let icon = match severity {
        "error" => "❌",
        "warning" => "⚠️",
        "success" => "✅",
        _ => "ℹ️",
    };
    let full_title = format!("{icon} {title}");

    #[cfg(target_os = "macos")]
    {
        let result = std::process::Command::new("terminal-notifier")
            .args([
                "-title",
                &full_title,
                "-message",
                message,
                "-group",
                "claude-core",
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
        if result.is_ok() {
            return true;
        }
        // Fallback to osascript
        let result = std::process::Command::new("osascript")
            .args([
                "-e",
                &format!(
                    "display notification \"{}\" with title \"{}\"",
                    message.replace('"', "\\\""),
                    full_title.replace('"', "\\\"")
                ),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
        return result.is_ok();
    }

    #[cfg(target_os = "linux")]
    {
        let result = std::process::Command::new("notify-send")
            .args([&full_title, message])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
        return result.is_ok();
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        false
    }
}
