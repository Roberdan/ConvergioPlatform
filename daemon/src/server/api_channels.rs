// Channel API endpoints: list channels, send message, health check.
// Config-driven: ntfy always present, telegram if Telegram credentials are configured.

use super::state::{ApiError, ServerState};
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/channels", get(list_channels))
        .route("/api/channels/:name/send", post(send_message))
        .route("/api/channels/:name/health", get(channel_health))
}

/// Known channel names based on config and environment.
fn available_channels() -> Vec<ChannelInfo> {
    crate::resilience::notify_config::NotificationSettings::load()
        .enabled_channel_names()
        .into_iter()
        .filter(|name| *name != "dashboard")
        .map(|name| ChannelInfo {
            name: name.to_string(),
            connected: false,
            last_message_at: None,
            error_count: 0,
        })
        .collect()
}

fn is_known_channel(name: &str) -> bool {
    available_channels().iter().any(|c| c.name == name)
}

#[derive(Debug, Clone, serde::Serialize)]
struct ChannelInfo {
    name: String,
    connected: bool,
    last_message_at: Option<String>,
    error_count: u64,
}

/// GET /api/channels — list all registered channels with health status.
async fn list_channels(
    State(_state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    let channels = available_channels();
    let channel_json: Vec<Value> = channels
        .iter()
        .map(|c| {
            json!({
                "name": c.name,
                "connected": c.connected,
                "last_message_at": c.last_message_at,
                "error_count": c.error_count,
            })
        })
        .collect();

    Ok(Json(json!({
        "ok": true,
        "channels": channel_json,
    })))
}

/// POST /api/channels/:name/send — send a manual message through a channel.
/// Body: {"message": "...", "severity": "info|warning|error|critical"}
async fn send_message(
    State(_state): State<ServerState>,
    Path(name): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    if !is_known_channel(&name) {
        return Err(ApiError::not_found(format!("channel '{name}' not found")));
    }

    let message = body
        .get("message")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing 'message' field"))?;
    let severity = body
        .get("severity")
        .and_then(Value::as_str)
        .unwrap_or("info");

    // Dispatch to the appropriate channel adapter
    let delivered = match name.as_str() {
        "ntfy" => {
            let cfg = super::api_notify::ntfy::load_config();
            super::api_notify::ntfy::send(&cfg, "Convergio", message, severity)
                .await
                .unwrap_or_else(|e| {
                    tracing::warn!("ntfy send failed: {e}");
                    false
                })
        }
        "telegram" | "macos" => match crate::resilience::notify_config::NotificationSettings::load()
            .channel_config(name.as_str())
        {
            Ok(Some(channel)) => {
                let severity = severity.parse().unwrap_or(crate::resilience::notify::NotifySeverity::Info);
                let results = crate::resilience::notify::dispatch(
                    &[channel],
                    &crate::resilience::notify::NotifyMessage {
                        title: "Convergio".to_string(),
                        message: message.to_string(),
                        severity,
                    },
                )
                .await;
                results.first().map(|result| result.success).unwrap_or(false)
            }
            Ok(None) => false,
            Err(error) => {
                tracing::warn!(channel = %name, %error, "channel misconfigured");
                false
            }
        },
        _ => false,
    };

    Ok(Json(json!({
        "ok": true,
        "channel": name,
        "delivered": delivered,
    })))
}

/// GET /api/channels/:name/health — single channel health detail.
async fn channel_health(
    State(_state): State<ServerState>,
    Path(name): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let channels = available_channels();
    let channel = channels
        .iter()
        .find(|c| c.name == name)
        .ok_or_else(|| ApiError::not_found(format!("channel '{name}' not found")))?;

    Ok(Json(json!({
        "ok": true,
        "channel_name": channel.name,
        "connected": channel.connected,
        "last_message_at": channel.last_message_at,
        "error_count": channel.error_count,
    })))
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn available_channels_includes_ntfy() {
        let channels = available_channels();
        assert!(channels.iter().any(|c| c.name == "ntfy"));
    }

    #[test]
    fn ntfy_is_known_channel() {
        assert!(is_known_channel("ntfy"));
    }

    #[test]
    fn unknown_channel_not_known() {
        assert!(!is_known_channel("nonexistent"));
    }

    #[test]
    fn router_builds_without_panic() {
        let _ = router();
    }

    #[test]
    fn channel_info_serializes_correctly() {
        let info = ChannelInfo {
            name: "ntfy".to_string(),
            connected: true,
            last_message_at: None,
            error_count: 0,
        };
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["name"], "ntfy");
        assert_eq!(json["connected"], true);
        assert_eq!(json["error_count"], 0);
    }
}
