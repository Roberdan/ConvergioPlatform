// Tests for F-04: memory sharing and access control
use super::sharing::{check_access, grant_access, MemoryAccessGrant};
use super::types::AccessLevel;
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
         );
         CREATE TABLE IF NOT EXISTS memory_access_grants (
           memory_id TEXT NOT NULL,
           granted_to TEXT NOT NULL,
           granted_at TEXT NOT NULL,
           PRIMARY KEY (memory_id, granted_to)
         );",
    )
    .expect("schema");
    conn
}

fn insert_memory(conn: &Connection, id: &str, owner: &str, access: &str) {
    conn.execute(
        "INSERT INTO agent_memories (id, agent_id, access_level, created_at)
         VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![id, owner, access, Utc::now().to_rfc3339()],
    )
    .expect("insert memory");
}

// F-04: Public memory is accessible by any agent
#[test]
fn public_memory_accessible_to_all() {
    let conn = setup_db();
    insert_memory(&conn, "mem-pub-1", "agent-owner", "Public");
    assert!(check_access(&conn, "mem-pub-1", "agent-stranger").expect("check"));
}

// F-04: Private memory is only accessible by the owner
#[test]
fn private_memory_only_accessible_to_owner() {
    let conn = setup_db();
    insert_memory(&conn, "mem-priv-1", "agent-owner", "Private");
    assert!(check_access(&conn, "mem-priv-1", "agent-owner").expect("check owner"));
    assert!(!check_access(&conn, "mem-priv-1", "agent-other").expect("check other"));
}

// F-04: Shared memory is accessible to explicitly granted agents
#[test]
fn shared_memory_accessible_to_granted_agents() {
    let conn = setup_db();
    insert_memory(&conn, "mem-shared-1", "agent-owner", "Shared");
    grant_access(&conn, "mem-shared-1", &["agent-grantee".to_string()]).expect("grant");
    assert!(check_access(&conn, "mem-shared-1", "agent-grantee").expect("grantee access"));
}

// F-04: Shared memory is NOT accessible to non-granted agents
#[test]
fn shared_memory_not_accessible_to_non_granted() {
    let conn = setup_db();
    insert_memory(&conn, "mem-shared-2", "agent-owner", "Shared");
    grant_access(&conn, "mem-shared-2", &["agent-grantee".to_string()]).expect("grant");
    assert!(!check_access(&conn, "mem-shared-2", "agent-outsider").expect("outsider access"));
}

// F-04: Shared memory owner always has access
#[test]
fn shared_memory_owner_always_has_access() {
    let conn = setup_db();
    insert_memory(&conn, "mem-shared-3", "agent-owner", "Shared");
    // No grants yet — owner still has access
    assert!(check_access(&conn, "mem-shared-3", "agent-owner").expect("owner access"));
}

// F-04: grant_access is idempotent — duplicate grants don't error
#[test]
fn grant_access_is_idempotent() {
    let conn = setup_db();
    insert_memory(&conn, "mem-idem-1", "agent-owner", "Shared");
    grant_access(&conn, "mem-idem-1", &["agent-a".to_string()]).expect("first grant");
    grant_access(&conn, "mem-idem-1", &["agent-a".to_string()]).expect("duplicate grant");
    assert!(check_access(&conn, "mem-idem-1", "agent-a").expect("access after duplicate grant"));
}

// F-04: grant_access stores correct MemoryAccessGrant fields
#[test]
fn grant_access_stores_grant_record() {
    let conn = setup_db();
    insert_memory(&conn, "mem-grant-1", "agent-owner", "Shared");
    grant_access(&conn, "mem-grant-1", &["agent-recipient".to_string()]).expect("grant");
    let grant: MemoryAccessGrant = conn
        .query_row(
            "SELECT memory_id, granted_to, granted_at FROM memory_access_grants
             WHERE memory_id = 'mem-grant-1' AND granted_to = 'agent-recipient'",
            [],
            |r| {
                Ok(MemoryAccessGrant {
                    memory_id: r.get(0)?,
                    granted_to: r.get(1)?,
                    granted_at: r.get(2)?,
                })
            },
        )
        .expect("query grant");
    assert_eq!(grant.memory_id, "mem-grant-1");
    assert_eq!(grant.granted_to, "agent-recipient");
    assert!(!grant.granted_at.is_empty());
}

// F-04: grant_access handles multiple agents in one call
#[test]
fn grant_access_multiple_agents() {
    let conn = setup_db();
    insert_memory(&conn, "mem-multi-1", "agent-owner", "Shared");
    grant_access(
        &conn,
        "mem-multi-1",
        &["agent-a".to_string(), "agent-b".to_string(), "agent-c".to_string()],
    )
    .expect("grant multiple");
    assert!(check_access(&conn, "mem-multi-1", "agent-a").expect("a"));
    assert!(check_access(&conn, "mem-multi-1", "agent-b").expect("b"));
    assert!(check_access(&conn, "mem-multi-1", "agent-c").expect("c"));
    assert!(!check_access(&conn, "mem-multi-1", "agent-d").expect("d"));
}

// F-04: AccessLevel::Shared used correctly
#[test]
fn access_level_shared_variant_is_correct() {
    assert_eq!(format!("{:?}", AccessLevel::Shared), "Shared");
    assert_eq!(format!("{:?}", AccessLevel::Private), "Private");
    assert_eq!(format!("{:?}", AccessLevel::Public), "Public");
}
