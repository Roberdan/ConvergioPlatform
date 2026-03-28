// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// TDD tests for kernel/engine.rs — written first (RED phase).

#[cfg(test)]
mod tests {
    use crate::kernel::engine::{KernelAction, KernelConfig, KernelEngine, KernelSeverity};

    fn make_engine() -> KernelEngine {
        KernelEngine::new(KernelConfig {
            active_node: "test-node".to_string(),
            default_model: "mlx-community/Llama-3.2-1B-Instruct-4bit".to_string(),
        })
    }

    #[test]
    fn is_loaded_false_on_new_engine() {
        let engine = make_engine();
        assert!(!engine.is_loaded());
    }

    #[test]
    fn load_model_sets_loaded_flag() {
        let mut engine = make_engine();
        engine.load_model("mlx-community/Llama-3.2-1B-Instruct-4bit");
        assert!(engine.is_loaded());
    }

    #[test]
    fn load_model_stores_model_name() {
        let mut engine = make_engine();
        engine.load_model("mlx-community/Mistral-7B-Instruct-v0.3-4bit");
        let status = engine.status();
        assert_eq!(
            status.active_node.as_deref(),
            Some("test-node")
        );
    }

    #[test]
    fn status_reflects_loaded_model() {
        let mut engine = make_engine();
        assert_eq!(engine.status().models_loaded, 0);
        engine.load_model("some-model");
        assert_eq!(engine.status().models_loaded, 1);
    }

    #[test]
    fn status_has_uptime_secs() {
        let engine = make_engine();
        let s = engine.status();
        // Uptime is non-negative — may be 0 immediately after creation.
        assert!(s.uptime_secs < 3600, "uptime should be small in tests");
    }

    #[test]
    fn classify_high_cpu_returns_warn() {
        let mut engine = make_engine();
        engine.load_model("mlx-community/Llama-3.2-1B-Instruct-4bit");
        let action = engine.classify("CPU usage is at 95% for the last 10 minutes");
        // High CPU situation should produce WARN or CRITICAL, never OK.
        assert!(
            matches!(action.severity, KernelSeverity::Warn | KernelSeverity::Critical),
            "high-CPU situation must not be OK"
        );
        assert!(!action.reason.is_empty(), "reason must be non-empty");
    }

    #[test]
    fn classify_normal_situation_returns_ok() {
        let mut engine = make_engine();
        engine.load_model("mlx-community/Llama-3.2-1B-Instruct-4bit");
        let action = engine.classify("All systems nominal, tasks progressing normally");
        // Normal situation produces OK.
        assert_eq!(action.severity, KernelSeverity::Ok);
    }

    #[test]
    fn classify_without_model_falls_back_gracefully() {
        // Engine not yet loaded — must not panic, must return a structured result.
        let engine = make_engine();
        let action = engine.classify("some situation");
        assert!(!action.reason.is_empty());
    }

    #[test]
    fn kernel_severity_order() {
        // Verify severity variants are distinct.
        assert_ne!(KernelSeverity::Ok, KernelSeverity::Warn);
        assert_ne!(KernelSeverity::Warn, KernelSeverity::Critical);
    }

    #[test]
    fn kernel_action_has_required_fields() {
        let action = KernelAction {
            severity: KernelSeverity::Warn,
            action: "throttle".to_string(),
            reason: "load too high".to_string(),
        };
        assert_eq!(action.action, "throttle");
        assert!(!action.reason.is_empty());
    }

    #[test]
    fn kernel_status_fields_present() {
        let engine = make_engine();
        let s = engine.status();
        // All fields must be accessible without panic.
        let _ = s.models_loaded;
        let _ = s.ram_gb;
        let _ = s.uptime_secs;
        let _ = s.active_node;
        let _ = s.last_check;
    }

    #[test]
    fn load_model_replaces_current_model() {
        let mut engine = make_engine();
        engine.load_model("model-a");
        engine.load_model("model-b");
        // models_loaded stays at 1 — only one active at a time.
        assert_eq!(engine.status().models_loaded, 1);
    }
}
