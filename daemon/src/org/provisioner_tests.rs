//! Tests for orgchart rendering and ProvisionResult construction.

use super::factory::{AgentSpec, Department, NightAgentSpec, OrgBlueprint};
use super::orgchart::{render_orgchart, render_orgchart_compact};
use super::provisioner::ProvisionResult;

fn sample_blueprint() -> OrgBlueprint {
    OrgBlueprint {
        name: "Fitness Studio".into(),
        slug: "fitness-studio".into(),
        mission: "Lose 5 kg in 3 months".into(),
        repo_path: Some("/home/user/fitness".into()),
        budget_usd: Some(50.0),
        ceo_agent: "fitness-ceo".into(),
        departments: vec![
            Department {
                name: "Nutrition".into(),
                agents: vec![AgentSpec {
                    name: "nutritionist".into(),
                    model: "sonnet".into(),
                    capabilities: vec!["nutrition".into()],
                }],
            },
            Department {
                name: "Training".into(),
                agents: vec![AgentSpec {
                    name: "trainer".into(),
                    model: "sonnet".into(),
                    capabilities: vec!["fitness".into(), "plans".into()],
                }],
            },
        ],
        night_agents: vec![
            NightAgentSpec {
                name: "daily-report".into(),
                schedule: "daily".into(),
                time: "2:00".into(),
                model: "haiku".into(),
            },
            NightAgentSpec {
                name: "dep-update".into(),
                schedule: "weekly".into(),
                time: "5:00".into(),
                model: "haiku".into(),
            },
        ],
    }
}

#[test]
fn orgchart_contains_box_drawing() {
    let chart = render_orgchart(&sample_blueprint());
    assert!(chart.contains('┌'), "missing top-left corner");
    assert!(chart.contains('└'), "missing bottom-left corner");
    assert!(chart.contains("Fitness Studio"), "missing org name");
    assert!(chart.contains("fitness-ceo"), "missing CEO");
    assert!(chart.contains("Nutrition"), "missing department");
    assert!(chart.contains("nutritionist"), "missing agent");
    assert!(chart.contains("daily-report"), "missing night agent");
}

#[test]
fn compact_chart_is_shorter() {
    let bp = sample_blueprint();
    let full = render_orgchart(&bp);
    let compact = render_orgchart_compact(&bp);
    assert!(
        compact.len() < full.len(),
        "compact ({}) should be shorter than full ({})",
        compact.len(),
        full.len(),
    );
    assert!(!compact.contains('┌'), "compact must not use box drawing");
    assert!(compact.contains("Fitness Studio"), "compact missing name");
}

#[test]
fn provision_result_fields() {
    let r = ProvisionResult {
        org_id: 42,
        plan_id: 100,
        agents_created: 3,
        night_agents_scheduled: 2,
        tasks_created: 5,
    };
    assert_eq!(r.org_id, 42);
    assert_eq!(r.plan_id, 100);
    assert_eq!(r.agents_created, 3);
    assert_eq!(r.night_agents_scheduled, 2);
    assert_eq!(r.tasks_created, 5);
}
