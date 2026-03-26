// Tests for F-05: memory attestation and trust chain
use super::attestation::{add_attestation, get_attestation_chain, trust_score};
use super::types::Attestation;
use chrono::Utc;
use rusqlite::Connection;

fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().expect("in-memory db");
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA busy_timeout=5000;
         CREATE TABLE IF NOT EXISTS agent_memories (
           id TEXT PRIMARY KEY,
           agent_id TEXT NOT NULL,
           memory_type TEXT NOT NULL DEFAULT 'Fact',
           content TEXT NOT NULL DEFAULT '',
           tags TEXT NOT NULL DEFAULT '[]',
           created_at TEXT NOT NULL DEFAULT '',
           expires_at TEXT,
           access_level TEXT NOT NULL DEFAULT 'Private',
           attestations TEXT NOT NULL DEFAULT '[]',
           deleted_at TEXT,
           shared_with TEXT NOT NULL DEFAULT '[]'
         );",
    )
    .expect("schema");
    conn
}

fn insert_memory(conn: &Connection, id: &str, owner: &str) {
    conn.execute(
        "INSERT INTO agent_memories (id, agent_id, created_at) VALUES (?1, ?2, ?3)",
        rusqlite::params![id, owner, Utc::now().to_rfc3339()],
    )
    .expect("insert memory");
}

fn make_attestation(agent_id: &str, confidence: f64) -> Attestation {
    Attestation {
        attesting_agent_id: agent_id.to_string(),
        timestamp: Utc::now(),
        confidence,
    }
}

// F-05: add_attestation appends an attestation to a memory
#[test]
fn add_attestation_appends_to_memory() {
    let conn = setup_db();
    insert_memory(&conn, "mem-attest-1", "agent-owner");
    let att = make_attestation("agent-validator", 0.9);
    add_attestation(&conn, "mem-attest-1", att).expect("add attestation");
    let chain = get_attestation_chain(&conn, "mem-attest-1").expect("chain");
    assert_eq!(chain.len(), 1);
    assert_eq!(chain[0].attesting_agent_id, "agent-validator");
    assert!((chain[0].confidence - 0.9).abs() < f64::EPSILON);
}

// F-05: multiple attestations are accumulated in order
#[test]
fn add_attestation_multiple_accumulates() {
    let conn = setup_db();
    insert_memory(&conn, "mem-multi-att", "agent-owner");
    add_attestation(&conn, "mem-multi-att", make_attestation("agent-a", 0.8)).expect("a");
    add_attestation(&conn, "mem-multi-att", make_attestation("agent-b", 0.6)).expect("b");
    add_attestation(&conn, "mem-multi-att", make_attestation("agent-c", 1.0)).expect("c");
    let chain = get_attestation_chain(&conn, "mem-multi-att").expect("chain");
    assert_eq!(chain.len(), 3);
    assert_eq!(chain[0].attesting_agent_id, "agent-a");
    assert_eq!(chain[1].attesting_agent_id, "agent-b");
    assert_eq!(chain[2].attesting_agent_id, "agent-c");
}

// F-05: get_attestation_chain returns empty vec for no attestations
#[test]
fn get_attestation_chain_empty_for_fresh_memory() {
    let conn = setup_db();
    insert_memory(&conn, "mem-no-att", "agent-owner");
    let chain = get_attestation_chain(&conn, "mem-no-att").expect("chain");
    assert_eq!(chain.len(), 0);
}

// F-05: get_attestation_chain returns error for unknown memory
#[test]
fn get_attestation_chain_error_on_unknown_id() {
    let conn = setup_db();
    let result = get_attestation_chain(&conn, "nonexistent-memory-id");
    assert!(result.is_err(), "should error on unknown memory");
}

// F-05: add_attestation returns error for unknown memory
#[test]
fn add_attestation_error_on_unknown_id() {
    let conn = setup_db();
    let att = make_attestation("agent-validator", 0.7);
    let result = add_attestation(&conn, "nonexistent-id", att);
    assert!(result.is_err(), "should error on unknown memory");
}

// F-05: trust_score returns average confidence across attestations
#[test]
fn trust_score_averages_confidence() {
    let attestations = vec![
        make_attestation("agent-a", 0.8),
        make_attestation("agent-b", 0.6),
        make_attestation("agent-c", 1.0),
    ];
    let score = trust_score(&attestations);
    let expected = (0.8 + 0.6 + 1.0) / 3.0;
    assert!((score - expected).abs() < 1e-10, "score={score} expected={expected}");
}

// F-05: trust_score returns 0.0 for empty attestation list
#[test]
fn trust_score_zero_for_empty() {
    let score = trust_score(&[]);
    assert_eq!(score, 0.0);
}

// F-05: trust_score returns exact value for single attestation
#[test]
fn trust_score_single_attestation() {
    let attestations = vec![make_attestation("agent-a", 0.75)];
    let score = trust_score(&attestations);
    assert!((score - 0.75).abs() < f64::EPSILON);
}

// F-05: attestation confidence is stored with full precision
#[test]
fn attestation_confidence_precision() {
    let conn = setup_db();
    insert_memory(&conn, "mem-prec", "agent-owner");
    let att = make_attestation("agent-precise", 0.123456789);
    add_attestation(&conn, "mem-prec", att).expect("add");
    let chain = get_attestation_chain(&conn, "mem-prec").expect("chain");
    assert!((chain[0].confidence - 0.123456789).abs() < 1e-9);
}
