// Tests for inference metrics collection (F-05).
// Pattern: AAA (Arrange / Act / Assert), one concern per test.
use super::{
    InferenceMetricsCollector, InferenceMetricsEntry, ModelMetrics, TimeWindow,
};
use chrono::{Duration, Utc};

// --- helpers ---

fn entry(model: &str, latency_ms: u64, tokens: u32, cost: f64, success: bool) -> InferenceMetricsEntry {
    InferenceMetricsEntry {
        model: model.to_string(),
        latency_ms,
        tokens_used: tokens,
        cost,
        success,
        timestamp: Utc::now(),
    }
}

fn entry_at(
    model: &str,
    latency_ms: u64,
    tokens: u32,
    cost: f64,
    success: bool,
    offset: Duration,
) -> InferenceMetricsEntry {
    InferenceMetricsEntry {
        model: model.to_string(),
        latency_ms,
        tokens_used: tokens,
        cost,
        success,
        // Negative offset places the entry in the past.
        timestamp: Utc::now() + offset,
    }
}

// --- record / retrieve ---

#[test]
fn record_single_entry_and_retrieve_metrics() {
    let mut collector = InferenceMetricsCollector::new();
    collector.record(entry("claude-3", 200, 500, 0.01, true));

    let m = collector.metrics_for("claude-3", TimeWindow::OneHour);
    assert_eq!(m.model, "claude-3");
    assert_eq!(m.request_count, 1);
}

#[test]
fn all_metrics_returns_one_entry_per_model() {
    let mut collector = InferenceMetricsCollector::new();
    collector.record(entry("model-a", 100, 100, 0.01, true));
    collector.record(entry("model-b", 200, 200, 0.02, true));
    collector.record(entry("model-a", 150, 150, 0.01, true));

    let all = collector.all_metrics(TimeWindow::OneHour);
    assert_eq!(all.len(), 2);
    let names: Vec<&str> = all.iter().map(|m| m.model.as_str()).collect();
    assert!(names.contains(&"model-a"));
    assert!(names.contains(&"model-b"));
}

// --- error_rate ---

#[test]
fn error_rate_zero_when_all_succeed() {
    let mut collector = InferenceMetricsCollector::new();
    for _ in 0..5 {
        collector.record(entry("model-x", 100, 50, 0.01, true));
    }
    let m = collector.metrics_for("model-x", TimeWindow::OneHour);
    assert!((m.error_rate - 0.0).abs() < f64::EPSILON);
}

#[test]
fn error_rate_computed_correctly() {
    let mut collector = InferenceMetricsCollector::new();
    collector.record(entry("model-x", 100, 50, 0.01, true));
    collector.record(entry("model-x", 100, 50, 0.01, false));
    collector.record(entry("model-x", 100, 50, 0.01, false));

    let m = collector.metrics_for("model-x", TimeWindow::OneHour);
    // 2 failures out of 3 requests
    assert!((m.error_rate - (2.0 / 3.0)).abs() < 1e-9);
}

// --- latency percentiles ---

#[test]
fn latency_p50_p95_p99_ordered() {
    let mut collector = InferenceMetricsCollector::new();
    for i in 1u64..=100 {
        collector.record(entry("model-y", i * 10, 100, 0.001, true));
    }
    let m = collector.metrics_for("model-y", TimeWindow::OneHour);
    assert!(m.latency_p50 <= m.latency_p95, "p50 must be <= p95");
    assert!(m.latency_p95 <= m.latency_p99, "p95 must be <= p99");
}

#[test]
fn latency_p50_near_median() {
    let mut collector = InferenceMetricsCollector::new();
    // 10 entries: 100..1000 in steps of 100
    for i in 1u64..=10 {
        collector.record(entry("model-z", i * 100, 50, 0.0, true));
    }
    let m = collector.metrics_for("model-z", TimeWindow::OneHour);
    // median of [100,200,...,1000] is 500 or 600 depending on interpolation; just verify it's in range
    assert!(m.latency_p50 >= 400 && m.latency_p50 <= 700, "p50 out of expected range: {}", m.latency_p50);
}

// --- avg_tokens_per_sec ---

#[test]
fn avg_tokens_per_sec_positive_for_successful_requests() {
    let mut collector = InferenceMetricsCollector::new();
    // 1000 tokens in 500 ms = 2 tok/ms = 2000 tok/s
    collector.record(entry("model-fast", 500, 1000, 0.01, true));
    let m = collector.metrics_for("model-fast", TimeWindow::OneHour);
    assert!(m.avg_tokens_per_sec > 0.0, "expected positive throughput");
}

#[test]
fn avg_tokens_per_sec_zero_when_no_success() {
    let mut collector = InferenceMetricsCollector::new();
    collector.record(entry("model-dead", 100, 50, 0.01, false));
    let m = collector.metrics_for("model-dead", TimeWindow::OneHour);
    assert_eq!(m.avg_tokens_per_sec, 0.0);
}

// --- avg_cost ---

#[test]
fn avg_cost_is_mean_of_all_requests() {
    let mut collector = InferenceMetricsCollector::new();
    collector.record(entry("model-c", 100, 100, 0.10, true));
    collector.record(entry("model-c", 100, 100, 0.20, true));
    let m = collector.metrics_for("model-c", TimeWindow::OneHour);
    assert!((m.avg_cost - 0.15).abs() < 1e-9);
}

// --- rolling window eviction ---

#[test]
fn entries_outside_window_are_excluded() {
    let mut collector = InferenceMetricsCollector::new();
    // Old entry — 2 hours ago, outside the 1h window
    collector.record(entry_at("model-w", 999, 500, 1.0, false, Duration::hours(-2)));
    // Recent entry — within 1h
    collector.record(entry_at("model-w", 100, 100, 0.01, true, Duration::seconds(-30)));

    let m = collector.metrics_for("model-w", TimeWindow::OneHour);
    // Only the recent success should be counted
    assert_eq!(m.request_count, 1, "old entry should be excluded from 1h window");
    assert!((m.error_rate - 0.0).abs() < f64::EPSILON);
}

#[test]
fn twenty_four_hour_window_includes_recent_entries() {
    let mut collector = InferenceMetricsCollector::new();
    // 2 hours ago — outside 1h but inside 24h
    collector.record(entry_at("model-v", 200, 200, 0.02, true, Duration::hours(-2)));

    let m_1h = collector.metrics_for("model-v", TimeWindow::OneHour);
    let m_24h = collector.metrics_for("model-v", TimeWindow::TwentyFourHours);

    assert_eq!(m_1h.request_count, 0, "2h-old entry outside 1h window");
    assert_eq!(m_24h.request_count, 1, "2h-old entry inside 24h window");
}

#[test]
fn seven_day_window_includes_old_entries() {
    let mut collector = InferenceMetricsCollector::new();
    // 30 hours ago — outside 24h but inside 7d
    collector.record(entry_at("model-u", 150, 150, 0.015, true, Duration::hours(-30)));

    let m_24h = collector.metrics_for("model-u", TimeWindow::TwentyFourHours);
    let m_7d = collector.metrics_for("model-u", TimeWindow::SevenDays);

    assert_eq!(m_24h.request_count, 0, "30h-old entry outside 24h window");
    assert_eq!(m_7d.request_count, 1, "30h-old entry inside 7d window");
}

// --- unknown model ---

#[test]
fn metrics_for_unknown_model_returns_zero_counts() {
    let collector = InferenceMetricsCollector::new();
    let m = collector.metrics_for("nonexistent", TimeWindow::OneHour);
    assert_eq!(m.request_count, 0);
    assert_eq!(m.error_rate, 0.0);
    assert_eq!(m.latency_p50, 0);
    assert_eq!(m.avg_cost, 0.0);
}

// --- window_label ---

#[test]
fn window_label_matches_time_window() {
    let mut collector = InferenceMetricsCollector::new();
    collector.record(entry("model-l", 100, 100, 0.0, true));

    let m1 = collector.metrics_for("model-l", TimeWindow::OneHour);
    let m24 = collector.metrics_for("model-l", TimeWindow::TwentyFourHours);
    let m7 = collector.metrics_for("model-l", TimeWindow::SevenDays);

    assert_eq!(m1.window_label, "1h");
    assert_eq!(m24.window_label, "24h");
    assert_eq!(m7.window_label, "7d");
}

// --- JSON serialization ---

#[test]
fn model_metrics_serializes_to_json() {
    let metrics = ModelMetrics {
        model: "gpt-4".to_string(),
        request_count: 10,
        error_rate: 0.1,
        latency_p50: 200,
        latency_p95: 500,
        latency_p99: 800,
        avg_tokens_per_sec: 1500.0,
        avg_cost: 0.05,
        window_label: "1h".to_string(),
    };
    let json = serde_json::to_string(&metrics).expect("serialization must not fail");
    assert!(json.contains("\"model\":\"gpt-4\""));
    assert!(json.contains("\"request_count\":10"));
}
