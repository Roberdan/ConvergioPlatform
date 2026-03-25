// TDD tests for watchdog — written BEFORE implementation (RED phase).
// F-26: Local LLM kernel watchdog.

#[cfg(test)]
mod tests {
    use super::super::notify::ChannelConfig;
    use super::super::watchdog::{
        CheckResult, WatchdogConfig, WatchdogStatus,
    };

    #[test]
    fn watchdog_config_defaults() {
        let cfg = WatchdogConfig::default();
        assert_eq!(cfg.check_interval_secs, 30);
        // Ollama default URL
        assert!(cfg.ollama_url.contains("11434"));
    }

    #[test]
    fn watchdog_config_channels_empty_by_default() {
        let cfg = WatchdogConfig::default();
        assert!(cfg.notification_channels.is_empty());
    }

    #[test]
    fn watchdog_config_with_channels() {
        let cfg = WatchdogConfig {
            check_interval_secs: 60,
            ollama_url: "http://localhost:11434".to_string(),
            notification_channels: vec![ChannelConfig::Ntfy {
                topic: "test-topic".to_string(),
                base_url: "https://ntfy.sh".to_string(),
            }],
            stale_threshold_secs: 300,
            daemon_url: "http://localhost:8420".to_string(),
            model_name: "llama3".to_string(),
        };
        assert_eq!(cfg.check_interval_secs, 60);
        assert_eq!(cfg.notification_channels.len(), 1);
    }

    #[test]
    fn check_result_pass() {
        let r = CheckResult::pass("health_check");
        assert!(r.ok);
        assert_eq!(r.check_name, "health_check");
        assert!(r.details.is_none());
    }

    #[test]
    fn check_result_fail_has_details() {
        let r = CheckResult::fail("health_check", "timeout after 5s");
        assert!(!r.ok);
        assert_eq!(r.check_name, "health_check");
        assert!(r.details.is_some());
    }

    #[test]
    fn watchdog_status_serializes() {
        let status = WatchdogStatus {
            running: true,
            checks_passed: 4,
            checks_failed: 1,
            last_check_at: Some("2026-03-25T09:00:00Z".to_string()),
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"running\":true"));
        assert!(json.contains("\"checks_passed\":4"));
    }

    #[test]
    fn notification_channel_ntfy_topic() {
        let ch = ChannelConfig::Ntfy {
            topic: "convergio-alerts".to_string(),
            base_url: "https://ntfy.sh".to_string(),
        };
        match ch {
            ChannelConfig::Ntfy { topic, .. } => assert_eq!(topic, "convergio-alerts"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn notification_channel_macos_variant_exists() {
        let ch = ChannelConfig::MacOS;
        // Just ensure the enum variant is usable
        assert!(matches!(ch, ChannelConfig::MacOS));
    }
}
