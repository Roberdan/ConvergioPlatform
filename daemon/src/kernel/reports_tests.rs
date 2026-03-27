// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// TDD tests for kernel/reports.rs — written BEFORE implementation (RED phase).
// Proactive Telegram reports: scheduled (daily/weekly) + event-driven (plan/wave).

#[cfg(test)]
mod tests {
    use crate::kernel::reports::{
        format_daily_report, format_plan_complete, format_wave_merge, format_weekly_report,
        parse_report_config, DailyMetrics, WaveInfo,
    };

    // ----- ReportConfig parsing -----------------------------------------------

    #[test]
    fn parse_config_all_enabled() {
        let cfg = parse_report_config("daily,critical,completion,weekly");
        assert!(cfg.daily);
        assert!(cfg.critical);
        assert!(cfg.completion);
        assert!(cfg.weekly);
    }

    #[test]
    fn parse_config_partial() {
        let cfg = parse_report_config("daily,weekly");
        assert!(cfg.daily);
        assert!(!cfg.critical);
        assert!(!cfg.completion);
        assert!(cfg.weekly);
    }

    #[test]
    fn parse_config_empty_enables_all() {
        // Empty string or "all" → all enabled by default
        let cfg = parse_report_config("");
        assert!(cfg.daily);
        assert!(cfg.critical);
        assert!(cfg.completion);
        assert!(cfg.weekly);
    }

    #[test]
    fn parse_config_single_daily() {
        let cfg = parse_report_config("daily");
        assert!(cfg.daily);
        assert!(!cfg.weekly);
        assert!(!cfg.completion);
    }

    // ----- format_daily_report ------------------------------------------------

    #[test]
    fn daily_report_contains_buongiorno() {
        let metrics = DailyMetrics {
            active_plans: 3,
            completed_tasks: 12,
            cost_yesterday_usd: 85.0,
            agents_used: 4,
            critical_events: 0,
        };
        let msg = format_daily_report(&metrics);
        assert!(msg.contains("Buongiorno"), "expected Italian greeting: {msg}");
    }

    #[test]
    fn daily_report_contains_plan_count() {
        let metrics = DailyMetrics {
            active_plans: 5,
            completed_tasks: 8,
            cost_yesterday_usd: 42.50,
            agents_used: 2,
            critical_events: 0,
        };
        let msg = format_daily_report(&metrics);
        assert!(msg.contains("5"), "expected active plan count in report: {msg}");
    }

    #[test]
    fn daily_report_no_critical_events_shows_none() {
        let metrics = DailyMetrics {
            active_plans: 1,
            completed_tasks: 3,
            cost_yesterday_usd: 10.0,
            agents_used: 1,
            critical_events: 0,
        };
        let msg = format_daily_report(&metrics);
        // Should mention no critical events in Italian
        assert!(
            msg.to_lowercase().contains("nessun") || msg.to_lowercase().contains("critico"),
            "expected no-critical-event note: {msg}"
        );
    }

    #[test]
    fn daily_report_with_critical_events_shows_count() {
        let metrics = DailyMetrics {
            active_plans: 2,
            completed_tasks: 5,
            cost_yesterday_usd: 30.0,
            agents_used: 3,
            critical_events: 2,
        };
        let msg = format_daily_report(&metrics);
        assert!(msg.contains("2"), "expected critical event count: {msg}");
    }

    // ----- format_plan_complete -----------------------------------------------

    #[test]
    fn plan_complete_contains_name() {
        let msg = format_plan_complete("Piano 729 Kernel Reports", "$120.50", "4h 30m");
        assert!(msg.contains("Piano 729"), "expected plan name: {msg}");
    }

    #[test]
    fn plan_complete_contains_cost() {
        let msg = format_plan_complete("MyPlan", "$85.00", "2h");
        assert!(msg.contains("$85.00"), "expected cost: {msg}");
    }

    #[test]
    fn plan_complete_contains_duration() {
        let msg = format_plan_complete("MyPlan", "$10.00", "1h 15m");
        assert!(msg.contains("1h 15m"), "expected duration: {msg}");
    }

    #[test]
    fn plan_complete_is_italian() {
        let msg = format_plan_complete("Piano 730", "$50.00", "3h");
        // Should contain Italian words for "completed" / "cost" / "duration"
        let lower = msg.to_lowercase();
        assert!(
            lower.contains("completat") || lower.contains("piano"),
            "expected Italian text: {msg}"
        );
    }

    // ----- format_wave_merge --------------------------------------------------

    #[test]
    fn wave_merge_contains_wave_id() {
        let info = WaveInfo {
            wave_id: "W3".to_owned(),
            plan_name: "Piano 729".to_owned(),
            next_wave: Some("W4".to_owned()),
        };
        let msg = format_wave_merge(&info);
        assert!(msg.contains("W3"), "expected wave id: {msg}");
    }

    #[test]
    fn wave_merge_contains_plan_name() {
        let info = WaveInfo {
            wave_id: "W1".to_owned(),
            plan_name: "Kernel Refactor".to_owned(),
            next_wave: None,
        };
        let msg = format_wave_merge(&info);
        assert!(msg.contains("Kernel Refactor"), "expected plan name: {msg}");
    }

    #[test]
    fn wave_merge_with_next_wave_shows_next() {
        let info = WaveInfo {
            wave_id: "W2".to_owned(),
            plan_name: "MyPlan".to_owned(),
            next_wave: Some("W3".to_owned()),
        };
        let msg = format_wave_merge(&info);
        assert!(msg.contains("W3"), "expected next wave: {msg}");
    }

    #[test]
    fn wave_merge_without_next_wave_shows_finale() {
        let info = WaveInfo {
            wave_id: "W5".to_owned(),
            plan_name: "FinalPlan".to_owned(),
            next_wave: None,
        };
        let msg = format_wave_merge(&info);
        // Should indicate no next wave (e.g. "ultima" or "completato")
        let lower = msg.to_lowercase();
        assert!(
            lower.contains("ultima") || lower.contains("completat") || lower.contains("fine"),
            "expected finale indication: {msg}"
        );
    }

    // ----- format_weekly_report -----------------------------------------------

    #[test]
    fn weekly_report_contains_riepilogo() {
        let msg = format_weekly_report(5, "$420.00", 3, "Ottimizzazione mesh completata");
        let lower = msg.to_lowercase();
        assert!(
            lower.contains("settiman") || lower.contains("riepilog"),
            "expected weekly context: {msg}"
        );
    }

    #[test]
    fn weekly_report_contains_cost() {
        let msg = format_weekly_report(10, "$999.99", 7, "Nessun apprendimento");
        assert!(msg.contains("$999.99"), "expected cost: {msg}");
    }

    #[test]
    fn weekly_report_contains_completed_plans() {
        let msg = format_weekly_report(4, "$200.00", 4, "Test learnings");
        assert!(msg.contains("4"), "expected completed plans count: {msg}");
    }

    // ----- spawn_report_loop (integration-style, dry-run via missing env) ----

    #[tokio::test]
    async fn report_plan_complete_no_token_logs_warn() {
        // Without CONVERGIO_TELEGRAM_TOKEN set, should not panic — logs warning
        std::env::remove_var("CONVERGIO_TELEGRAM_TOKEN");
        std::env::remove_var("CONVERGIO_TELEGRAM_CHAT_ID");
        // Must not panic — event-driven report degrades gracefully
        crate::kernel::reports::report_plan_complete("TestPlan", "$0.00", "0m").await;
    }

    #[tokio::test]
    async fn report_wave_merge_no_token_logs_warn() {
        std::env::remove_var("CONVERGIO_TELEGRAM_TOKEN");
        std::env::remove_var("CONVERGIO_TELEGRAM_CHAT_ID");
        let info = WaveInfo {
            wave_id: "W1".to_owned(),
            plan_name: "TestPlan".to_owned(),
            next_wave: None,
        };
        crate::kernel::reports::report_wave_merge(&info).await;
    }
}
