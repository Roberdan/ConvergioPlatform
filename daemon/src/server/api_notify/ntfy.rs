// NtfyChannel — push notifications via ntfy.sh HTTP API.
// Delivers notifications to self-hosted or public ntfy.sh instances.

use serde::Deserialize;

/// Configuration for ntfy.sh push notifications.
#[derive(Debug, Clone, Deserialize)]
pub struct NtfyConfig {
    pub enabled: bool,
    pub url: String,
    pub topic: String,
}

impl Default for NtfyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            url: "https://ntfy.sh".to_string(),
            topic: "convergio".to_string(),
        }
    }
}

/// Map severity string to ntfy priority (1-5).
/// critical/error -> 5 (max), warning -> 3 (default), info/other -> 1 (min)
pub fn severity_to_priority(severity: &str) -> u8 {
    match severity {
        "critical" | "error" => 5,
        "warning" => 3,
        _ => 1,
    }
}

/// Send a notification via ntfy.sh HTTP POST.
/// Returns Ok(true) if delivered, Ok(false) if ntfy disabled, Err on failure.
pub async fn send(
    config: &NtfyConfig,
    title: &str,
    message: &str,
    severity: &str,
) -> Result<bool, reqwest::Error> {
    if !config.enabled {
        return Ok(false);
    }

    let priority = severity_to_priority(severity);
    let url = format!("{}/{}", config.url.trim_end_matches('/'), config.topic);

    reqwest::Client::new()
        .post(&url)
        .header("Title", title)
        .header("Priority", priority.to_string())
        .header("Tags", severity)
        .body(message.to_string())
        .send()
        .await?;

    Ok(true)
}

/// Load NtfyConfig from `claude-config/config/notifications.conf`.
pub fn load_config() -> NtfyConfig {
    let settings = crate::resilience::notify_config::NotificationSettings::load();
    NtfyConfig {
        enabled: settings.ntfy_enabled,
        url: settings.ntfy_server,
        topic: settings.ntfy_topic,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_critical_maps_to_priority_5() {
        assert_eq!(severity_to_priority("critical"), 5);
    }

    #[test]
    fn severity_error_maps_to_priority_5() {
        assert_eq!(severity_to_priority("error"), 5);
    }

    #[test]
    fn severity_warning_maps_to_priority_3() {
        assert_eq!(severity_to_priority("warning"), 3);
    }

    #[test]
    fn severity_info_maps_to_priority_1() {
        assert_eq!(severity_to_priority("info"), 1);
    }

    #[test]
    fn severity_unknown_maps_to_priority_1() {
        assert_eq!(severity_to_priority("unknown"), 1);
    }

    #[test]
    fn default_config_uses_ntfy_sh() {
        let cfg = NtfyConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.url, "https://ntfy.sh");
        assert_eq!(cfg.topic, "convergio");
    }

    #[test]
    fn config_deserializes_from_json() {
        let json = r#"{"enabled":false,"url":"https://ntfy.example.org","topic":"alerts"}"#;
        let cfg: NtfyConfig = serde_json::from_str(json).unwrap();
        assert!(!cfg.enabled);
        assert_eq!(cfg.url, "https://ntfy.example.org");
        assert_eq!(cfg.topic, "alerts");
    }

    #[tokio::test]
    async fn send_returns_false_when_disabled() {
        let cfg = NtfyConfig {
            enabled: false,
            url: "https://ntfy.example.org".to_string(),
            topic: "test".to_string(),
        };
        let result = send(&cfg, "title", "msg", "info").await.unwrap();
        assert!(!result, "disabled config should return false without HTTP call");
    }

    #[tokio::test]
    async fn send_builds_correct_url_and_headers() {
        // Async TCP listener avoids deadlock with single-threaded test runner
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let url = format!("http://127.0.0.1:{port}");

        let handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 4096];
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let n = stream.read(&mut buf).await.unwrap();
            let request = String::from_utf8_lossy(&buf[..n]).to_string();
            let _ = stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                .await;
            request
        });

        let cfg = NtfyConfig {
            enabled: true,
            url: url.clone(),
            topic: "convergio".to_string(),
        };

        let result = send(&cfg, "Ali needs help", "no peers available", "warning").await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        let request = handle.await.unwrap();
        assert!(
            request.contains("POST /convergio"),
            "should POST to /convergio: {request}"
        );
        assert!(
            request.contains("title: Ali needs help"),
            "should have Title header: {request}"
        );
        assert!(
            request.contains("priority: 3"),
            "warning should map to priority 3: {request}"
        );
    }
}
