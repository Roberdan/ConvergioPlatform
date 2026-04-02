// Tests for cloud escalation and inference level logic.

use crate::kernel::cloud_escalation::CLOUD_MODEL;
use crate::kernel::engine::{InferenceLevel, KernelConfig, KernelEngine};

#[test]
fn test_cloud_model_is_opus() {
    assert!(CLOUD_MODEL.contains("opus"), "must target Opus");
}

#[test]
fn test_cloud_model_routes_to_claude() {
    let (p, _) = crate::server::provider::provider_for_model(CLOUD_MODEL);
    assert_eq!(p, crate::server::llm_client::Provider::ClaudeSubscription);
}

#[test]
fn test_inference_level_write_italian() {
    let mut eng = KernelEngine::new(KernelConfig::default());
    eng.load_model("test-model");
    assert_eq!(eng.inference_level_for("crea un piano"), InferenceLevel::Cloud);
}

#[test]
fn test_inference_level_write_english() {
    let mut eng = KernelEngine::new(KernelConfig::default());
    eng.load_model("test-model");
    assert_eq!(eng.inference_level_for("create a new org"), InferenceLevel::Cloud);
}

#[test]
fn test_inference_level_read_italian() {
    let mut eng = KernelEngine::new(KernelConfig::default());
    eng.load_model("test-model");
    assert_eq!(eng.inference_level_for("stato dei piani"), InferenceLevel::Local);
}

#[test]
fn test_inference_level_ali_escalation() {
    let mut eng = KernelEngine::new(KernelConfig::default());
    eng.load_model("test-model");
    assert_eq!(eng.inference_level_for("parla con ali"), InferenceLevel::Cloud);
}

#[test]
fn test_inference_level_no_model() {
    let eng = KernelEngine::new(KernelConfig::default());
    assert_eq!(eng.inference_level_for("qualsiasi cosa"), InferenceLevel::Cloud);
}

#[test]
fn test_inference_level_cost_query() {
    let mut eng = KernelEngine::new(KernelConfig::default());
    eng.load_model("test-model");
    assert_eq!(eng.inference_level_for("quanto costa"), InferenceLevel::Local);
}
