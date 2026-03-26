<<<<<<< HEAD
use super::make_memory;
use super::open_store;
use super::super::super::types::{Attestation, MemoryError, RecallQuery};
use super::super::super::MemoryStore;
use chrono::{Duration, Utc};

// F-03: forget() soft-deletes — memory no longer returned by recall
#[test]
fn forget_soft_deletes() {
    let (store, _f) = open_store();
    let mem = make_memory("agent-forget", "Temporary memory", &[]);
    let id = store.remember(mem).expect("remember");
    store.forget(&id).expect("forget");
    let results = store
        .recall(RecallQuery { limit: 10, ..Default::default() })
        .expect("recall");
    assert_eq!(results.len(), 0, "soft-deleted memory must not appear in recall");
}

// F-03: forget() on unknown id returns MemoryError::NotFound
#[test]
fn forget_unknown_id_returns_not_found() {
    let (store, _f) = open_store();
    let err = store.forget("00000000-0000-0000-0000-000000000000").unwrap_err();
    assert!(matches!(err, MemoryError::NotFound(_)));
}

// F-06: reap_expired() removes memories past expires_at
#[test]
fn reap_expired_removes_past_memories() {
    let (store, _f) = open_store();
    let mut expired = make_memory("agent-reap", "This expires", &[]);
    expired.expires_at = Some(Utc::now() - Duration::seconds(1));
    let mut alive = make_memory("agent-reap", "This stays", &[]);
    alive.expires_at = Some(Utc::now() + Duration::hours(1));
    store.remember(expired).expect("expired");
    store.remember(alive).expect("alive");
    store.reap_expired().expect("reap");
    let results = store
        .recall(RecallQuery { limit: 10, ..Default::default() })
        .expect("recall");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].content, "This stays");
}

// F-06: attest() appends attestation to memory
#[test]
fn attest_appends_attestation() {
    let (store, _f) = open_store();
    let mem = make_memory("agent-attest", "Memory to attest", &[]);
    let id = store.remember(mem).expect("remember");
    let attestation = Attestation {
        attesting_agent_id: "agent-validator".to_string(),
        timestamp: Utc::now(),
        confidence: 0.95,
    };
    store.attest(&id, attestation).expect("attest");
    let results = store
        .recall(RecallQuery { limit: 10, ..Default::default() })
        .expect("recall");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].attestations.len(), 1);
    assert_eq!(results[0].attestations[0].attesting_agent_id, "agent-validator");
}

// F-06: share() records shared access (smoke test)
#[test]
fn share_does_not_error() {
    let (store, _f) = open_store();
    let mem = make_memory("agent-share", "Shareable memory", &[]);
    let id = store.remember(mem).expect("remember");
    store
        .share(&id, &["agent-a".to_string(), "agent-b".to_string()])
        .expect("share");
}

// recall respects limit parameter
#[test]
fn recall_respects_limit() {
    let (store, _f) = open_store();
    for i in 0..10 {
        store
            .remember(make_memory("agent-limit", &format!("Memory {i}"), &[]))
            .expect("remember");
    }
    let results = store
        .recall(RecallQuery { limit: 3, ..Default::default() })
        .expect("recall");
    assert_eq!(results.len(), 3);
}
||||||| parent of 703864c (feat(daemon): Plan 715 T4A-03 — memory sharing and attestation)
=======
use super::make_memory;
use super::open_store;
use super::super::super::types::{Attestation, AccessLevel, MemoryError, RecallQuery};
use super::super::super::MemoryStore;
use chrono::{Duration, Utc};

// F-03: forget() soft-deletes — memory no longer returned by recall
#[test]
fn forget_soft_deletes() {
    let (store, _f) = open_store();
    let mem = make_memory("agent-forget", "Temporary memory", &[]);
    let id = store.remember(mem).expect("remember");
    store.forget(&id).expect("forget");
    let results = store
        .recall(RecallQuery { limit: 10, ..Default::default() })
        .expect("recall");
    assert_eq!(results.len(), 0, "soft-deleted memory must not appear in recall");
}

// F-03: forget() on unknown id returns MemoryError::NotFound
#[test]
fn forget_unknown_id_returns_not_found() {
    let (store, _f) = open_store();
    let err = store.forget("00000000-0000-0000-0000-000000000000").unwrap_err();
    assert!(matches!(err, MemoryError::NotFound(_)));
}

// F-06: reap_expired() removes memories past expires_at
#[test]
fn reap_expired_removes_past_memories() {
    let (store, _f) = open_store();
    let mut expired = make_memory("agent-reap", "This expires", &[]);
    expired.expires_at = Some(Utc::now() - Duration::seconds(1));
    let mut alive = make_memory("agent-reap", "This stays", &[]);
    alive.expires_at = Some(Utc::now() + Duration::hours(1));
    store.remember(expired).expect("expired");
    store.remember(alive).expect("alive");
    store.reap_expired().expect("reap");
    let results = store
        .recall(RecallQuery { limit: 10, ..Default::default() })
        .expect("recall");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].content, "This stays");
}

// F-05: attest() appends attestation to memory
#[test]
fn attest_appends_attestation() {
    let (store, _f) = open_store();
    let mem = make_memory("agent-attest", "Memory to attest", &[]);
    let id = store.remember(mem).expect("remember");
    let attestation = Attestation {
        attesting_agent_id: "agent-validator".to_string(),
        timestamp: Utc::now(),
        confidence: 0.95,
    };
    store.attest(&id, attestation).expect("attest");
    let results = store
        .recall(RecallQuery { limit: 10, ..Default::default() })
        .expect("recall");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].attestations.len(), 1);
    assert_eq!(results[0].attestations[0].attesting_agent_id, "agent-validator");
}

// F-04: share() grants access — recipient can recall via querying_agent_id
#[test]
fn share_grants_access_via_querying_agent() {
    let (store, _f) = open_store();
    let mut mem = make_memory("agent-owner", "Shared content", &[]);
    mem.access_level = AccessLevel::Private;
    let id = store.remember(mem).expect("remember");

    // Before sharing: recipient cannot recall
    let before = store
        .recall(RecallQuery {
            querying_agent_id: Some("agent-recipient".to_string()),
            limit: 10,
            ..Default::default()
        })
        .expect("before recall");
    assert_eq!(before.len(), 0, "recipient must not see private memory before share");

    // After sharing: recipient can recall
    store.share(&id, &["agent-recipient".to_string()]).expect("share");
    let after = store
        .recall(RecallQuery {
            querying_agent_id: Some("agent-recipient".to_string()),
            limit: 10,
            ..Default::default()
        })
        .expect("after recall");
    assert_eq!(after.len(), 1, "recipient must see memory after share");
    assert_eq!(after[0].id, id);
}

// F-04: recall without querying_agent_id returns all (admin mode)
#[test]
fn recall_without_querying_agent_returns_all() {
    let (store, _f) = open_store();
    store.remember(make_memory("agent-a", "Private A", &[])).expect("a");
    store.remember(make_memory("agent-b", "Private B", &[])).expect("b");
    let results = store
        .recall(RecallQuery { limit: 10, ..Default::default() })
        .expect("recall");
    assert_eq!(results.len(), 2);
}

// F-04: Public memories visible to all agents in querying_agent_id mode
#[test]
fn public_memory_visible_to_any_querying_agent() {
    let (store, _f) = open_store();
    let mut mem = make_memory("agent-pub", "Public knowledge", &[]);
    mem.access_level = AccessLevel::Public;
    store.remember(mem).expect("remember");
    let results = store
        .recall(RecallQuery {
            querying_agent_id: Some("agent-random".to_string()),
            limit: 10,
            ..Default::default()
        })
        .expect("recall");
    assert_eq!(results.len(), 1);
}

// recall respects limit parameter
#[test]
fn recall_respects_limit() {
    let (store, _f) = open_store();
    for i in 0..10 {
        store
            .remember(make_memory("agent-limit", &format!("Memory {i}"), &[]))
            .expect("remember");
    }
    let results = store
        .recall(RecallQuery { limit: 3, ..Default::default() })
        .expect("recall");
    assert_eq!(results.len(), 3);
}
>>>>>>> 703864c (feat(daemon): Plan 715 T4A-03 — memory sharing and attestation)
