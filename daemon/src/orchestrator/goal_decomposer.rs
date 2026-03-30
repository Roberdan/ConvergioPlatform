// Goal decomposer: maps natural-language goals to multi-domain execution plans.
// Each domain maps to a catalog agent; waves follow research→design→build→validate→launch.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Domain {
    Technology,
    Product,
    Design,
    Marketing,
    Sales,
    Legal,
    Operations,
    Finance,
    People,
    Strategy,
}

impl Domain {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Technology => "technology",
            Self::Product => "product",
            Self::Design => "design",
            Self::Marketing => "marketing",
            Self::Sales => "sales",
            Self::Legal => "legal",
            Self::Operations => "operations",
            Self::Finance => "finance",
            Self::People => "people",
            Self::Strategy => "strategy",
        }
    }

    /// Catalog agent assigned to this domain.
    pub fn agent(&self) -> &'static str {
        match self {
            Self::Technology => "baccio-tech-architect",
            Self::Product => "marcello-pm",
            Self::Design => "sara-ux-ui-designer",
            Self::Marketing => "sofia-marketing-strategist",
            Self::Sales => "fabio-sales-business-development",
            Self::Legal => "elena-legal-compliance-expert",
            Self::Operations => "enrico-business-process-engineer",
            Self::Finance => "amy-cfo",
            Self::People => "giulia-hr-talent-acquisition",
            Self::Strategy => "antonio-strategy-expert",
        }
    }

    fn keywords(&self) -> &'static [&'static str] {
        match self {
            Self::Technology => &[
                "tech", "code", "api", "backend", "frontend", "software", "infra",
                "deploy", "build", "database", "saas", "app", "platform", "system", "server",
            ],
            Self::Product => &[
                "product", "feature", "mvp", "roadmap", "sprint", "backlog", "user story",
            ],
            Self::Design => &[
                "design", "ui", "ux", "interface", "brand", "visual", "logo", "prototype",
                "wireframe",
            ],
            Self::Marketing => &[
                "marketing", "campaign", "content", "seo", "social", "ads", "growth", "launch",
                "awareness",
            ],
            Self::Sales => &[
                "sales", "customer", "client", "revenue", "pipeline", "crm", "deal", "outreach",
                "b2b",
            ],
            Self::Legal => &[
                "legal", "contract", "compliance", "privacy", "gdpr", "terms", "license", "ip",
                "trademark",
            ],
            Self::Operations => &[
                "operations", "process", "workflow", "ops", "support", "onboarding", "automation",
                "sla",
            ],
            Self::Finance => &[
                "finance", "budget", "cost", "pricing", "funding", "roi", "financial", "invoice",
                "subscription",
            ],
            Self::People => &[
                "hiring", "team", "hr", "people", "recruit", "talent", "culture", "employee",
            ],
            Self::Strategy => &[
                "strategy", "vision", "mission", "market", "competitor", "positioning", "goal",
                "objective", "expansion",
            ],
        }
    }

    /// Estimated task count contributed by this domain per base wave.
    pub fn tasks_per_wave(&self) -> u32 {
        match self {
            Self::Technology => 5,
            Self::Product => 4,
            Self::Design => 3,
            Self::Marketing => 3,
            Self::Sales => 2,
            Self::Legal => 2,
            Self::Operations => 2,
            Self::Finance => 2,
            Self::People => 2,
            Self::Strategy => 3,
        }
    }
}

/// Standard wave names and relative task multipliers (build is double).
pub const WAVE_NAMES: &[&str] = &["research", "design", "build", "validate", "launch"];
const WAVE_MULTIPLIERS: &[u32] = &[1, 1, 2, 1, 1];

#[derive(Debug, Serialize, Deserialize)]
pub struct DomainPlan {
    pub domain: String,
    pub agent: String,
    pub estimated_tasks: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Wave {
    pub name: String,
    pub order: u32,
    pub domains: Vec<String>,
    pub estimated_tasks: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DecomposedGoal {
    pub goal: String,
    pub goal_id: String,
    pub domains: Vec<DomainPlan>,
    pub agents: Vec<String>,
    pub waves: Vec<Wave>,
    pub estimated_tasks: u32,
}

/// Detect relevant domains from a natural-language goal using keyword matching.
/// Always includes Strategy; falls back to [Strategy, Technology] when nothing matches.
pub fn detect_domains(goal: &str) -> Vec<Domain> {
    let lower = goal.to_lowercase();
    let all = [
        Domain::Technology,
        Domain::Product,
        Domain::Design,
        Domain::Marketing,
        Domain::Sales,
        Domain::Legal,
        Domain::Operations,
        Domain::Finance,
        Domain::People,
        Domain::Strategy,
    ];
    let mut found: Vec<Domain> = all
        .into_iter()
        .filter(|d| d.keywords().iter().any(|kw| lower.contains(kw)))
        .collect();

    if found.is_empty() {
        return vec![Domain::Strategy, Domain::Technology];
    }
    if !found.contains(&Domain::Strategy) {
        found.push(Domain::Strategy);
    }
    found
}

/// Deterministic goal_id slug derived from goal text.
pub fn goal_id(goal: &str) -> String {
    let slug = goal
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .take(5)
        .collect::<Vec<_>>()
        .join("-");
    format!("goal-{slug}")
}

/// Decompose a goal into a multi-domain wave plan (pure, no I/O).
pub fn decompose(goal: &str) -> DecomposedGoal {
    let domains = detect_domains(goal);
    let domain_plans: Vec<DomainPlan> = domains
        .iter()
        .map(|d| DomainPlan {
            domain: d.label().to_string(),
            agent: d.agent().to_string(),
            estimated_tasks: d.tasks_per_wave(),
        })
        .collect();
    let agents: Vec<String> = domain_plans.iter().map(|d| d.agent.clone()).collect();
    let domain_labels: Vec<String> = domains.iter().map(|d| d.label().to_string()).collect();
    let base_tasks: u32 = domains.iter().map(|d| d.tasks_per_wave()).sum();

    let waves: Vec<Wave> = WAVE_NAMES
        .iter()
        .zip(WAVE_MULTIPLIERS.iter())
        .enumerate()
        .map(|(i, (name, mult))| Wave {
            name: name.to_string(),
            order: i as u32 + 1,
            domains: domain_labels.clone(),
            estimated_tasks: base_tasks * mult,
        })
        .collect();

    let estimated_tasks: u32 = waves.iter().map(|w| w.estimated_tasks).sum();
    DecomposedGoal {
        goal: goal.to_string(),
        goal_id: goal_id(goal),
        domains: domain_plans,
        agents,
        waves,
        estimated_tasks,
    }
}

#[cfg(test)]
mod tests {
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
}
