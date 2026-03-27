use super::registry::ArtifactRegistry;
use super::types::{Artifact, ArtifactType, Maturity};
use serde_json::json;

fn make_artifact(name: &str, atype: ArtifactType, domain: &str) -> Artifact {
    Artifact {
        id: 0,
        artifact_type: atype,
        name: name.to_string(),
        description: format!("Test {name}"),
        domain: domain.to_string(),
        maturity: Maturity::Stable,
        source_path: format!("/test/{name}.md"),
        file_hash: "abc123".to_string(),
        model: None,
        constraints: vec![],
        metadata: json!({}),
    }
}

#[test]
fn register_and_get() {
    let reg = ArtifactRegistry::new();
    let id = reg.register(make_artifact("dario-debugger", ArtifactType::Agent, "debug")).unwrap();
    let a = reg.get(id).unwrap();
    assert_eq!(a.name, "dario-debugger");
}

#[test]
fn idempotent_upsert() {
    let reg = ArtifactRegistry::new();
    let a = make_artifact("rex-reviewer", ArtifactType::Agent, "review");
    let id1 = reg.register(a.clone()).unwrap();
    let id2 = reg.register(a).unwrap();
    assert_eq!(id1, id2);
    assert_eq!(reg.count(), 1);
}

#[test]
fn list_with_filters() {
    let reg = ArtifactRegistry::new();
    reg.register(make_artifact("agent-a", ArtifactType::Agent, "security")).unwrap();
    reg.register(make_artifact("skill-b", ArtifactType::Skill, "security")).unwrap();
    reg.register(make_artifact("agent-c", ArtifactType::Agent, "devops")).unwrap();

    let agents = reg.list(Some(ArtifactType::Agent), None, None);
    assert_eq!(agents.len(), 2);

    let sec = reg.list(None, Some("security"), None);
    assert_eq!(sec.len(), 2);

    let sec_agents = reg.list(Some(ArtifactType::Agent), Some("security"), None);
    assert_eq!(sec_agents.len(), 1);
}

#[test]
fn sorted_by_name() {
    let reg = ArtifactRegistry::new();
    reg.register(make_artifact("zebra", ArtifactType::Rule, "a")).unwrap();
    reg.register(make_artifact("alpha", ArtifactType::Rule, "a")).unwrap();
    let list = reg.list(None, None, None);
    assert_eq!(list[0].name, "alpha");
}

#[test]
fn get_not_found() {
    let reg = ArtifactRegistry::new();
    assert!(reg.get(999).is_err());
}
