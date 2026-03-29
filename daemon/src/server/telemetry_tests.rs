// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Tests for API telemetry module.
// WHY serial: tests share global atomic counters and must not interleave.

use super::*;
use std::sync::Mutex;

// Serialize tests that touch global state.
static TEST_LOCK: Mutex<()> = Mutex::new(());

// --- normalise_path ---

#[test]
fn normalise_replaces_numeric_segments() {
    assert_eq!(normalise_path("/api/metrics/run/42"), "/api/metrics/run/:id");
    assert_eq!(normalise_path("/api/plan-db/json/1234"), "/api/plan-db/json/:id");
}

#[test]
fn normalise_preserves_non_numeric() {
    assert_eq!(normalise_path("/api/health"), "/api/health");
    assert_eq!(normalise_path("/api/mesh/status"), "/api/mesh/status");
}

#[test]
fn normalise_handles_empty_and_root() {
    assert_eq!(normalise_path("/"), "/");
    assert_eq!(normalise_path(""), "");
}

// --- EndpointStats ---

#[test]
fn endpoint_stats_records_correctly() {
    let mut stats = EndpointStats::new();
    stats.record(50, false);
    stats.record(150, false);
    stats.record(500, true);
    assert_eq!(stats.count, 3);
    assert_eq!(stats.errors, 1);
    assert_eq!(stats.total_ms, 700);
    assert_eq!(stats.max_ms, 500);
}

#[test]
fn endpoint_stats_avg_ms() {
    let mut stats = EndpointStats::new();
    assert_eq!(stats.avg_ms(), 0.0);
    stats.record(100, false);
    stats.record(200, false);
    assert!((stats.avg_ms() - 150.0).abs() < 0.01);
}

#[test]
fn endpoint_stats_histogram_buckets() {
    let mut stats = EndpointStats::new();
    // 3ms should land in bucket <=5
    stats.record(3, false);
    assert!(stats.histogram[0] >= 1, "3ms should be in <=5ms bucket");
    // 1000ms should land in bucket <=1000
    stats.record(1000, false);
    let idx_1000 = HISTOGRAM_BUCKETS.iter().position(|&b| b == 1000).unwrap();
    assert!(stats.histogram[idx_1000] >= 1, "1000ms should be in <=1000ms bucket");
}

// --- Global counters (serialised via TEST_LOCK) ---

#[test]
fn record_request_increments_totals() {
    let _g = TEST_LOCK.lock().unwrap();
    reset();
    record_request("/api/health", 10, false);
    record_request("/api/health", 20, true);
    record_request("/api/plans", 5, false);
    let snap = snapshot();
    assert_eq!(snap["total_requests"], 3);
    assert_eq!(snap["total_errors"], 1);
    assert!(snap["endpoints"].as_array().unwrap().len() >= 2);
}

#[test]
fn record_request_groups_by_normalised_path() {
    let _g = TEST_LOCK.lock().unwrap();
    reset();
    record_request("/api/metrics/run/1", 10, false);
    record_request("/api/metrics/run/2", 20, false);
    record_request("/api/metrics/run/99", 30, false);
    let snap = snapshot();
    let endpoints = snap["endpoints"].as_array().unwrap();
    let grouped = endpoints.iter().find(|e| e["path"] == "/api/metrics/run/:id");
    assert!(grouped.is_some(), "expected grouped endpoint");
    assert_eq!(grouped.unwrap()["count"], 3);
}

#[test]
fn snapshot_error_rate_calculation() {
    let _g = TEST_LOCK.lock().unwrap();
    reset();
    record_request("/api/a", 1, false);
    record_request("/api/b", 1, true);
    let snap = snapshot();
    let rate = snap["error_rate"].as_f64().unwrap();
    assert!((rate - 50.0).abs() < 0.1, "expected ~50%, got {rate}");
}

#[test]
fn snapshot_empty_returns_zeros() {
    let _g = TEST_LOCK.lock().unwrap();
    reset();
    let snap = snapshot();
    assert_eq!(snap["total_requests"], 0);
    assert_eq!(snap["total_errors"], 0);
    assert_eq!(snap["error_rate"], 0.0);
    assert!(snap["endpoints"].as_array().unwrap().is_empty());
}

#[test]
fn reset_clears_all_state() {
    let _g = TEST_LOCK.lock().unwrap();
    record_request("/api/test", 10, false);
    reset();
    let snap = snapshot();
    assert_eq!(snap["total_requests"], 0);
    assert!(snap["endpoints"].as_array().unwrap().is_empty());
}
