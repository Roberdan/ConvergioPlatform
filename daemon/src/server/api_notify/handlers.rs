use crate::server::state::{query_rows, ApiError, ServerState};
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/notify", post(handle_notify))
        .route("/api/notify/queue", get(handle_queue))
        .route("/api/notify/deliver", post(handle_deliver))
}

/// POST /api/notify — create notification, attempt native delivery + mesh relay
/// Body: {severity, title, message, plan_id?, link?}
    #[tracing::instrument(skip_all)]
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

    // DB insert must complete before any .await (rusqlite::Connection is !Send)
    let (notif_id, native_ok) = {
        let conn = state.get_conn()?;
        conn.execute(
            "INSERT INTO notification_queue (severity, title, message, plan_id, link, status) \
             VALUES (?1, ?2, ?3, ?4, ?5, 'pending')",
            rusqlite::params![severity, title, message, plan_id, link],
        )
        .map_err(|e| ApiError::internal(format!("notify insert failed: {e}")))?;

        let nid: i64 = conn
            .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
            .map_err(|e| ApiError::internal(format!("rowid failed: {e}")))?;

        let native = try_native_notify(title, message, severity);
        (nid, native)
    };

    // Attempt ntfy.sh push notification (requires .await)
    let ntfy_cfg = super::ntfy::load_config();
    let ntfy_result = super::ntfy::send(&ntfy_cfg, title, message, severity).await;
    let ntfy_ok = match &ntfy_result {
        Ok(delivered) => *delivered,
        Err(e) => {
            tracing::error!(channel = "ntfy", error = %e, "notification delivery failed");
            false
        }
    };

    let delivered = native_ok || ntfy_ok;

    // Build per-channel status report
    let mut channels_status = vec![];
    channels_status.push(json!({
        "channel": "native",
        "success": native_ok,
        "error": if native_ok { None } else { Some("terminal-notifier failed or unavailable") },
    }));
    channels_status.push(match &ntfy_result {
        Ok(true) => json!({"channel": "ntfy", "success": true, "error": null}),
        Ok(false) => json!({"channel": "ntfy", "success": false, "error": "ntfy disabled"}),
        Err(e) => json!({"channel": "ntfy", "success": false, "error": e.to_string()}),
    });

    // Mark as delivered if any channel succeeded
    if delivered {
        let conn = state.get_conn()?;
        conn.execute(
            "UPDATE notification_queue SET status = 'delivered', \
             delivered_at = datetime('now') WHERE id = ?1",
            rusqlite::params![notif_id],
        )
        .map_err(|e| ApiError::internal(format!("notification status update failed: {e}")))?;
    }

    // Broadcast via WebSocket for real-time dashboard updates
    if let Err(e) = state.ws_tx.send(json!({
        "type": "notification",
        "id": notif_id,
        "severity": severity,
        "title": title,
        "message": message,
    })) {
        tracing::debug!("ws notification broadcast: {e}");
    }

    let any_failed = channels_status.iter().any(|c| c["success"] == false);
    Ok(Json(json!({
        "ok": delivered,
        "id": notif_id,
        "status": if delivered { "delivered" } else { "pending" },
        "channels": channels_status,
        "partial_failure": delivered && any_failed,
    })))
}

/// GET /api/notify/queue — list pending notifications
    #[tracing::instrument(skip_all)]
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
    #[tracing::instrument(skip_all)]
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
    let mut errors: Vec<Value> = Vec::new();

    for id_val in ids {
        if let Some(id) = id_val.as_i64() {
            match conn.execute(
                "UPDATE notification_queue SET status = 'delivered', \
                 delivered_at = datetime('now') WHERE id = ?1 AND status = 'pending'",
                rusqlite::params![id],
            ) {
                Ok(changed) => delivered += changed,
                Err(e) => {
                    tracing::error!(notification_id = id, error = %e, "deliver update failed");
                    errors.push(json!({"id": id, "error": e.to_string()}));
                }
            }
        }
    }

    Ok(Json(json!({
        "ok": errors.is_empty(),
        "delivered": delivered,
        "errors": errors,
    })))
}

/// Try to send a native OS notification — daemon-native only (no osascript).
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
        match std::process::Command::new("terminal-notifier")
            .args(["-title", &full_title, "-message", message, "-group", "claude-core"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(_) => true,
            Err(e) => {
                tracing::error!(channel = "native", error = %e, "terminal-notifier failed");
                false
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        match std::process::Command::new("notify-send")
            .args([&full_title, message])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(_) => true,
            Err(e) => {
                tracing::error!(channel = "native", error = %e, "notify-send failed");
                false
            }
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        tracing::error!(channel = "native", "no native notification tool available");
        false
    }
}
