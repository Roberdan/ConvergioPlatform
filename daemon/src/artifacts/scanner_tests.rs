use super::registry::ArtifactRegistry;
use super::scanner::scan_artifacts;
use tempfile::TempDir;

fn setup_test_repo(dir: &std::path::Path) {
    let agents_dir = dir.join("claude-config/agents");
    std::fs::create_dir_all(&agents_dir).unwrap();
    std::fs::write(
        agents_dir.join("dario-debugger.agent.md"),
        "---\nname: dario-debugger\ndescription: Systematic debugging\nmodel: opus-4.6\n---\n# Debug",
    ).unwrap();
    std::fs::write(
        agents_dir.join("rex-reviewer.agent.md"),
        "---\nname: rex-reviewer\ndescription: Code review\nmodel: sonnet-4.6\n---\n# Review",
    ).unwrap();

    let rules_dir = dir.join("claude-config/rules");
    std::fs::create_dir_all(&rules_dir).unwrap();
    std::fs::write(
        rules_dir.join("compliance.md"),
        "---\nname: compliance\ndescription: GDPR and security rules\n---\n# Rules",
    ).unwrap();
}

#[test]
fn scan_finds_agents_and_rules() {
    let dir = TempDir::new().unwrap();
    setup_test_repo(dir.path());
    let reg = ArtifactRegistry::new();
    let count = scan_artifacts(dir.path(), &reg).unwrap();
    assert_eq!(count, 3);
    assert_eq!(reg.count(), 3);
}

#[test]
fn scan_extracts_frontmatter() {
    let dir = TempDir::new().unwrap();
    setup_test_repo(dir.path());
    let reg = ArtifactRegistry::new();
    scan_artifacts(dir.path(), &reg).unwrap();
    let agents = reg.list(Some(super::types::ArtifactType::Agent), None, None);
    let dario = agents.iter().find(|a| a.name == "dario-debugger").unwrap();
    assert_eq!(dario.description, "Systematic debugging");
    assert_eq!(dario.model.as_deref(), Some("opus-4.6"));
}

#[test]
fn scan_empty_dir() {
    let dir = TempDir::new().unwrap();
    let reg = ArtifactRegistry::new();
    let count = scan_artifacts(dir.path(), &reg).unwrap();
    assert_eq!(count, 0);
}

#[test]
fn scan_idempotent() {
    let dir = TempDir::new().unwrap();
    setup_test_repo(dir.path());
    let reg = ArtifactRegistry::new();
    scan_artifacts(dir.path(), &reg).unwrap();
    scan_artifacts(dir.path(), &reg).unwrap();
    assert_eq!(reg.count(), 3, "re-scan should not duplicate");
}
