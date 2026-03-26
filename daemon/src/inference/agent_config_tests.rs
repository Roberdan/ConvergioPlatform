#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::path::Path;
    use tempfile::TempDir;

    use crate::inference::agent_config::{
        apply_agent_constraints, AgentConfigRegistry, AgentInferenceConfig, BudgetTracker,
    };
    use crate::inference::types::{InferenceConstraints, InferenceRequest, InferenceTier};

    fn write_agent_yaml(dir: &Path, name: &str, yaml: &str) {
        let agent_dir = dir.join(name);
        std::fs::create_dir_all(&agent_dir).unwrap();
        let mut f = std::fs::File::create(agent_dir.join("inference.yaml")).unwrap();
        f.write_all(yaml.as_bytes()).unwrap();
    }

    fn make_req(agent_id: &str, tier: InferenceTier) -> InferenceRequest {
        InferenceRequest {
            prompt: "test".to_string(),
            max_tokens: 100,
            tier_hint: Some(tier),
            agent_id: agent_id.to_string(),
            constraints: InferenceConstraints { max_latency_ms: None, max_cost: None },
        }
    }

    fn make_cfg(max_tier: InferenceTier, budget: u64) -> AgentInferenceConfig {
        AgentInferenceConfig {
            preferred_model: None,
            max_tier,
            budget_tokens_per_day: budget,
            latency_sla_ms: 5000,
            allowed_models: vec![],
        }
    }

    // ── Deserialization ────────────────────────────────────────────────────────

    #[test]
    fn test_deserialize_full_config() {
        let yaml = "preferred_model: sonnet\nmax_tier: t3\nbudget_tokens_per_day: 1000000\nlatency_sla_ms: 5000\nallowed_models:\n  - haiku\n  - sonnet\n  - opus\n";
        let cfg: AgentInferenceConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cfg.preferred_model, Some("sonnet".to_string()));
        assert_eq!(cfg.max_tier, InferenceTier::T3Complex);
        assert_eq!(cfg.budget_tokens_per_day, 1_000_000);
        assert_eq!(cfg.allowed_models, vec!["haiku", "sonnet", "opus"]);
    }

    #[test]
    fn test_deserialize_minimal_no_preferred_model() {
        let yaml = "max_tier: t2\nbudget_tokens_per_day: 500000\nlatency_sla_ms: 3000\nallowed_models: []\n";
        let cfg: AgentInferenceConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cfg.preferred_model, None);
        assert_eq!(cfg.max_tier, InferenceTier::T2Standard);
    }

    #[test]
    fn test_tier_deserialization_all_variants() {
        for (s, expected) in [
            ("t1", InferenceTier::T1Trivial),
            ("t2", InferenceTier::T2Standard),
            ("t3", InferenceTier::T3Complex),
            ("t4", InferenceTier::T4Critical),
        ] {
            let yaml = format!("max_tier: {s}\nbudget_tokens_per_day: 0\nlatency_sla_ms: 0\nallowed_models: []");
            let cfg: AgentInferenceConfig = serde_yaml::from_str(&yaml).unwrap();
            assert_eq!(cfg.max_tier, expected, "failed for {s}");
        }
    }

    // ── AgentConfigRegistry ────────────────────────────────────────────────────

    #[test]
    fn test_registry_loads_multiple_agents() {
        let tmp = TempDir::new().unwrap();
        let yaml = "preferred_model: sonnet\nmax_tier: t3\nbudget_tokens_per_day: 1000000\nlatency_sla_ms: 5000\nallowed_models: [haiku, sonnet]";
        write_agent_yaml(tmp.path(), "coordinator", yaml);
        write_agent_yaml(tmp.path(), "executor", "max_tier: t2\nbudget_tokens_per_day: 200000\nlatency_sla_ms: 2000\nallowed_models: [haiku]");

        let registry = AgentConfigRegistry::load_directory(tmp.path()).unwrap();
        assert!(registry.get("coordinator").is_some());
        assert!(registry.get("executor").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_registry_get_returns_correct_fields() {
        let tmp = TempDir::new().unwrap();
        write_agent_yaml(tmp.path(), "planner", "preferred_model: opus\nmax_tier: t4\nbudget_tokens_per_day: 2000000\nlatency_sla_ms: 10000\nallowed_models: [opus]");

        let registry = AgentConfigRegistry::load_directory(tmp.path()).unwrap();
        let cfg = registry.get("planner").unwrap();
        assert_eq!(cfg.preferred_model, Some("opus".to_string()));
        assert_eq!(cfg.max_tier, InferenceTier::T4Critical);
    }

    #[test]
    fn test_registry_empty_dir_returns_ok() {
        let tmp = TempDir::new().unwrap();
        let registry = AgentConfigRegistry::load_directory(tmp.path()).unwrap();
        assert!(registry.get("anyone").is_none());
    }

    #[test]
    fn test_registry_default_config_is_permissive() {
        let default = AgentConfigRegistry::default_config();
        assert_eq!(default.max_tier, InferenceTier::T4Critical);
        assert!(default.budget_tokens_per_day >= 1_000_000);
    }

    // ── BudgetTracker ──────────────────────────────────────────────────────────

    #[test]
    fn test_budget_initial_zero() {
        assert_eq!(BudgetTracker::new().tokens_used_today("x"), 0);
    }

    #[test]
    fn test_budget_accumulates() {
        let mut t = BudgetTracker::new();
        t.record_usage("a", 500);
        t.record_usage("a", 300);
        assert_eq!(t.tokens_used_today("a"), 800);
    }

    #[test]
    fn test_budget_independent_agents() {
        let mut t = BudgetTracker::new();
        t.record_usage("a", 1000);
        t.record_usage("b", 200);
        assert_eq!(t.tokens_used_today("a"), 1000);
        assert_eq!(t.tokens_used_today("b"), 200);
    }

    #[test]
    fn test_budget_over_budget_detection() {
        let mut t = BudgetTracker::new();
        t.record_usage("a", 900_000);
        let cfg = make_cfg(InferenceTier::T3Complex, 1_000_000);
        assert!(!t.is_over_budget("a", &cfg));
        t.record_usage("a", 200_000);
        assert!(t.is_over_budget("a", &cfg));
    }

    // ── apply_agent_constraints ────────────────────────────────────────────────

    #[test]
    fn test_clamps_to_max_tier() {
        let t = BudgetTracker::new();
        let cfg = make_cfg(InferenceTier::T2Standard, 1_000_000);
        let req = make_req("x", InferenceTier::T4Critical);
        assert_eq!(apply_agent_constraints(&req, &cfg, &t), InferenceTier::T2Standard);
    }

    #[test]
    fn test_no_clamp_when_within_max() {
        let t = BudgetTracker::new();
        let cfg = make_cfg(InferenceTier::T4Critical, 1_000_000);
        let req = make_req("x", InferenceTier::T2Standard);
        assert_eq!(apply_agent_constraints(&req, &cfg, &t), InferenceTier::T2Standard);
    }

    #[test]
    fn test_downgrade_when_over_budget() {
        let mut t = BudgetTracker::new();
        t.record_usage("x", 1_100_000);
        let cfg = make_cfg(InferenceTier::T3Complex, 1_000_000);
        let req = make_req("x", InferenceTier::T3Complex);
        assert_eq!(apply_agent_constraints(&req, &cfg, &t), InferenceTier::T2Standard);
    }

    #[test]
    fn test_downgrade_floor_is_t1() {
        let mut t = BudgetTracker::new();
        t.record_usage("x", 999_999_999);
        let cfg = make_cfg(InferenceTier::T1Trivial, 100);
        let req = make_req("x", InferenceTier::T1Trivial);
        assert_eq!(apply_agent_constraints(&req, &cfg, &t), InferenceTier::T1Trivial);
    }

    #[test]
    fn test_no_tier_hint_defaults_to_t1() {
        let t = BudgetTracker::new();
        let cfg = make_cfg(InferenceTier::T4Critical, 1_000_000);
        let mut req = make_req("x", InferenceTier::T1Trivial);
        req.tier_hint = None;
        assert_eq!(apply_agent_constraints(&req, &cfg, &t), InferenceTier::T1Trivial);
    }
}
