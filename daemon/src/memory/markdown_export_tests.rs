use super::*;
use crate::memory::types::{AccessLevel, Memory, MemoryType};
use chrono::Utc;
use tempfile::TempDir;

fn make_mem(id: &str, agent: &str, content: &str, tags: &[&str]) -> Memory {
    Memory {
        id: id.to_string(),
        agent_id: agent.to_string(),
        memory_type: MemoryType::Fact,
        content: content.to_string(),
        tags: tags.iter().map(|t| t.to_string()).collect(),
        created_at: Utc::now(),
        expires_at: None,
        access_level: AccessLevel::Private,
        attestations: vec![],
    }
}

#[test]
fn export_creates_file() {
    let dir = TempDir::new().unwrap();
    let mem = make_mem("m-001", "agent-x", "Kubernetes uses etcd", &["k8s"]);
    let path = export_memory(&mem, dir.path()).unwrap();
    assert!(path.exists());
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("m-001"));
    assert!(content.contains("Kubernetes uses etcd"));
}

#[test]
fn export_appends_to_existing() {
    let dir = TempDir::new().unwrap();
    let m1 = make_mem("m-001", "agent-x", "First fact", &["topic"]);
    let m2 = make_mem("m-002", "agent-x", "Second fact", &["topic"]);
    export_memory(&m1, dir.path()).unwrap();
    export_memory(&m2, dir.path()).unwrap();
    let path = dir.path().join("fact_topic.md");
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("m-001"));
    assert!(content.contains("m-002"));
}

#[test]
fn export_dedup_same_id() {
    let dir = TempDir::new().unwrap();
    let mem = make_mem("m-dup", "agent-x", "Duplicate entry", &["dedup"]);
    export_memory(&mem, dir.path()).unwrap();
    export_memory(&mem, dir.path()).unwrap();
    let path = dir.path().join("fact_dedup.md");
    let content = std::fs::read_to_string(&path).unwrap();
    assert_eq!(content.matches("m-dup").count(), 1);
}

#[test]
fn export_all_multiple() {
    let dir = TempDir::new().unwrap();
    let mems = vec![
        make_mem("m-a", "agent-x", "Fact A", &["alpha"]),
        make_mem("m-b", "agent-x", "Fact B", &["beta"]),
    ];
    let count = export_all(&mems, dir.path()).unwrap();
    assert_eq!(count, 2);
}

#[test]
fn slug_from_content_works() {
    assert_eq!(slug_from_content("Hello World Test"), "hello_world_test");
    assert_eq!(slug_from_content("a"), "a");
}

#[test]
fn import_roundtrip() {
    let dir = TempDir::new().unwrap();
    let mem = make_mem("m-rt", "agent-x", "Roundtrip test content", &["roundtrip"]);
    export_memory(&mem, dir.path()).unwrap();
    let imported = import_from_markdown(dir.path()).unwrap();
    assert!(!imported.is_empty());
    assert!(imported.iter().any(|m| m.id == "m-rt"));
}
