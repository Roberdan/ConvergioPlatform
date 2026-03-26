// IPC integration layer for the inference router.
//
// Agents send JSON-encoded commands through the standard IPC protocol.
// This module translates between raw IPC strings and the typed inference API.
//
// Supported commands:
//   inference.route   — route an InferenceRequest and return InferenceResponse
//   inference.status  — return health status for all tracked endpoints
//   inference.metrics — return ModelMetrics for a requested time window
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    health::HealthChecker,
    metrics::{InferenceMetricsCollector, TimeWindow},
    router::InferenceRouter,
    types::InferenceRequest,
};

// ---- public response type ------------------------------------------------

/// Envelope returned for every IPC command — always valid JSON.
#[derive(Debug, Serialize, Deserialize)]
pub struct InferenceIpcResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl InferenceIpcResponse {
    /// Build a successful response wrapping arbitrary JSON data.
    pub fn ok(data: Value) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    /// Build an error response with a human-readable message.
    pub fn err(message: String) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(message),
        }
    }

    /// Serialise to a JSON string; panics only when serde itself is broken.
    fn into_json(self) -> String {
        serde_json::to_string(&self).unwrap_or_else(|e| {
            format!(r#"{{"ok":false,"error":"serialization failure: {}"}}"#, e)
        })
    }
}

// ---- metrics window parsing ----------------------------------------------

/// Payload for `inference.metrics`.
#[derive(Debug, Deserialize, Default)]
struct MetricsPayload {
    window: Option<String>,
}

fn parse_window(label: Option<&str>) -> TimeWindow {
    match label {
        Some("24h") => TimeWindow::TwentyFourHours,
        Some("7d") => TimeWindow::SevenDays,
        _ => TimeWindow::OneHour, // default
    }
}

// ---- health snapshot type ------------------------------------------------

/// Serializable summary of one endpoint's health status.
#[derive(Debug, Serialize)]
struct EndpointStatusSummary {
    name: String,
    status: String,
    detail: Option<String>,
}

// ---- handler -------------------------------------------------------------

/// Dispatches IPC commands to the inference subsystem.
///
/// All three sub-systems (router, metrics, health) are owned here so the
/// handler can be constructed and used independently in tests.
pub struct InferenceIpcHandler {
    router: InferenceRouter,
    metrics: InferenceMetricsCollector,
    health: HealthChecker,
}

impl InferenceIpcHandler {
    pub fn new(
        router: InferenceRouter,
        metrics: InferenceMetricsCollector,
        health: HealthChecker,
    ) -> Self {
        Self { router, metrics, health }
    }

    /// Dispatch one IPC command and return a JSON-encoded `InferenceIpcResponse`.
    ///
    /// Never returns `Err` — all failures are encoded as `ok=false` JSON so
    /// callers always receive valid data.
    pub fn handle_command(&self, command: &str, payload: &str) -> Result<String, String> {
        let response = match command {
            "inference.route" => self.handle_route(payload),
            "inference.status" => self.handle_status(),
            "inference.metrics" => self.handle_metrics(payload),
            other => InferenceIpcResponse::err(format!("unknown command: {}", other)),
        };
        Ok(response.into_json())
    }

    // ---- command handlers ------------------------------------------------

    fn handle_route(&self, payload: &str) -> InferenceIpcResponse {
        let request: InferenceRequest = match serde_json::from_str(payload) {
            Ok(r) => r,
            Err(e) => {
                return InferenceIpcResponse::err(format!("invalid payload: {}", e));
            }
        };

        match self.router.route(&request) {
            Ok(response) => match serde_json::to_value(&response) {
                Ok(v) => InferenceIpcResponse::ok(v),
                Err(e) => InferenceIpcResponse::err(format!("serialization error: {}", e)),
            },
            Err(msg) => InferenceIpcResponse::err(msg),
        }
    }

    fn handle_status(&self) -> InferenceIpcResponse {
        // Build a status snapshot from the HealthChecker's known endpoints.
        // We rely on the caller having registered endpoint names at construction.
        let summaries: Vec<EndpointStatusSummary> = self
            .health
            .endpoint_names()
            .iter()
            .map(|name| {
                use super::health::EndpointHealthStatus;
                let (status_str, detail) = match self.health.status(name) {
                    EndpointHealthStatus::Healthy => ("healthy".to_string(), None),
                    EndpointHealthStatus::Degraded(reason) => {
                        ("degraded".to_string(), Some(reason))
                    }
                    EndpointHealthStatus::Down => ("down".to_string(), None),
                };
                EndpointStatusSummary {
                    name: name.to_string(),
                    status: status_str,
                    detail,
                }
            })
            .collect();

        match serde_json::to_value(&summaries) {
            Ok(v) => InferenceIpcResponse::ok(v),
            Err(e) => InferenceIpcResponse::err(format!("serialization error: {}", e)),
        }
    }

    fn handle_metrics(&self, payload: &str) -> InferenceIpcResponse {
        // Accept empty string or valid JSON; missing window defaults to 1h.
        let mp: MetricsPayload = if payload.trim().is_empty() {
            MetricsPayload::default()
        } else {
            serde_json::from_str(payload).unwrap_or_default()
        };

        let window = parse_window(mp.window.as_deref());
        let all = self.metrics.all_metrics(window);

        match serde_json::to_value(&all) {
            Ok(v) => InferenceIpcResponse::ok(v),
            Err(e) => InferenceIpcResponse::err(format!("serialization error: {}", e)),
        }
    }
}
