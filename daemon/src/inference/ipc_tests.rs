// Tests for InferenceIpcHandler — validates IPC command dispatch and JSON serialization.
use super::ipc::{InferenceIpcHandler, InferenceIpcResponse};
use crate::inference::{
    health::HealthChecker,
    metrics::InferenceMetricsCollector,
    router::InferenceRouter,
    types::{InferenceTier, ModelEndpoint, ModelProvider},
};

fn make_router_with_model(name: &str, healthy: bool) -> InferenceRouter {
    let mut router = InferenceRouter::new();
    router.register_model(ModelEndpoint {
        name: name.to_string(),
        provider: ModelProvider::Local,
        url: format!("http://localhost/{}", name),
        tier_range: (InferenceTier::T1Trivial, InferenceTier::T4Critical),
        healthy,
    });
    router
}

fn make_handler(healthy_model: bool) -> InferenceIpcHandler {
    let router = make_router_with_model("gemma-3-1b", healthy_model);
    let metrics = InferenceMetricsCollector::new();
    let checker = HealthChecker::new(vec!["gemma-3-1b"]);
    InferenceIpcHandler::new(router, metrics, checker)
}

// --- inference.route ---

#[test]
fn route_command_returns_ok_response() {
    let handler = make_handler(true);
    let payload = serde_json::json!({
        "prompt": "What is 2+2?",
        "max_tokens": 64,
        "tier_hint": "T1Trivial",
        "agent_id": "test-agent",
        "constraints": { "max_latency_ms": null, "max_cost": null }
    })
    .to_string();

    let result = handler.handle_command("inference.route", &payload);
    assert!(result.is_ok(), "expected ok, got: {:?}", result);

    let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(json["ok"], true);
    assert!(json["data"]["model_used"].is_string());
    assert_eq!(json["data"]["model_used"], "gemma-3-1b");
}

#[test]
fn route_command_returns_error_when_no_healthy_model() {
    let handler = make_handler(false); // unhealthy model
    let payload = serde_json::json!({
        "prompt": "Hello",
        "max_tokens": 32,
        "tier_hint": null,
        "agent_id": "agent-x",
        "constraints": { "max_latency_ms": null, "max_cost": null }
    })
    .to_string();

    let result = handler.handle_command("inference.route", &payload);
    // Should return Ok(JSON) with ok=false, not Err
    assert!(result.is_ok());
    let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(json["ok"], false);
    assert!(json["error"].is_string());
}

#[test]
fn route_command_rejects_malformed_payload() {
    let handler = make_handler(true);
    let result = handler.handle_command("inference.route", "not-valid-json");
    assert!(result.is_ok()); // always returns JSON
    let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(json["ok"], false);
}

// --- inference.status ---

#[test]
fn status_command_returns_health_for_all_endpoints() {
    let handler = make_handler(true);
    let result = handler.handle_command("inference.status", "{}");
    assert!(result.is_ok());
    let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(json["ok"], true);
    assert!(json["data"].is_array());
}

#[test]
fn status_command_payload_is_ignored() {
    let handler = make_handler(true);
    // Even empty or malformed payload must work for status
    let result = handler.handle_command("inference.status", "");
    assert!(result.is_ok());
    let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(json["ok"], true);
}

// --- inference.metrics ---

#[test]
fn metrics_command_returns_empty_list_when_no_observations() {
    let handler = make_handler(true);
    let result = handler.handle_command("inference.metrics", r#"{"window":"1h"}"#);
    assert!(result.is_ok());
    let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(json["ok"], true);
    assert!(json["data"].is_array());
    // No observations recorded, so empty
    assert_eq!(json["data"].as_array().unwrap().len(), 0);
}

#[test]
fn metrics_command_defaults_to_one_hour_window() {
    let handler = make_handler(true);
    // No window specified — should default to 1h
    let result = handler.handle_command("inference.metrics", "{}");
    assert!(result.is_ok());
    let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(json["ok"], true);
}

#[test]
fn metrics_command_accepts_24h_window() {
    let handler = make_handler(true);
    let result = handler.handle_command("inference.metrics", r#"{"window":"24h"}"#);
    assert!(result.is_ok());
    let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(json["ok"], true);
}

#[test]
fn metrics_command_accepts_7d_window() {
    let handler = make_handler(true);
    let result = handler.handle_command("inference.metrics", r#"{"window":"7d"}"#);
    assert!(result.is_ok());
    let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(json["ok"], true);
}

// --- unknown command ---

#[test]
fn unknown_command_returns_error_response() {
    let handler = make_handler(true);
    let result = handler.handle_command("inference.unknown", "{}");
    assert!(result.is_ok());
    let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(json["ok"], false);
    assert!(json["error"].as_str().unwrap().contains("unknown command"));
}

// --- InferenceIpcResponse serialization ---

#[test]
fn ipc_response_ok_serializes_correctly() {
    let resp = InferenceIpcResponse::ok(serde_json::json!({"key": "value"}));
    let s = serde_json::to_string(&resp).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["key"], "value");
    assert!(parsed["error"].is_null());
}

#[test]
fn ipc_response_err_serializes_correctly() {
    let resp = InferenceIpcResponse::err("something went wrong".to_string());
    let s = serde_json::to_string(&resp).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(parsed["ok"], false);
    assert!(parsed["data"].is_null());
    assert_eq!(parsed["error"], "something went wrong");
}
