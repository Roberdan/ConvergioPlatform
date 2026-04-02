//! Global orgchart renderer — all orgs in one ecosystem view.

use serde::Serialize;
use std::collections::BTreeMap;

/// A single member within an org.
#[derive(Debug, Clone, Serialize)]
pub struct OrgMember {
    pub agent: String,
    pub role: String,
    pub department: String,
}

/// A plan linked to an org.
#[derive(Debug, Clone, Serialize)]
pub struct OrgPlanInfo {
    pub name: String,
    pub status: String,
}

/// Summary of a single org for the global chart.
#[derive(Debug, Clone, Serialize)]
pub struct OrgSummary {
    pub slug: String,
    pub name: String,
    pub org_type: String,
    pub agent_count: usize,
    pub plan_count: usize,
    pub status: String,
    pub ceo_agent: String,
    pub members: Vec<OrgMember>,
    pub plans: Vec<OrgPlanInfo>,
}

/// Render a box-drawn ASCII chart showing all orgs in the ecosystem.
pub fn render_global_orgchart(
    orgs: &[OrgSummary],
    mesh_summary: &str,
    total_agents: usize,
    total_plans: usize,
) -> String {
    let w = 60;
    let top = format!("\u{250c}\u{2500}\u{2500} Convergio Ecosystem {}\u{2510}", "\u{2500}".repeat(w - 22));
    let bot = format!("\u{2514}{}\u{2518}", "\u{2500}".repeat(w));
    let mut lines: Vec<String> = Vec::new();
    lines.push(top);
    lines.push(pad_line("", w));

    let verticals: Vec<&OrgSummary> = orgs.iter().filter(|o| o.org_type == "vertical").collect();
    let horizontals: Vec<&OrgSummary> = orgs.iter().filter(|o| o.org_type != "vertical").collect();

    lines.push(pad_line("  VERTICAL ORGS (project-specific):", w));
    if verticals.is_empty() {
        lines.push(pad_line("    (none)", w));
    } else {
        render_org_list(&verticals, w, &mut lines);
    }
    lines.push(pad_line("", w));

    lines.push(pad_line("  HORIZONTAL ORGS (cross-project services):", w));
    if horizontals.is_empty() {
        lines.push(pad_line("    (none)", w));
    } else {
        render_org_list(&horizontals, w, &mut lines);
    }
    lines.push(pad_line("", w));

    lines.push(pad_line(&format!("  MESH: {mesh_summary}"), w));
    lines.push(pad_line(&format!("  AGENTS: {total_agents} total"), w));
    lines.push(pad_line(&format!("  PLANS: {total_plans}"), w));
    lines.push(bot);

    lines.join("\n")
}

fn render_org_list(orgs: &[&OrgSummary], w: usize, lines: &mut Vec<String>) {
    let count = orgs.len();
    for (i, org) in orgs.iter().enumerate() {
        let is_last = i == count - 1;
        let branch = if is_last { "\u{2514}\u{2500}\u{2500}" } else { "\u{251c}\u{2500}\u{2500}" };
        let cont = if is_last { "   " } else { "\u{2502}  " };
        lines.push(pad_line(&format!("  {} {} ({})", branch, org.name, org.status), w));

        // CEO line
        if !org.ceo_agent.is_empty() {
            lines.push(pad_line(&format!("  {}   CEO: {}", cont, org.ceo_agent), w));
        }

        // Group members by department
        let depts = group_by_department(&org.members);
        let dept_keys: Vec<&String> = depts.keys().collect();
        let dept_count = dept_keys.len();
        for (di, dept) in dept_keys.iter().enumerate() {
            let is_last_dept = di == dept_count - 1;
            let d_branch = if is_last_dept { "\u{2514}\u{2500}\u{2500}" } else { "\u{251c}\u{2500}\u{2500}" };
            let d_cont = if is_last_dept { "   " } else { "\u{2502}  " };
            lines.push(pad_line(&format!("  {}   {} {}", cont, d_branch, dept), w));
            let agents = &depts[*dept];
            for (ai, m) in agents.iter().enumerate() {
                let is_last_a = ai == agents.len() - 1;
                let a_branch = if is_last_a { "\u{2514}\u{2500}\u{2500}" } else { "\u{251c}\u{2500}\u{2500}" };
                let label = format!("{} ({})", m.agent, m.role);
                lines.push(pad_line(&format!("  {}   {} {} {}", cont, d_cont, a_branch, label), w));
            }
        }

        // Plans
        if !org.plans.is_empty() {
            lines.push(pad_line(&format!("  {}   Plans:", cont), w));
            for p in &org.plans {
                lines.push(pad_line(&format!("  {}     - {} [{}]", cont, p.name, p.status), w));
            }
        }
    }
}

fn group_by_department(members: &[OrgMember]) -> BTreeMap<String, Vec<&OrgMember>> {
    let mut map: BTreeMap<String, Vec<&OrgMember>> = BTreeMap::new();
    for m in members {
        let dept = if m.department.is_empty() { "General" } else { &m.department };
        map.entry(dept.to_string()).or_default().push(m);
    }
    map
}

/// Pad or truncate text into a bordered line of width w.
fn pad_line(text: &str, w: usize) -> String {
    let content = if text.len() > w { &text[..w] } else { text };
    format!("\u{2502}{:<width$}\u{2502}", content, width = w)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_orgs() -> Vec<OrgSummary> {
        vec![
            OrgSummary {
                slug: "convergio-platform".into(),
                name: "Convergio Platform".into(),
                org_type: "vertical".into(),
                agent_count: 2,
                plan_count: 1,
                status: "active".into(),
                ceo_agent: "claude-m5max-lead".into(),
                members: vec![
                    OrgMember { agent: "api-builder".into(), role: "architect".into(), department: "Core".into() },
                    OrgMember { agent: "test-runner".into(), role: "qa".into(), department: "QA".into() },
                ],
                plans: vec![OrgPlanInfo { name: "Phase 21".into(), status: "doing".into() }],
            },
            OrgSummary {
                slug: "infra-ops".into(),
                name: "Infra Ops".into(),
                org_type: "horizontal".into(),
                agent_count: 1,
                plan_count: 0,
                status: "active".into(),
                ceo_agent: "ops-lead".into(),
                members: vec![
                    OrgMember { agent: "deploy-bot".into(), role: "operator".into(), department: "Deploy".into() },
                ],
                plans: vec![],
            },
        ]
    }

    #[test]
    fn test_global_orgchart_contains_sections() {
        let orgs = sample_orgs();
        let chart = render_global_orgchart(&orgs, "M5Max <-> M1Pro", 3, 1);
        assert!(chart.contains("Convergio Ecosystem"));
        assert!(chart.contains("VERTICAL ORGS"));
        assert!(chart.contains("HORIZONTAL ORGS"));
        assert!(chart.contains("Convergio Platform"));
        assert!(chart.contains("CEO: claude-m5max-lead"));
        assert!(chart.contains("api-builder (architect)"));
        assert!(chart.contains("test-runner (qa)"));
        assert!(chart.contains("Phase 21 [doing]"));
        assert!(chart.contains("AGENTS: 3 total"));
    }

    #[test]
    fn test_global_orgchart_empty_orgs() {
        let chart = render_global_orgchart(&[], "none", 0, 0);
        assert!(chart.contains("(none)"));
        assert!(chart.contains("VERTICAL ORGS"));
        assert!(chart.contains("HORIZONTAL ORGS"));
    }

    #[test]
    fn test_global_orgchart_departments_grouped() {
        let orgs = vec![OrgSummary {
            slug: "dev".into(),
            name: "Dev Team".into(),
            org_type: "vertical".into(),
            agent_count: 2,
            plan_count: 0,
            status: "active".into(),
            ceo_agent: "lead".into(),
            members: vec![
                OrgMember { agent: "alice".into(), role: "dev".into(), department: "UI".into() },
                OrgMember { agent: "bob".into(), role: "dev".into(), department: "UI".into() },
            ],
            plans: vec![],
        }];
        let chart = render_global_orgchart(&orgs, "standalone", 2, 0);
        assert!(chart.contains("UI"));
        assert!(chart.contains("alice (dev)"));
        assert!(chart.contains("bob (dev)"));
    }
}
