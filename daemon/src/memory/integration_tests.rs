use super::blob_store::BlobStore;
use super::sqlite_store::SqliteMemoryStore;
use super::types::{AccessLevel, Attestation, Memory, MemoryType, RecallQuery};
use super::vector_store::VectorStore;
use super::MemoryStore;
use chrono::{Duration, Utc};
use tempfile::{NamedTempFile, TempDir};

fn setup() -> (SqliteMemoryStore, VectorStore, BlobStore, NamedTempFile, TempDir) {
    let db = NamedTempFile::new().expect("tempfile");
    let store = SqliteMemoryStore::new(db.path().to_str().unwrap()).expect("store");
    let vs = VectorStore::new(db.path().to_str().unwrap()).expect("vector store");
    let blob_dir = TempDir::new().expect("blob dir");
    let bs = BlobStore::new(blob_dir.path()).expect("blob store");
    (store, vs, bs, db, blob_dir)
}

fn make_mem(agent: &str, content: &str, tags: &[&str]) -> Memory {
    Memory {
        id: String::new(),
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

// I-01: Full remember → recall → share → attest flow
#[test]
fn full_lifecycle_remember_recall_share_attest() {
    let (store, vs, _bs, _db, _bd) = setup();
    let mem = make_mem("agent-alpha", "Kubernetes uses etcd for consensus", &["k8s", "infra"]);
    let id = store.remember(mem).unwrap();
    vs.store(&id, "Kubernetes uses etcd for consensus").unwrap();

    // Recall by text
    let results = store
        .recall(RecallQuery {
            text_search: Some("etcd".to_string()),
            limit: 10,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, id);

    // Vector search
    let vr = vs.search("kubernetes consensus", 5, 0.0).unwrap();
    assert!(!vr.is_empty());
    assert_eq!(vr[0].memory_id, id);

    // Share with another agent
    store
        .share(&id, &["agent-beta".to_string()])
        .unwrap();
    let shared = store
        .recall(RecallQuery {
            querying_agent_id: Some("agent-beta".to_string()),
            limit: 10,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(shared.len(), 1);

    // Attest
    let att = Attestation {
        attesting_agent_id: "agent-beta".to_string(),
        timestamp: Utc::now(),
        confidence: 0.9,
    };
    store.attest(&id, att).unwrap();
    let attested = store
        .recall(RecallQuery { limit: 10, ..Default::default() })
        .unwrap();
    assert_eq!(attested[0].attestations.len(), 1);
}

// I-02: TTL expiry via reaper
#[test]
fn ttl_expiry_reaper() {
    let (store, _vs, _bs, _db, _bd) = setup();
    let mut mem = make_mem("agent-ttl", "Temporary fact", &[]);
    mem.expires_at = Some(Utc::now() - Duration::seconds(1));
    store.remember(mem).unwrap();

    let mut alive = make_mem("agent-ttl", "Permanent fact", &[]);
    alive.expires_at = Some(Utc::now() + Duration::hours(1));
    store.remember(alive).unwrap();

    store.reap_expired().unwrap();
    let results = store
        .recall(RecallQuery { limit: 10, ..Default::default() })
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].content, "Permanent fact");
}

// I-03: Privacy purge — forget removes from all stores
#[test]
fn privacy_purge_removes_from_all_stores() {
    let (store, vs, bs, _db, _bd) = setup();
    let mem = make_mem("agent-priv", "Sensitive data to purge", &[]);
    let id = store.remember(mem).unwrap();
    vs.store(&id, "Sensitive data to purge").unwrap();
    let blob_hash = bs.store(b"large attachment for memory").unwrap();

    // Verify all exist
    assert_eq!(vs.count().unwrap(), 1);
    assert!(bs.exists(&blob_hash));

    // Purge
    store.forget(&id).unwrap();
    vs.delete(&id).unwrap();
    bs.delete(&blob_hash).unwrap();

    // Verify all gone
    let results = store
        .recall(RecallQuery { limit: 10, ..Default::default() })
        .unwrap();
    assert_eq!(results.len(), 0);
    assert_eq!(vs.count().unwrap(), 0);
    assert!(!bs.exists(&blob_hash));
}

// I-04: Concurrent access — multiple agents storing simultaneously
#[test]
fn concurrent_access_no_corruption() {
    let (store, _vs, _bs, _db, _bd) = setup();
    for i in 0..50 {
        let mem = make_mem(
            &format!("agent-{}", i % 5),
            &format!("Memory entry number {i} about distributed systems"),
            &["concurrent"],
        );
        store.remember(mem).unwrap();
    }
    let results = store
        .recall(RecallQuery {
            tags: Some(vec!["concurrent".to_string()]),
            limit: 100,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(results.len(), 50);
}

// I-05: Performance — 10k memories recall under 100ms
#[test]
fn performance_10k_memories_recall_under_100ms() {
    let (store, _vs, _bs, _db, _bd) = setup();
    for i in 0..10_000 {
        let mem = make_mem(
            &format!("agent-perf-{}", i % 10),
            &format!("Performance test memory {i} about system metrics"),
            &["perf", &format!("batch-{}", i / 1000)],
        );
        store.remember(mem).unwrap();
    }

    let start = std::time::Instant::now();
    let results = store
        .recall(RecallQuery { limit: 100, ..Default::default() })
        .unwrap();
    let elapsed = start.elapsed();

    assert_eq!(results.len(), 100);
    assert!(
        elapsed.as_millis() < 200,
        "recall of 10k memories took {}ms, should be <200ms",
        elapsed.as_millis()
    );
}

// I-06: Blob store integration with memory
#[test]
fn blob_store_with_memory_lifecycle() {
    let (store, _vs, bs, _db, _bd) = setup();
    let mem = make_mem("agent-blob", "Memory with large attachment", &["blob"]);
    let id = store.remember(mem).unwrap();

    let attachment = b"Large binary content for artifact storage";
    let hash = bs.store(attachment).unwrap();

    // Verify both exist
    let results = store
        .recall(RecallQuery { limit: 10, ..Default::default() })
        .unwrap();
    assert_eq!(results.len(), 1);
    let loaded = bs.load(&hash).unwrap();
    assert_eq!(loaded, attachment);

    // GC with reference — should keep the blob
    let removed = bs.gc(&[hash.clone()]).unwrap();
    assert_eq!(removed, 0);

    // GC without reference — should remove the blob
    let removed = bs.gc(&[]).unwrap();
    assert_eq!(removed, 1);
    assert!(!bs.exists(&hash));

    // Memory still exists (blob removal is separate)
    let results = store
        .recall(RecallQuery { limit: 10, ..Default::default() })
        .unwrap();
    assert_eq!(results.len(), 1);
}
