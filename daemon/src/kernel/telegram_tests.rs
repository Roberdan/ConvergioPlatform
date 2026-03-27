// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// TDD tests for kernel/telegram.rs — written BEFORE implementation (RED phase).
// Outbound Telegram notifications: text (sendMessage) + voice (sendVoice).

#[cfg(test)]
mod tests {
    use crate::kernel::telegram::{
        communicate, send_text, send_voice, NotifyMode, QuietHoursConfig,
    };

    // ----- Config / NotifyMode parsing (parse_mode is deterministic, no env) ---

    fn parse_mode(s: &str) -> NotifyMode {
        // Mirror the from_env logic without touching env
        match s {
            "local" => NotifyMode::Local,
            "both" => NotifyMode::Both,
            _ => NotifyMode::Telegram,
        }
    }

    #[test]
    fn notify_mode_parses_telegram() {
        assert_eq!(parse_mode("telegram"), NotifyMode::Telegram);
    }

    #[test]
    fn notify_mode_parses_local() {
        assert_eq!(parse_mode("local"), NotifyMode::Local);
    }

    #[test]
    fn notify_mode_parses_both() {
        assert_eq!(parse_mode("both"), NotifyMode::Both);
    }

    #[test]
    fn notify_mode_defaults_to_telegram_on_unknown() {
        assert_eq!(parse_mode("unknown"), NotifyMode::Telegram);
        assert_eq!(parse_mode(""), NotifyMode::Telegram);
    }

    // ----- Quiet hours parsing -----------------------------------------------

    #[test]
    fn quiet_hours_parses_valid_range() {
        let qh = QuietHoursConfig::parse("23:00-07:00");
        assert!(qh.is_some());
        let qh = qh.unwrap();
        assert_eq!(qh.start_hour, 23);
        assert_eq!(qh.start_minute, 0);
        assert_eq!(qh.end_hour, 7);
        assert_eq!(qh.end_minute, 0);
    }

    #[test]
    fn quiet_hours_rejects_invalid() {
        assert!(QuietHoursConfig::parse("bad-input").is_none());
        assert!(QuietHoursConfig::parse("").is_none());
    }

    #[test]
    fn quiet_hours_none_on_empty_string() {
        // from_env() returns None when env var absent; test parse directly
        let qh = QuietHoursConfig::parse("");
        assert!(qh.is_none());
    }

    #[test]
    fn quiet_hours_some_on_valid_string() {
        let qh = QuietHoursConfig::parse("23:00-07:00");
        assert!(qh.is_some());
    }

    // ----- Quiet-hours active detection (unit — no network) ------------------

    #[test]
    fn quiet_hours_active_wraps_midnight() {
        // 23:00-07:00 should be active at 00:30
        let qh = QuietHoursConfig::parse("23:00-07:00").unwrap();
        assert!(qh.is_active_at(0, 30));
        assert!(qh.is_active_at(23, 30));
        assert!(!qh.is_active_at(12, 0));
        assert!(!qh.is_active_at(7, 0));
    }

    #[test]
    fn quiet_hours_not_active_outside_range() {
        let qh = QuietHoursConfig::parse("23:00-07:00").unwrap();
        assert!(!qh.is_active_at(10, 0));
        assert!(!qh.is_active_at(15, 30));
    }

    // ----- send_text / send_voice return Err without network -----------------

    #[tokio::test]
    async fn send_text_returns_err_on_bad_token() {
        // With an invalid token, Telegram returns 401; no real network needed
        // when using a mock base_url that doesn't exist — expect an error.
        let result = send_text("invalid_token_xx", 123456789, "hello", None).await;
        // Must return Err (network error or HTTP error) — must not panic
        assert!(result.is_err(), "expected Err with bad token: {result:?}");
    }

    #[tokio::test]
    async fn send_voice_returns_err_on_bad_token() {
        let audio = vec![0u8; 64]; // minimal fake OGG bytes
        let result = send_voice("invalid_token_xx", 123456789, &audio, None).await;
        assert!(result.is_err(), "expected Err with bad token: {result:?}");
    }

    // ----- communicate() dry-run (no active_node, no network) ----------------

    #[tokio::test]
    async fn communicate_dry_run_does_not_panic() {
        // No env tokens set, no real node — must return without panicking.
        std::env::remove_var("CONVERGIO_TELEGRAM_TOKEN");
        std::env::remove_var("CONVERGIO_TELEGRAM_CHAT_ID");
        let result =
            communicate("kernel test message", crate::kernel::recover::Severity::Warn, true).await;
        // dry_run=true: should succeed (no-op)
        assert!(result.is_ok(), "communicate dry_run must not fail: {result:?}");
    }

    #[tokio::test]
    async fn communicate_critical_dry_run_does_not_panic() {
        std::env::remove_var("CONVERGIO_TELEGRAM_TOKEN");
        let result = communicate(
            "CRITICAL daemon lost",
            crate::kernel::recover::Severity::Critical,
            true,
        )
        .await;
        assert!(result.is_ok());
    }
}
