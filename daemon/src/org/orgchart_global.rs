//! Global orgchart renderer — all orgs in one ecosystem view.

use serde::Serialize;

/// Summary of a single org for the global chart.
#[derive(Debug, Clone, Serialize)]
pub struct OrgSummary {
    pub slug: String,
    pub name: String,
    pub org_type: String,
    pub agent_count: usize,
    pub plan_count: usize,
    pub status: String,
}

/// Render a box-drawn ASCII chart showing all orgs in the ecosystem.
pub fn render_global_orgchart(
    orgs: &[OrgSummary],
    mesh_summary: &str,
    total_agents: usize,
    total_plans: usize,
) -> String {
    let w = 52;
    let top = format!("\u{250c}\u{2500}\u{2500} Convergio Ecosystem {}\u{2510}", "\u{2500}".repeat(w - 22));
    let bot = format!("\u{2514}{}\u{2518}", "\u{2500}".repeat(w));
    let mut lines: Vec<String> = Vec::new();
    lines.push(top);
    lines.push(pad_line("", w));

    // Split by org_type
    let verticals: Vec<&OrgSummary> = orgs.iter().filter(|o| o.org_type == "vertical").collect();
    let horizontals: Vec<&OrgSummary> = orgs.iter().filter(|o| o.org_type == "horizontal").collect();

    // Vertical orgs section
    lines.push(pad_line("  VERTICAL ORGS (project-specific):", w));
    if verticals.is_empty() {
        lines.push(pad_line("    (none)", w));
    } else {
        render_org_list(&verticals, w, &mut lines);
    }
    lines.push(pad_line("", w));

    // Horizontal orgs section
    lines.push(pad_line("  HORIZONTAL ORGS (cross-project services):", w));
    if horizontals.is_empty() {
        lines.push(pad_line("    (none)", w));
    } else {
        render_org_list(&horizontals, w, &mut lines);
    }
    lines.push(pad_line("", w));

    // Footer: mesh, agents, plans
    lines.push(pad_line(&format!("  MESH: {mesh_summary}"), w));
    lines.push(pad_line(&format!("  AGENTS: {total_agents} total"), w));
    lines.push(pad_line(&format!("  PLANS: {total_plans}"), w));
    lines.push(bot);

    lines.join("\n")
}

fn render_org_list(orgs: &[&OrgSummary], w: usize, lines: &mut Vec<String>) {
    let count = orgs.len();
    for (i, org) in orgs.iter().enumerate() {
        let branch = if i == count - 1 { "\u{2514}\u{2500}\u{2500}" } else { "\u{251c}\u{2500}\u{2500}" };
        let detail = format!(
            "  {} {} ({}, {} agents, {} plans)",
            branch, org.name, org.status, org.agent_count, org.plan_count,
        );
        lines.push(pad_line(&detail, w));
    }
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
                agent_count: 12,
                plan_count: 5,
                status: "active".into(),
            },
            OrgSummary {
                slug: "analytics-hub".into(),
                name: "Analytics Hub".into(),
                org_type: "vertical".into(),
                agent_count: 4,
                plan_count: 2,
                status: "active".into(),
            },
            OrgSummary {
                slug: "infra-ops".into(),
                name: "Infra Ops".into(),
                org_type: "horizontal".into(),
                agent_count: 6,
                plan_count: 3,
                status: "active".into(),
            },
            OrgSummary {
                slug: "security".into(),
                name: "Security".into(),
                org_type: "horizontal".into(),
                agent_count: 2,
                plan_count: 1,
                status: "standby".into(),
            },
        ]
    }

    #[test]
    fn test_global_orgchart_contains_sections() {
        let orgs = sample_orgs();
        let chart = render_global_orgchart(&orgs, "M5Max(coord) <-> M1Pro", 24, 11);

        assert!(chart.contains("Convergio Ecosystem"));
        assert!(chart.contains("VERTICAL ORGS"));
        assert!(chart.contains("HORIZONTAL ORGS"));
        assert!(chart.contains("Convergio Platform"));
        assert!(chart.contains("Analytics Hub"));
        assert!(chart.contains("Infra Ops"));
        assert!(chart.contains("Security"));
        assert!(chart.contains("MESH: M5Max(coord) <-> M1Pro"));
        assert!(chart.contains("AGENTS: 24 total"));
        assert!(chart.contains("PLANS: 11"));
    }

    #[test]
    fn test_global_orgchart_empty_orgs() {
        let chart = render_global_orgchart(&[], "none", 0, 0);
        assert!(chart.contains("(none)"));
        assert!(chart.contains("VERTICAL ORGS"));
        assert!(chart.contains("HORIZONTAL ORGS"));
    }

    #[test]
    fn test_global_orgchart_vertical_only() {
        let orgs = vec![OrgSummary {
            slug: "solo".into(),
            name: "Solo Project".into(),
            org_type: "vertical".into(),
            agent_count: 3,
            plan_count: 1,
            status: "active".into(),
        }];
        let chart = render_global_orgchart(&orgs, "standalone", 3, 1);
        assert!(chart.contains("Solo Project"));
        assert!(chart.contains("HORIZONTAL ORGS"));
        // Horizontal section should show (none)
        let horiz_idx = chart.find("HORIZONTAL ORGS").unwrap();
        let after = &chart[horiz_idx..];
        assert!(after.contains("(none)"));
    }
}
