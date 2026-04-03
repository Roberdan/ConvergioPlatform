// Tests for cloud escalation and write-intent classification.

use crate::kernel::cloud_escalation::CLOUD_MODEL;
use crate::kernel::voice_router_helpers::is_write_intent;

#[test]
fn test_cloud_model_is_opus() {
    assert!(CLOUD_MODEL.contains("opus"), "must target Opus");
}

// Provider routing is tested only on macOS where the full provider table is
// compiled in.  On Linux CI this module may be built without the kernel
// feature that wires up ClaudeSubscription routing.
#[cfg(target_os = "macos")]
#[test]
fn test_cloud_model_routes_to_claude() {
    let (p, _) = crate::server::provider::provider_for_model(CLOUD_MODEL);
    assert_eq!(p, crate::server::llm_client::Provider::ClaudeSubscription);
}

#[test]
fn test_write_intent_italian() {
    assert!(is_write_intent("crea un piano"));
}

#[test]
fn test_write_intent_english() {
    assert!(is_write_intent("create a new org"));
}

#[test]
fn test_read_intent_italian() {
    assert!(!is_write_intent("stato dei piani"));
}

#[test]
fn test_write_intent_ali_escalation() {
    assert!(is_write_intent("parla con ali"));
}

// KernelEngine wraps AppleFmBridge which is Apple-Silicon-specific; only
// meaningful to run on macOS where the hardware/mlx_lm path is exercised.
#[cfg(target_os = "macos")]
#[test]
fn test_no_model_returns_cloud() {
    use crate::kernel::engine::{InferenceLevel, KernelConfig, KernelEngine};
    let eng = KernelEngine::new(KernelConfig::default());
    assert_eq!(eng.inference_level_for("qualsiasi cosa"), InferenceLevel::Cloud);
}

#[test]
fn test_read_intent_cost_query() {
    assert!(!is_write_intent("quanto costa"));
}
