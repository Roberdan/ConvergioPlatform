// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Shared helpers for voice_router: HTTP utility, mute state, plan-list parsers.

use serde_json::Value;
use std::sync::Mutex;
use std::time::{Duration, Instant};

// Mute state: Option<Instant> = expiry; None = not muted.
pub(crate) static MUTE_UNTIL: Mutex<Option<Instant>> = Mutex::new(None);

pub(crate) fn get_json(url: &str) -> Result<Value, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;
    client.get(url).send().map_err(|e| e.to_string())?.json::<Value>().map_err(|e| e.to_string())
}

pub(crate) fn is_muted() -> bool {
    MUTE_UNTIL.lock().ok()
        .and_then(|g| *g)
        .map(|exp| Instant::now() < exp)
        .unwrap_or(false)
}

/// Parses /api/plan-db/list response: returns (doing_plan_count, remaining_tasks).
/// Plans with status=="doing" are active; remaining = sum of (tasks_total - tasks_done).
pub(crate) fn parse_status_from_plan_list(v: &Value) -> (u64, u64) {
    let plans = match v.get("plans").and_then(|p| p.as_array()) {
        Some(arr) => arr,
        None => return (0, 0),
    };
    let mut active: u64 = 0;
    let mut tasks_left: u64 = 0;
    for plan in plans {
        let status = plan.get("status").and_then(|s| s.as_str()).unwrap_or("");
        if status == "doing" {
            active += 1;
            let total = plan.get("tasks_total").and_then(|x| x.as_u64()).unwrap_or(0);
            let done = plan.get("tasks_done").and_then(|x| x.as_u64()).unwrap_or(0);
            tasks_left += total.saturating_sub(done);
        }
    }
    (active, tasks_left)
}

/// Parses /api/plan-db/list response: returns (total_cost, plan_count).
/// Sums total_cost across all plans (any status).
pub(crate) fn parse_cost_from_plan_list(v: &Value) -> (f64, usize) {
    let plans = match v.get("plans").and_then(|p| p.as_array()) {
        Some(arr) => arr,
        None => return (0.0, 0),
    };
    let mut total_cost: f64 = 0.0;
    for plan in plans {
        total_cost += plan.get("total_cost").and_then(|x| x.as_f64()).unwrap_or(0.0);
    }
    (total_cost, plans.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // --- parse_status_from_plan_list ---

    #[test]
    fn status_counts_only_doing_plans() {
        let payload = json!({
            "ok": true,
            "plans": [
                {"status": "doing", "tasks_total": 10, "tasks_done": 4},
                {"status": "doing", "tasks_total": 5,  "tasks_done": 5},
                {"status": "done",  "tasks_total": 8,  "tasks_done": 8},
                {"status": "todo",  "tasks_total": 3,  "tasks_done": 0},
            ]
        });
        let (active, left) = parse_status_from_plan_list(&payload);
        // Only the 2 "doing" plans count; second plan contributes 0 remaining tasks.
        assert_eq!(active, 2, "expected 2 doing plans");
        assert_eq!(left, 6, "expected 10-4 = 6 remaining tasks");
    }

    #[test]
    fn status_empty_plans_array() {
        let payload = json!({"ok": true, "plans": []});
        let (active, left) = parse_status_from_plan_list(&payload);
        assert_eq!(active, 0);
        assert_eq!(left, 0);
    }

    #[test]
    fn status_missing_plans_key() {
        let payload = json!({"ok": true});
        let (active, left) = parse_status_from_plan_list(&payload);
        assert_eq!(active, 0);
        assert_eq!(left, 0);
    }

    #[test]
    fn status_tasks_done_exceeds_total_saturates_at_zero() {
        // tasks_done > tasks_total should not underflow — saturating_sub guards this.
        let payload = json!({
            "ok": true,
            "plans": [
                {"status": "doing", "tasks_total": 2, "tasks_done": 5}
            ]
        });
        let (active, left) = parse_status_from_plan_list(&payload);
        assert_eq!(active, 1);
        assert_eq!(left, 0);
    }

    // --- parse_cost_from_plan_list ---

    #[test]
    fn cost_sums_all_plans() {
        let payload = json!({
            "ok": true,
            "plans": [
                {"status": "doing", "total_cost": 1.50},
                {"status": "done",  "total_cost": 2.25},
                {"status": "todo",  "total_cost": 0.75},
            ]
        });
        let (cost, count) = parse_cost_from_plan_list(&payload);
        assert!((cost - 4.50).abs() < 0.001, "expected 4.50, got {cost}");
        assert_eq!(count, 3);
    }

    #[test]
    fn cost_missing_total_cost_field_defaults_to_zero() {
        let payload = json!({
            "ok": true,
            "plans": [
                {"status": "doing"},
                {"status": "done", "total_cost": 1.0},
            ]
        });
        let (cost, count) = parse_cost_from_plan_list(&payload);
        assert!((cost - 1.0).abs() < 0.001);
        assert_eq!(count, 2);
    }

    #[test]
    fn cost_empty_plans() {
        let payload = json!({"ok": true, "plans": []});
        let (cost, count) = parse_cost_from_plan_list(&payload);
        assert_eq!(cost, 0.0);
        assert_eq!(count, 0);
    }
}
