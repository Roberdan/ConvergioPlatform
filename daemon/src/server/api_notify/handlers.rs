use super::metrics;
use crate::server::state::{query_rows, ApiError, ServerState};
use crate::server::telemetry;
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use crate::resilience::notify::{dispatch, ChannelResult, NotifyMessage, NotifySeverity};
use crate::resilience::notify_config::NotificationSettings;

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
    let trace_id = telemetry::current_request_id()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    // DB insert must complete before any .await (rusqlite::Connection is !Send)
    let notif_id = {
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
        nid
    };

    let settings = NotificationSettings::load();
    let notify_message = NotifyMessage {
        title: title.to_string(),
        message: message.to_string(),
        severity: parse_notify_severity(severity),
    };
    let mut configured_channels = Vec::new();
    let mut channel_results: Vec<ChannelResult> = Vec::new();
    for channel_name in ["macos", "ntfy", "telegram"] {
        match settings.channel_config(channel_name) {
            Ok(Some(channel)) => configured_channels.push(channel),
            Ok(None) => {}
            Err(error) => channel_results.push(ChannelResult {
                channel: channel_name.to_string(),
                success: false,
                error: Some(error),
                duration_ms: 0,
            }),
        }
    }
    channel_results.extend(dispatch(&configured_channels, &notify_message).await);

    // Mark as delivered if any channel succeeded.
    let delivered = channel_results.iter().any(|result| result.success);
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
    let dashboard_ok = state.ws_tx.send(json!({
        "type": "notification",
        "id": notif_id,
        "severity": severity,
        "title": title,
        "message": message,
    })).is_ok();
    if settings.dashboard_enabled {
        channel_results.push(ChannelResult {
            channel: "dashboard".to_string(),
            success: dashboard_ok,
            error: if dashboard_ok {
                None
            } else {
                Some("websocket broadcast failed".to_string())
            },
            duration_ms: 0,
        });
    }
    {
        let conn = state.get_conn()?;
        metrics::record_delivery_attempts(&conn, notif_id, &trace_id, &channel_results)?;
    }

    let channels_status = channel_results
        .iter()
        .map(|result| json!({
            "channel": result.channel,
            "success": result.success,
            "error": result.error,
            "duration_ms": result.duration_ms,
        }))
        .collect::<Vec<_>>();
    let any_failed = channel_results.iter().any(|result| !result.success);
    Ok(Json(json!({
        "ok": delivered,
        "id": notif_id,
        "trace_id": trace_id,
        "status": if delivered { "delivered" } else { "pending" },
        "channels": channels_status,
        "partial_failure": delivered && any_failed,
    })))
}

fn parse_notify_severity(raw: &str) -> NotifySeverity {
    match raw {
        "critical" | "error" => NotifySeverity::Critical,
        "warning" => NotifySeverity::Warning,
        _ => NotifySeverity::Info,
    }
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
