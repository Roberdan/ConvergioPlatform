// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// API telemetry: request counters, response time histogram, endpoint metrics.
// WHY: Observability across all API handlers for health classification + MCP.

use axum::body::Body;
use axum::http::header::HeaderName;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::Instant;
use uuid::Uuid;

tokio::task_local! {
    static REQUEST_TRACE_ID: String;
}

pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// Global request counter (total requests served since daemon start).
static TOTAL_REQUESTS: AtomicU64 = AtomicU64::new(0);

/// Global error counter (responses with 4xx/5xx status).
static TOTAL_ERRORS: AtomicU64 = AtomicU64::new(0);

/// Per-endpoint metrics stored in a global map.
static ENDPOINT_METRICS: RwLock<Option<HashMap<String, EndpointStats>>> =
    RwLock::new(None);

/// Histogram bucket boundaries in milliseconds.
const HISTOGRAM_BUCKETS: &[u64] = &[5, 10, 25, 50, 100, 250, 500, 1000, 5000];

#[derive(Debug, Clone)]
pub struct EndpointStats {
    pub count: u64,
    pub errors: u64,
    pub total_ms: u64,
    pub max_ms: u64,
    /// Histogram buckets: count of requests <= each boundary.
    pub histogram: Vec<u64>,
}

impl EndpointStats {
    fn new() -> Self {
        Self {
            count: 0,
            errors: 0,
            total_ms: 0,
            max_ms: 0,
            histogram: vec![0; HISTOGRAM_BUCKETS.len()],
        }
    }

    fn record(&mut self, duration_ms: u64, is_error: bool) {
        self.count += 1;
        self.total_ms += duration_ms;
        if duration_ms > self.max_ms {
            self.max_ms = duration_ms;
        }
        if is_error {
            self.errors += 1;
        }
        for (i, &boundary) in HISTOGRAM_BUCKETS.iter().enumerate() {
            if duration_ms <= boundary {
                self.histogram[i] += 1;
            }
        }
    }

    pub fn avg_ms(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.total_ms as f64 / self.count as f64
        }
    }
}

/// Record a request to the given endpoint path with its duration and status.
pub fn record_request(path: &str, duration_ms: u64, is_error: bool) {
    TOTAL_REQUESTS.fetch_add(1, Ordering::Relaxed);
    if is_error {
        TOTAL_ERRORS.fetch_add(1, Ordering::Relaxed);
    }
    // Normalise path: strip numeric segments for grouping (e.g. /api/metrics/run/42 → /api/metrics/run/:id)
    let normalised = normalise_path(path);
    if let Ok(mut guard) = ENDPOINT_METRICS.write() {
        let map = guard.get_or_insert_with(HashMap::new);
        map.entry(normalised).or_insert_with(EndpointStats::new).record(duration_ms, is_error);
    }
}

/// Normalise numeric path segments to `:id` for metric grouping.
fn normalise_path(path: &str) -> String {
    path.split('/')
        .map(|seg| if seg.chars().all(|c| c.is_ascii_digit()) && !seg.is_empty() { ":id" } else { seg })
        .collect::<Vec<_>>()
        .join("/")
}

/// Get a snapshot of all telemetry data as JSON.
pub fn snapshot() -> Value {
    let total = TOTAL_REQUESTS.load(Ordering::Relaxed);
    let errors = TOTAL_ERRORS.load(Ordering::Relaxed);
    let endpoints: Vec<Value> = match ENDPOINT_METRICS.read() {
        Ok(guard) => guard.as_ref().map(|map| {
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by(|a, b| b.1.count.cmp(&a.1.count));
            entries.iter().map(|(path, stats)| json!({
                "path": path,
                "count": stats.count,
                "errors": stats.errors,
                "avg_ms": (stats.avg_ms() * 100.0).round() / 100.0,
                "max_ms": stats.max_ms,
                "histogram": HISTOGRAM_BUCKETS.iter().zip(&stats.histogram)
                    .map(|(b, c)| json!({"le_ms": b, "count": c}))
                    .collect::<Vec<_>>(),
            })).collect()
        }).unwrap_or_default(),
        Err(_) => vec![],
    };
    json!({
        "total_requests": total,
        "total_errors": errors,
        "error_rate": if total > 0 { (errors as f64 / total as f64 * 10000.0).round() / 100.0 } else { 0.0 },
        "endpoints": endpoints,
    })
}

pub fn current_request_id() -> Option<String> {
    REQUEST_TRACE_ID.try_with(Clone::clone).ok() // intentional: request ID only available inside telemetry middleware scope
}

/// Reset all counters (useful for testing).
pub fn reset() {
    TOTAL_REQUESTS.store(0, Ordering::Relaxed);
    TOTAL_ERRORS.store(0, Ordering::Relaxed);
    if let Ok(mut guard) = ENDPOINT_METRICS.write() {
        *guard = None;
    }
}

/// Axum middleware: records request count and response time for every request.
pub async fn telemetry_layer(mut req: Request<Body>, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let request_id = req
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok()) // intentional: header may not be valid UTF-8 string
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    req.extensions_mut().insert(request_id.clone());
    let start = Instant::now();
    let mut response = REQUEST_TRACE_ID
        .scope(request_id.clone(), async move { next.run(req).await })
        .await;
    let duration_ms = start.elapsed().as_millis() as u64;
    let is_error = response.status().is_client_error() || response.status().is_server_error();
    if let Ok(header_value) = request_id.parse() {
        response.headers_mut().insert(
            HeaderName::from_static(REQUEST_ID_HEADER),
            header_value,
        );
    }
    record_request(&path, duration_ms, is_error);
    response
}

#[cfg(test)]
#[path = "telemetry_tests.rs"]
mod tests;
