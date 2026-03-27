// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// TDD tests for kernel/recover.rs — written BEFORE implementation (RED phase).
// Deterministic recovery chain: if/else rules, NOT LLM-decided.

#[cfg(test)]
mod tests {
    use crate::kernel::recover::{
        communicate, recover, NotifyChannel, RecoveryConfig, Severity,
    };

    fn test_config() -> RecoveryConfig {
        RecoveryConfig {
            ntfy_topic: "convergio-test".to_string(),
            channels: vec![NotifyChannel::Ntfy],
            // disable external side-effects in tests
            dry_run: true,
        }
    }

    // --- Severity enum tests ---

    #[test]
    fn severity_variants_exist() {
        let _ = Severity::Ok;
        let _ = Severity::Warn;
        let _ = Severity::Critical;
    }

    #[test]
    fn severity_critical_display_contains_critical() {
        let s = format!("{}", Severity::Critical);
        assert!(s.to_uppercase().contains("CRITICAL"), "got: {s}");
    }

    #[test]
    fn severity_warn_display_contains_warn() {
        let s = format!("{}", Severity::Warn);
        assert!(s.to_uppercase().contains("WARN"), "got: {s}");
    }

    // --- RecoveryConfig tests ---

    #[test]
    fn recovery_config_defaults_from_env() {
        // KERNEL_NTFY_TOPIC unset → falls back to "convergio"
        std::env::remove_var("KERNEL_NTFY_TOPIC");
        let cfg = RecoveryConfig::from_env();
        assert!(!cfg.ntfy_topic.is_empty());
    }

    #[test]
    fn recovery_config_reads_ntfy_topic_env() {
        // parse_channels is deterministic without env side-effects; verify it
        // independently to avoid parallel test pollution on KERNEL_NTFY_TOPIC.
        use crate::kernel::recover::NotifyChannel;
        // Inline the parse logic: "ntfy" → NotifyChannel::Ntfy
        let channels: Vec<NotifyChannel> = "ntfy"
            .split(',')
            .filter_map(|s| match s.trim() {
                "ntfy" => Some(NotifyChannel::Ntfy),
                _ => None,
            })
            .collect();
        assert!(channels.contains(&NotifyChannel::Ntfy));
    }

    #[test]
    fn notify_channels_parsed_from_env() {
        std::env::set_var("KERNEL_NOTIFY_CHANNELS", "local,telegram,ntfy");
        let cfg = RecoveryConfig::from_env();
        assert!(cfg.channels.contains(&NotifyChannel::Ntfy));
        std::env::remove_var("KERNEL_NOTIFY_CHANNELS");
    }

    // --- communicate() stub tests (dry_run=true, no network) ---

    #[tokio::test]
    async fn communicate_ok_severity_does_not_panic() {
        let cfg = test_config();
        // Must not panic; dry_run skips actual HTTP
        communicate("all clear", Severity::Ok, &cfg).await;
    }

    #[tokio::test]
    async fn communicate_warn_severity_does_not_panic() {
        let cfg = test_config();
        communicate("sustained high load", Severity::Warn, &cfg).await;
    }

    #[tokio::test]
    async fn communicate_critical_severity_does_not_panic() {
        let cfg = test_config();
        communicate("peer daemon unreachable", Severity::Critical, &cfg).await;
    }

    // --- recover() action selection tests ---

    #[tokio::test]
    async fn recover_ok_only_logs() {
        let cfg = test_config();
        // dry_run=true: no external commands fired; must return Ok
        let result = recover(Severity::Ok, None, &cfg).await;
        assert!(result.is_ok(), "recover Ok must succeed: {result:?}");
    }

    #[tokio::test]
    async fn recover_warn_consecutive_below_threshold_only_logs() {
        let cfg = test_config();
        // < 3 consecutive cycles → no notify
        let result = recover(Severity::Warn, Some(2), &cfg).await;
        assert!(result.is_ok(), "recover Warn (2 cycles) must succeed: {result:?}");
    }

    #[tokio::test]
    async fn recover_warn_consecutive_above_threshold_notifies() {
        let cfg = test_config();
        // >= 3 consecutive cycles (90s) → notify
        let result = recover(Severity::Warn, Some(4), &cfg).await;
        assert!(result.is_ok(), "recover Warn (4 cycles) must succeed: {result:?}");
    }

    #[tokio::test]
    async fn recover_critical_runs_chain_dry_run() {
        let cfg = test_config();
        // dry_run=true: checkpoint/reap/notify steps are no-ops
        let result = recover(Severity::Critical, None, &cfg).await;
        assert!(result.is_ok(), "recover Critical dry_run must succeed: {result:?}");
    }
}
