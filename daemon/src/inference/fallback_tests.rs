/// Tests for inference fallback chains (F-06).
///
/// Verifies: default chains, YAML loading, chain lookup, FallbackExecutor with max 3 attempts,
/// fallback triggers (error, health-down), and logging of each fallback with reason.
#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    use crate::inference::fallback::{
        FallbackChain, FallbackConfig, FallbackExecutor, FallbackResult,
    };
    use crate::inference::types::InferenceTier;

    // --- FallbackChain ---

    #[test]
    fn fallback_chain_holds_ordered_models() {
        let chain = FallbackChain::new(vec!["local".into(), "haiku".into(), "sonnet".into()]);
        assert_eq!(chain.models(), &["local", "haiku", "sonnet"]);
    }

    // --- FallbackConfig::default_chains ---

    #[test]
    fn default_t1_chain_is_local_haiku_sonnet() {
        let cfg = FallbackConfig::default_chains();
        let chain = cfg.chain_for(&InferenceTier::T1Trivial);
        assert_eq!(chain, &["local", "haiku", "sonnet"]);
    }

    #[test]
    fn default_t2_chain_is_haiku_local_sonnet() {
        let cfg = FallbackConfig::default_chains();
        let chain = cfg.chain_for(&InferenceTier::T2Standard);
        assert_eq!(chain, &["haiku", "local", "sonnet"]);
    }

    #[test]
    fn default_t3_chain_is_sonnet_opus() {
        let cfg = FallbackConfig::default_chains();
        let chain = cfg.chain_for(&InferenceTier::T3Complex);
        assert_eq!(chain, &["sonnet", "opus"]);
    }

    #[test]
    fn default_t4_chain_is_opus_sonnet() {
        let cfg = FallbackConfig::default_chains();
        let chain = cfg.chain_for(&InferenceTier::T4Critical);
        assert_eq!(chain, &["opus", "sonnet"]);
    }

    #[test]
    fn default_max_attempts_is_3() {
        let cfg = FallbackConfig::default_chains();
        assert_eq!(cfg.max_attempts(), 3);
    }

    // --- FallbackConfig::load (YAML) ---

    #[test]
    fn load_from_valid_yaml_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fallback.yaml");
        std::fs::write(
            &path,
            r#"
max_attempts: 2
chains:
  t1: [gemma, haiku]
  t2: [haiku, gemma]
  t3: [sonnet, opus]
  t4: [opus, sonnet]
"#,
        )
        .unwrap();

        let cfg = FallbackConfig::load(&path).expect("should parse");
        assert_eq!(cfg.max_attempts(), 2);
        assert_eq!(cfg.chain_for(&InferenceTier::T1Trivial), &["gemma", "haiku"]);
    }

    #[test]
    fn load_returns_error_for_missing_file() {
        let result = FallbackConfig::load(Path::new("/nonexistent/fallback.yaml"));
        assert!(result.is_err());
    }

    #[test]
    fn load_returns_error_for_invalid_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.yaml");
        std::fs::write(&path, "not: valid: yaml: [").unwrap();
        let result = FallbackConfig::load(&path);
        assert!(result.is_err());
    }

    // --- FallbackExecutor::execute_with_fallback ---

    #[test]
    fn succeeds_on_first_attempt_when_no_error() {
        let chain = vec!["local".to_string(), "haiku".to_string()];
        let result = FallbackExecutor::execute_with_fallback(&chain, 3, |model| {
            if model == "local" {
                Ok("ok".to_string())
            } else {
                Err("should not reach".to_string())
            }
        });

        let r = result.expect("should succeed");
        assert_eq!(r.model_used, "local");
        assert_eq!(r.attempt, 1);
        assert!(r.fallback_reason.is_none());
    }

    #[test]
    fn falls_back_to_second_model_on_first_error() {
        let chain = vec!["local".to_string(), "haiku".to_string()];
        let result = FallbackExecutor::execute_with_fallback(&chain, 3, |model| {
            if model == "local" {
                Err("timeout".to_string())
            } else {
                Ok("ok".to_string())
            }
        });

        let r = result.expect("should succeed via fallback");
        assert_eq!(r.model_used, "haiku");
        assert_eq!(r.attempt, 2);
        assert_eq!(r.fallback_reason.as_deref(), Some("timeout"));
    }

    #[test]
    fn stops_after_max_attempts_and_returns_err() {
        let chain = vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(), // would be attempt 4 — must not be reached
        ];
        let attempts_made = Arc::new(Mutex::new(0usize));
        let counter = attempts_made.clone();

        let result = FallbackExecutor::execute_with_fallback(&chain, 3, move |_model| {
            *counter.lock().unwrap() += 1;
            Err::<String, String>("error".to_string())
        });

        assert!(result.is_err());
        // max_attempts = 3, so exactly 3 attempts despite 4 models in chain
        assert_eq!(*attempts_made.lock().unwrap(), 3);
    }

    #[test]
    fn returns_err_when_chain_is_empty() {
        let chain: Vec<String> = vec![];
        let result =
            FallbackExecutor::execute_with_fallback(&chain, 3, |_| Ok("never".to_string()));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty chain"));
    }

    #[test]
    fn fallback_result_records_last_fallback_reason() {
        // First fails with "health=down", second succeeds
        let chain = vec!["primary".to_string(), "secondary".to_string()];
        let result = FallbackExecutor::execute_with_fallback(&chain, 3, |model| {
            if model == "primary" {
                Err("health=down".to_string())
            } else {
                Ok("ok".to_string())
            }
        });

        let r = result.unwrap();
        assert_eq!(r.fallback_reason.as_deref(), Some("health=down"));
    }

    // --- FallbackResult fields ---

    #[test]
    fn fallback_result_exposes_model_used_and_attempt() {
        let r = FallbackResult {
            model_used: "sonnet".to_string(),
            attempt: 2,
            fallback_reason: Some("timeout".to_string()),
        };
        assert_eq!(r.model_used, "sonnet");
        assert_eq!(r.attempt, 2);
        assert_eq!(r.fallback_reason.as_deref(), Some("timeout"));
    }
}
