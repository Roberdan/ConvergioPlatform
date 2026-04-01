//! Tests for org factory — mission-based and repo-based org design.

use super::factory::{design_org_from_mission, design_org_from_repo, slugify};
use super::repo_scanner::{CiInfo, RepoProfile, RepoStructure};

#[test]
fn slugify_basic() {
    assert_eq!(slugify("My Cool Org"), "my-cool-org");
    assert_eq!(slugify("  hello world  "), "hello-world");
    assert_eq!(slugify("A--B  C"), "a-b-c");
    assert_eq!(slugify("café123"), "caf-123");
}

#[test]
fn mission_fitness_has_training_dept() {
    let bp = design_org_from_mission("FitCo", "A fitness app for daily training", 500.0);
    assert_eq!(bp.slug, "fitco");
    assert_eq!(bp.ceo_agent, "fitco-ceo");
    assert!(bp.departments.iter().any(|d| d.name == "Training"));
    assert!(bp.departments.iter().any(|d| d.name == "Nutrition"));
    assert_eq!(bp.budget_usd, Some(500.0));
}

#[test]
fn mission_software_has_dev_dept() {
    let bp = design_org_from_mission("DevHub", "Build a SaaS platform for code review", 1000.0);
    assert!(bp.departments.iter().any(|d| d.name == "Development"));
    assert!(bp.departments.iter().any(|d| d.name == "QA"));
    assert!(bp.departments.iter().any(|d| d.name == "DevOps"));
}

#[test]
fn mission_marketing_has_content_dept() {
    let bp = design_org_from_mission("GrowthLab", "marketing automation for sales teams", 750.0);
    assert!(bp.departments.iter().any(|d| d.name == "Marketing"));
    assert!(bp.departments.iter().any(|d| d.name == "Content"));
}

#[test]
fn mission_default_departments() {
    let bp = design_org_from_mission("GenericOrg", "a mysterious venture", 200.0);
    assert!(bp.departments.iter().any(|d| d.name == "Strategy"));
    assert!(bp.departments.iter().any(|d| d.name == "Execution"));
}

#[test]
fn night_agents_always_include_daily_report() {
    let bp = design_org_from_mission("AnyOrg", "anything at all", 100.0);
    assert!(bp.night_agents.iter().any(|n| n.schedule == "daily_report"));
}

#[test]
fn mission_software_night_agents() {
    let bp = design_org_from_mission("CodeCo", "software platform for developers", 500.0);
    assert!(bp.night_agents.iter().any(|n| n.schedule == "monitor_prs"));
    assert!(bp.night_agents.iter().any(|n| n.schedule == "dep_update"));
}

fn rust_repo_profile() -> RepoProfile {
    RepoProfile {
        path: "/home/user/projects/iron-api".to_string(),
        languages: vec![("Rust".to_string(), 15000)],
        frameworks: vec!["actix-web".to_string()],
        structure: RepoStructure {
            has_src: true,
            has_tests: true,
            has_docs: false,
            has_ci: true,
            manifest_files: vec!["Cargo.toml".to_string()],
        },
        ci: Some(CiInfo {
            provider: "github-actions".to_string(),
            workflows: vec!["ci.yml".to_string()],
        }),
        readme_summary: "A Rust API server".to_string(),
        total_files: 42,
        total_lines: 15000,
        dependencies: vec!["actix-web".to_string(), "serde".to_string()],
    }
}

#[test]
fn repo_rust_has_backend_dept() {
    let profile = rust_repo_profile();
    let bp = design_org_from_repo(&profile, None, 800.0);
    assert_eq!(bp.name, "iron-api");
    assert_eq!(bp.slug, "iron-api");
    assert!(bp.departments.iter().any(|d| d.name == "Backend"));
    let backend = bp.departments.iter().find(|d| d.name == "Backend").unwrap();
    assert!(backend.agents.iter().any(|a| a.name.contains("rust-dev")));
}

#[test]
fn repo_with_ci_gets_devops_dept() {
    let profile = rust_repo_profile();
    let bp = design_org_from_repo(&profile, None, 800.0);
    assert!(bp.departments.iter().any(|d| d.name == "DevOps"));
}

#[test]
fn repo_with_ci_gets_pr_monitor_night_agent() {
    let profile = rust_repo_profile();
    let bp = design_org_from_repo(&profile, None, 800.0);
    assert!(bp.night_agents.iter().any(|n| n.schedule == "monitor_prs"));
    assert!(bp.night_agents.iter().any(|n| n.schedule == "test_coverage"));
}

#[test]
fn repo_night_agents_always_have_daily_report() {
    let profile = rust_repo_profile();
    let bp = design_org_from_repo(&profile, None, 800.0);
    assert!(bp.night_agents.iter().any(|n| n.schedule == "daily_report"));
    assert!(bp.night_agents.iter().any(|n| n.schedule == "stale_branch_cleanup"));
}

#[test]
fn repo_with_deps_gets_dep_update_night_agent() {
    let profile = rust_repo_profile();
    let bp = design_org_from_repo(&profile, None, 800.0);
    assert!(bp.night_agents.iter().any(|n| n.schedule == "dependency_update"));
}

#[test]
fn repo_name_override() {
    let profile = rust_repo_profile();
    let bp = design_org_from_repo(&profile, Some("CustomName"), 500.0);
    assert_eq!(bp.name, "CustomName");
    assert_eq!(bp.slug, "customname");
}

#[test]
fn all_night_agents_use_haiku() {
    let bp = design_org_from_mission("TestOrg", "software platform", 100.0);
    for agent in &bp.night_agents {
        assert_eq!(
            agent.model, "claude-haiku-4-5",
            "night agent {} should use haiku", agent.name,
        );
    }
}
