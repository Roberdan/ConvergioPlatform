// When included from sqlite_store.rs via #[path], super = sqlite_store module.
use super::SqliteMemoryStore;
use super::super::types::{AccessLevel, Memory, MemoryType, RecallQuery};
use super::super::MemoryStore;
use chrono::{Duration, Utc};
use tempfile::NamedTempFile;

pub fn make_memory(agent_id: &str, content: &str, tags: &[&str]) -> Memory {
    Memory {
        id: String::new(),
        agent_id: agent_id.to_string(),
        memory_type: MemoryType::Fact,
        content: content.to_string(),
        tags: tags.iter().map(|t| t.to_string()).collect(),
        created_at: Utc::now(),
        expires_at: None,
        access_level: AccessLevel::Private,
        attestations: vec![],
    }
}

pub fn open_store() -> (SqliteMemoryStore, NamedTempFile) {
    let f = NamedTempFile::new().expect("tempfile");
    let store = SqliteMemoryStore::new(f.path().to_str().unwrap()).expect("store");
    (store, f)
}

// F-01: remember() inserts and returns a non-empty UUID
#[test]
fn remember_returns_uuid() {
    let (store, _f) = open_store();
    let mem = make_memory("agent-alpha", "The capital of Italy is Rome", &["geo", "italy"]);
    let id = store.remember(mem).expect("remember");
    assert!(!id.is_empty(), "id must be a non-empty UUID");
    assert_eq!(id.len(), 36, "UUID should be 36 chars");
}

// F-01: stored memory is retrievable via recall
#[test]
fn remember_then_recall_all() {
    let (store, _f) = open_store();
    let mem = make_memory("agent-beta", "Rust is systems language", &["rust", "lang"]);
    let id = store.remember(mem).expect("remember");
    let results = store
        .recall(RecallQuery { limit: 10, ..Default::default() })
        .expect("recall");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, id);
    assert_eq!(results[0].content, "Rust is systems language");
    assert_eq!(results[0].agent_id, "agent-beta");
}

// F-02: recall filters by memory_type
#[test]
fn recall_filters_by_type() {
    let (store, _f) = open_store();
    let mut fact = make_memory("agent-gamma", "SQLite supports FTS5", &[]);
    fact.memory_type = MemoryType::Fact;
    let mut decision = make_memory("agent-gamma", "Use WAL journal mode", &[]);
    decision.memory_type = MemoryType::Decision;
    store.remember(fact).expect("fact");
    store.remember(decision).expect("decision");
    let results = store
        .recall(RecallQuery {
            memory_type: Some(MemoryType::Fact),
            limit: 10,
            ..Default::default()
        })
        .expect("recall");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].memory_type, MemoryType::Fact);
}

// F-02: recall filters by tags
#[test]
fn recall_filters_by_tags() {
    let (store, _f) = open_store();
    store.remember(make_memory("agent-delta", "Content A", &["rust", "async"])).expect("a");
    store.remember(make_memory("agent-delta", "Content B", &["python"])).expect("b");
    store.remember(make_memory("agent-delta", "Content C", &["rust", "sync"])).expect("c");
    let results = store
        .recall(RecallQuery {
            tags: Some(vec!["rust".to_string()]),
            limit: 10,
            ..Default::default()
        })
        .expect("recall");
    assert_eq!(results.len(), 2);
    for r in &results {
        assert!(r.tags.contains(&"rust".to_string()));
    }
}

// F-02: recall filters by agent_id
#[test]
fn recall_filters_by_agent_id() {
    let (store, _f) = open_store();
    store.remember(make_memory("agent-one", "Memory from one", &[])).expect("one");
    store.remember(make_memory("agent-two", "Memory from two", &[])).expect("two");
    let results = store
        .recall(RecallQuery {
            agent_id: Some("agent-one".to_string()),
            limit: 10,
            ..Default::default()
        })
        .expect("recall");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].agent_id, "agent-one");
}

// F-02: recall filters by time_range
#[test]
fn recall_filters_by_time_range() {
    let (store, _f) = open_store();
    let mut old_mem = make_memory("agent-time", "Old memory", &[]);
    old_mem.created_at = Utc::now() - Duration::hours(2);
    let mut new_mem = make_memory("agent-time", "Recent memory", &[]);
    new_mem.created_at = Utc::now() - Duration::minutes(5);
    store.remember(old_mem).expect("old");
    store.remember(new_mem).expect("new");
    let results = store
        .recall(RecallQuery {
            time_range: Some((Utc::now() - Duration::hours(1), Utc::now() + Duration::hours(1))),
            limit: 10,
            ..Default::default()
        })
        .expect("recall");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].content, "Recent memory");
}

// F-02: recall with FTS5 text search
#[test]
fn recall_full_text_search() {
    let (store, _f) = open_store();
    store
        .remember(make_memory("agent-fts", "distributed systems consensus algorithm", &["cs"]))
        .expect("a");
    store
        .remember(make_memory("agent-fts", "machine learning gradient descent", &["ml"]))
        .expect("b");
    let results = store
        .recall(RecallQuery {
            text_search: Some("consensus".to_string()),
            limit: 10,
            ..Default::default()
        })
        .expect("recall");
    assert_eq!(results.len(), 1);
    assert!(results[0].content.contains("consensus"));
}

#[path = "sqlite_store_tests_advanced.rs"]
mod advanced;
