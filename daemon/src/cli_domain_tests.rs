// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Tests for cli_domain: list and map subcommands.
// Uses a real in-memory SQLite DB to verify behavior without mocking internals.

use rusqlite::Connection;

fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().expect("in-memory db");
    conn.execute_batch(
        "CREATE TABLE domain_skill_map (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            domain      TEXT    NOT NULL,
            skill_name  TEXT    NOT NULL,
            description TEXT,
            created_at  DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(domain, skill_name)
        );
        INSERT INTO domain_skill_map (domain, skill_name, description) VALUES
            ('healthcare', 'research', 'Medical research and clinical analysis'),
            ('deploy',     'release',  'Deployment and release management'),
            ('design',     'prepare',  'Design preparation and setup');",
    )
    .expect("setup");
    conn
}

#[test]
fn test_list() {
    let conn = setup_db();
    let rows = super::query_domain_list(&conn).expect("query_domain_list");
    assert_eq!(rows.len(), 3, "expected 3 seeded rows, got {}", rows.len());
    let domains: Vec<&str> = rows.iter().map(|r| r.domain.as_str()).collect();
    assert!(domains.contains(&"healthcare"), "healthcare missing");
    assert!(domains.contains(&"deploy"), "deploy missing");
    assert!(domains.contains(&"design"), "design missing");
}

#[test]
fn test_map_valid() {
    let conn = setup_db();
    // skill_dir existence is checked at CLI layer, not DB layer; pass None to skip FS check
    let result = super::insert_domain_map(&conn, "analytics", "research", Some("Data analytics"));
    assert!(result.is_ok(), "insert should succeed: {:?}", result);
    let rows = super::query_domain_list(&conn).expect("query after insert");
    assert_eq!(rows.len(), 4, "expected 4 rows after insert");
    let found = rows
        .iter()
        .find(|r| r.domain == "analytics" && r.skill_name == "research");
    assert!(found.is_some(), "new row not found");
}

#[test]
fn test_map_invalid_skill() {
    // Skill directory check: use a path guaranteed not to exist
    let nonexistent = "/tmp/__cvg_test_skill_that_does_not_exist_xyz123__";
    let err = super::validate_skill_dir(nonexistent);
    assert!(err.is_err(), "should error for nonexistent skill dir");
    let msg = err.unwrap_err();
    assert!(
        msg.contains("not found") || msg.contains("does not exist"),
        "error message should mention skill not found: {msg}"
    );
}
