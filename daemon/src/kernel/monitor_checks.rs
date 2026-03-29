// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Extended monitor checks: stale locks, API telemetry health.

use super::monitor::{KernelCheckResult, HTTP_TIMEOUT};
use reqwest::Client;
use std::time::Duration;

/// Scan /tmp for stale .lock files older than `threshold_secs`.
pub fn detect_stale_locks(threshold_secs: u64) -> KernelCheckResult {
    let cutoff = Duration::from_secs(threshold_secs);
    let now = std::time::SystemTime::now();
    let stale: Vec<_> = std::fs::read_dir("/tmp").into_iter().flatten().flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "lock"))
        .filter(|e| std::fs::metadata(e.path()).ok()
            .and_then(|m| m.modified().ok())
            .map(|t| now.duration_since(t).unwrap_or_default() > cutoff)
            .unwrap_or(false))
        .map(|e| e.path().display().to_string())
        .collect();
    if stale.is_empty() { KernelCheckResult::pass("stale_locks") }
    else { KernelCheckResult::fail("stale_locks", &format!("stale: {}", stale.join(", "))) }
}

/// Check /api/telemetry for high error rate (>10%) or slow endpoints (avg >1s).
pub async fn check_api_telemetry(daemon_url: &str) -> KernelCheckResult {
    let c = Client::builder().timeout(HTTP_TIMEOUT).build().unwrap_or_default();
    match c.get(format!("{daemon_url}/api/telemetry")).send().await {
        Err(e) => KernelCheckResult::fail("api_telemetry", &e.to_string()),
        Ok(r) => match r.json::<serde_json::Value>().await {
            Err(e) => KernelCheckResult::fail("api_telemetry", &e.to_string()),
            Ok(j) => {
                let error_rate = j["error_rate"].as_f64().unwrap_or(0.0);
                let empty = vec![];
                let slow: Vec<_> = j["endpoints"].as_array().unwrap_or(&empty).iter()
                    .filter(|e| e["avg_ms"].as_f64().unwrap_or(0.0) > 1000.0)
                    .filter_map(|e| e["path"].as_str())
                    .collect();
                if error_rate > 10.0 {
                    KernelCheckResult::fail("api_telemetry", &format!("error rate {error_rate:.1}%"))
                } else if !slow.is_empty() {
                    KernelCheckResult::fail("api_telemetry", &format!("slow: {}", slow.join(",")))
                } else {
                    KernelCheckResult::pass("api_telemetry")
                }
            }
        }
    }
}
