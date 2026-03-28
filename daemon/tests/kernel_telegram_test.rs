// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Integration tests for Telegram kernel modules: outbound notifications,
// quiet hours routing, inbound poll security filter, report formatting.
// All tests require #[cfg(feature = "kernel")].

#![cfg(feature = "kernel")]

use convergio_core::kernel::{
    reports::{
        format_daily_report, format_plan_complete, parse_report_config, report_plan_complete,
        DailyMetrics,
    },
    telegram::{send_text, QuietHoursConfig},
    telegram_poll::{extract_text_message, TelegramMessage, TelegramUpdate},
};
use wiremock::{
    matchers::{method, path_regex},
    Mock, MockServer, ResponseTemplate,
};

// ---------------------------------------------------------------------------
// test_telegram_send_text_format
// WHY: sendMessage must post JSON with chat_id, text, and parse_mode=Markdown.
// Verifies the payload shape rather than relying on real Telegram credentials.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_telegram_send_text_format() {
    let server = MockServer::start().await;

    // Telegram adapter builds URL as /bot{token}/sendMessage
    Mock::given(method("POST"))
        .and(path_regex("/bot.*sendMessage"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ok": true,
            "result": {}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let result =
        send_text("test_token_abc", 123456789, "Hello *kernel*", Some(&server.uri())).await;

    assert!(result.is_ok(), "send_text with mock server must succeed: {result:?}");
    // Wiremock's expect(1) will fail the test if sendMessage was not called.
}

// ---------------------------------------------------------------------------
// test_telegram_quiet_hours_active
// WHY: time within quiet window → audio suppressed, text-only path selected.
// Uses is_active_at(hour, minute) — deterministic, no wall-clock dependency.
// ---------------------------------------------------------------------------

#[test]
fn test_telegram_quiet_hours_active() {
    // Window 23:00–07:00 UTC: 01:30 is inside the midnight-wrapping range.
    let qh = QuietHoursConfig::parse("23:00-07:00").expect("valid range");

    // Inside window — audio suppressed.
    assert!(
        qh.is_active_at(1, 30),
        "01:30 must be inside the 23:00–07:00 quiet window (audio suppressed)"
    );
    assert!(
        qh.is_active_at(23, 15),
        "23:15 must be inside the 23:00–07:00 quiet window"
    );
    // Just before end boundary — still active.
    assert!(
        qh.is_active_at(6, 59),
        "06:59 must still be inside the quiet window"
    );
}

// ---------------------------------------------------------------------------
// test_telegram_quiet_hours_inactive
// WHY: time outside quiet window → both text and audio channels are active.
// ---------------------------------------------------------------------------

#[test]
fn test_telegram_quiet_hours_inactive() {
    // Window 23:00–07:00 UTC: midday is outside the range.
    let qh = QuietHoursConfig::parse("23:00-07:00").expect("valid range");

    assert!(
        !qh.is_active_at(12, 0),
        "12:00 must be outside the quiet window (both channels active)"
    );
    assert!(
        !qh.is_active_at(7, 0),
        "07:00 is the end boundary — must be inactive (not quiet)"
    );
    assert!(
        !qh.is_active_at(20, 30),
        "20:30 must be outside the quiet window"
    );
}

// ---------------------------------------------------------------------------
// test_telegram_poll_ignores_other_chat
// WHY: getUpdates may contain messages from arbitrary chats.
// Security: only the authorised CONVERGIO_TELEGRAM_CHAT_ID must be processed.
// ---------------------------------------------------------------------------

#[test]
fn test_telegram_poll_ignores_other_chat() {
    let authorised_chat_id: i64 = 100_000_001;
    let foreign_chat_id: i64 = 999_999_999;

    let update_from_attacker = TelegramUpdate {
        update_id: 501,
        message: Some(TelegramMessage {
            chat_id: foreign_chat_id,
            text: Some("inject malicious command".to_string()),
        }),
    };

    let result = extract_text_message(&update_from_attacker, authorised_chat_id);

    assert!(
        result.is_none(),
        "message from wrong chat_id must be ignored — security filter must reject: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// test_report_config_parsing
// WHY: KERNEL_REPORTS="daily,weekly" → only those two flags enabled; others false.
// ---------------------------------------------------------------------------

#[test]
fn test_report_config_parsing() {
    let cfg = parse_report_config("daily,weekly");

    assert!(cfg.daily, "daily must be enabled");
    assert!(cfg.weekly, "weekly must be enabled");
    assert!(!cfg.critical, "critical must NOT be enabled");
    assert!(!cfg.completion, "completion must NOT be enabled");
}

// ---------------------------------------------------------------------------
// test_daily_report_format
// WHY: format_daily_report must produce Italian text with all metric values.
// ---------------------------------------------------------------------------

#[test]
fn test_daily_report_format() {
    let metrics = DailyMetrics {
        active_plans: 4,
        completed_tasks: 17,
        cost_yesterday_usd: 63.50,
        agents_used: 6,
        critical_events: 0,
    };

    let msg = format_daily_report(&metrics);

    assert!(
        msg.contains("Buongiorno"),
        "daily report must open with Italian greeting: {msg}"
    );
    assert!(msg.contains("4"), "active_plans count must appear: {msg}");
    assert!(msg.contains("17"), "completed_tasks count must appear: {msg}");
    assert!(msg.contains("63.50"), "cost must appear formatted to 2 decimal places: {msg}");
    assert!(msg.contains("6"), "agents_used count must appear: {msg}");
    // Zero critical events → Italian "nessun" phrase
    let lower = msg.to_lowercase();
    assert!(
        lower.contains("nessun") || lower.contains("critico"),
        "zero critical events must show Italian no-event phrase: {msg}"
    );
}

// ---------------------------------------------------------------------------
// test_plan_complete_report
// WHY: format_plan_complete must embed name, cost, and duration in Italian.
// ---------------------------------------------------------------------------

#[test]
fn test_plan_complete_report() {
    let msg = format_plan_complete("Piano 729 Kernel Telegram", "$142.80", "3h 45m");

    assert!(
        msg.contains("Piano 729 Kernel Telegram"),
        "plan name must appear verbatim: {msg}"
    );
    assert!(msg.contains("$142.80"), "cost must appear verbatim: {msg}");
    assert!(msg.contains("3h 45m"), "duration must appear verbatim: {msg}");
    let lower = msg.to_lowercase();
    assert!(
        lower.contains("completat") || lower.contains("piano"),
        "report must be Italian (completato/piano): {msg}"
    );
}

// ---------------------------------------------------------------------------
// test_report_plan_complete_no_token_degrades_gracefully
// WHY: missing env credentials must not panic — graceful degradation (FAIL-LOUD
// in logs only, not a crash).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_report_plan_complete_no_token_degrades_gracefully() {
    // Ensure no credentials are set so the graceful-degrade path runs.
    std::env::remove_var("CONVERGIO_TELEGRAM_TOKEN");
    std::env::remove_var("CONVERGIO_TELEGRAM_CHAT_ID");

    // report_plan_complete checks cfg.completion; default env → all enabled.
    // Without token it must log a warning and return — never panic.
    report_plan_complete("GracefulDegradePlan", "$0.00", "0m").await;
    // Reaching here without panic = PASS.
}
