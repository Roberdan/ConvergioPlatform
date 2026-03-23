// Cost Center API fetch — /api/metrics/cost and /api/metrics/summary

use reqwest::Client;
use serde_json::Value;

use crate::tui::{CostByDate, CostData, CostEntry, CostSummary};

/// Parse /api/metrics/cost JSON response into (Vec<CostEntry> by_model, Vec<CostEntry> by_project, Vec<CostByDate>).
/// Separated from fetch to allow unit testing without a live server.
pub fn parse_cost_response(v: &Value) -> (Vec<CostEntry>, Vec<CostEntry>, Vec<CostByDate>) {
    let by_model = v
        .get("by_model")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|item| CostEntry {
                    model: item
                        .get("model")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    calls: item.get("calls").and_then(Value::as_i64).unwrap_or(0),
                    cost_usd: item.get("cost_usd").and_then(Value::as_f64).unwrap_or(0.0),
                })
                .collect()
        })
        .unwrap_or_default();

    let by_project = v
        .get("by_project")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|item| CostEntry {
                    model: item
                        .get("project_id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    calls: item.get("calls").and_then(Value::as_i64).unwrap_or(0),
                    cost_usd: item.get("cost_usd").and_then(Value::as_f64).unwrap_or(0.0),
                })
                .collect()
        })
        .unwrap_or_default();

    let by_date = v
        .get("by_date")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|item| CostByDate {
                    date: item
                        .get("date")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    cost_usd: item.get("cost_usd").and_then(Value::as_f64).unwrap_or(0.0),
                })
                .collect()
        })
        .unwrap_or_default();

    (by_model, by_project, by_date)
}

/// Parse /api/metrics/summary JSON response into CostSummary.
pub fn parse_summary_response(v: &Value) -> CostSummary {
    CostSummary {
        run_count: v.get("run_count").and_then(Value::as_i64).unwrap_or(0),
        avg_duration_secs: v
            .get("avg_duration_secs")
            .and_then(Value::as_f64)
            .unwrap_or(0.0),
        total_cost_usd: v
            .get("total_cost_usd")
            .and_then(Value::as_f64)
            .unwrap_or(0.0),
    }
}

/// GET {api_url}/api/metrics/cost?days=7 -> CostData (by_model, by_project, by_date only; summary left default)
pub async fn fetch_cost(client: &Client, api_url: &str) -> CostData {
    let url = format!("{api_url}/api/metrics/cost?days=7");
    match client.get(&url).send().await {
        Ok(resp) => match resp.json::<Value>().await {
            Ok(v) => {
                let (by_model, by_project, by_date) = parse_cost_response(&v);
                CostData {
                    by_model,
                    by_project,
                    by_date,
                    summary: CostSummary::default(),
                }
            }
            Err(_) => CostData::default(),
        },
        Err(_) => CostData::default(),
    }
}

/// GET {api_url}/api/metrics/summary -> CostSummary
pub async fn fetch_metrics_summary(client: &Client, api_url: &str) -> CostSummary {
    let url = format!("{api_url}/api/metrics/summary");
    match client.get(&url).send().await {
        Ok(resp) => match resp.json::<Value>().await {
            Ok(v) => parse_summary_response(&v),
            Err(_) => CostSummary::default(),
        },
        Err(_) => CostSummary::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cost_response_maps_all_fields() {
        let json = serde_json::json!({
            "ok": true,
            "by_model": [
                {"model": "claude-opus-4.6", "calls": 42, "cost_usd": 1.25},
                {"model": "claude-sonnet-4.6", "calls": 10, "cost_usd": 0.30}
            ],
            "by_project": [
                {"project_id": "convergio", "calls": 100, "cost_usd": 3.50}
            ],
            "by_date": [
                {"date": "2026-03-23", "cost_usd": 0.75}
            ]
        });
        let (by_model, by_project, by_date) = parse_cost_response(&json);
        assert_eq!(by_model.len(), 2);
        assert_eq!(by_model[0].model, "claude-opus-4.6");
        assert_eq!(by_model[0].calls, 42);
        assert!((by_model[0].cost_usd - 1.25).abs() < f64::EPSILON);
        assert_eq!(by_project.len(), 1);
        assert_eq!(by_project[0].model, "convergio");
        assert!((by_project[0].cost_usd - 3.50).abs() < f64::EPSILON);
        assert_eq!(by_date.len(), 1);
        assert_eq!(by_date[0].date, "2026-03-23");
        assert!((by_date[0].cost_usd - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_cost_response_handles_empty() {
        let json = serde_json::json!({"ok": true});
        let (by_model, by_project, by_date) = parse_cost_response(&json);
        assert!(by_model.is_empty());
        assert!(by_project.is_empty());
        assert!(by_date.is_empty());
    }

    #[test]
    fn parse_summary_response_maps_fields() {
        let json = serde_json::json!({
            "ok": true,
            "run_count": 42,
            "avg_duration_secs": 1234.5,
            "total_cost_usd": 5.25
        });
        let summary = parse_summary_response(&json);
        assert_eq!(summary.run_count, 42);
        assert!((summary.avg_duration_secs - 1234.5).abs() < f64::EPSILON);
        assert!((summary.total_cost_usd - 5.25).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_summary_response_handles_empty() {
        let json = serde_json::json!({});
        let summary = parse_summary_response(&json);
        assert_eq!(summary.run_count, 0);
        assert_eq!(summary.avg_duration_secs, 0.0);
        assert_eq!(summary.total_cost_usd, 0.0);
    }
}
