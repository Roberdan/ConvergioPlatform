use super::*;

#[test]
fn goal_decompose_saas() {
    let r = decompose("Launch recipe SaaS");
    assert!(r.domains.iter().any(|d| d.domain == "technology"));
    assert!(r.domains.iter().any(|d| d.agent == "baccio-tech-architect"));
    assert_eq!(r.waves.len(), 5);
    assert!(r.estimated_tasks > 0);
}

#[test]
fn goal_id_slug() {
    let id = goal_id("Launch recipe SaaS");
    assert!(id.starts_with("goal-launch"));
}

#[test]
fn domain_agent_mapping() {
    assert_eq!(Domain::Technology.agent(), "baccio-tech-architect");
    assert_eq!(Domain::Design.agent(), "sara-ux-ui-designer");
    assert_eq!(Domain::Marketing.agent(), "sofia-marketing-strategist");
    assert_eq!(Domain::Finance.agent(), "amy-cfo");
    assert_eq!(Domain::Strategy.agent(), "antonio-strategy-expert");
}

#[test]
fn empty_goal_fallback() {
    let r = decompose("XYZ unknown 42");
    assert!(!r.domains.is_empty());
    assert!(r.domains.iter().any(|d| d.domain == "strategy"));
}

#[test]
fn strategy_always_present() {
    let r = decompose("Build a mobile app");
    assert!(r.domains.iter().any(|d| d.domain == "strategy"));
}

#[test]
fn wave_order_correct() {
    let r = decompose("Launch SaaS platform");
    let names: Vec<&str> = r.waves.iter().map(|w| w.name.as_str()).collect();
    assert_eq!(names, ["research", "design", "build", "validate", "launch"]);
    assert_eq!(r.waves[2].order, 3); // build is wave 3
}
