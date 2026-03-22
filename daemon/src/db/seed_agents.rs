// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Seed the agent_catalog table with default agents from the platform.

use rusqlite::Connection;

/// Default agent definitions shipped with the platform.
/// (name, category, description, model)
const DEFAULT_AGENTS: &[(&str, &str, &str, &str)] = &[
    (
        "Convergio",
        "technical",
        "Platform control plane expert",
        "claude-sonnet-4.6",
    ),
    (
        "ConvergioLLM",
        "technical",
        "Local LLM infrastructure",
        "claude-sonnet-4.6",
    ),
    (
        "execute",
        "core",
        "Task execution with TDD",
        "gpt-5.3-codex",
    ),
    (
        "validate",
        "core",
        "Thor quality validation",
        "claude-opus-4.6",
    ),
    (
        "planner",
        "core",
        "Plan creation and orchestration",
        "claude-opus-4.6-1m",
    ),
    (
        "plan-reviewer",
        "core",
        "Plan review",
        "claude-sonnet-4.6",
    ),
    (
        "adversarial-debugger",
        "technical",
        "Deep debugging",
        "claude-opus-4.6",
    ),
    (
        "context-optimizer",
        "core",
        "Context optimization",
        "claude-opus-4.6",
    ),
    (
        "code-reviewer",
        "technical",
        "Code review",
        "claude-haiku-4.5",
    ),
    (
        "check",
        "core",
        "Quick checks",
        "gpt-5.1-codex-mini",
    ),
];

/// Insert default agents into agent_catalog using INSERT OR IGNORE.
///
/// Returns the number of rows actually inserted (ignores duplicates).
pub fn seed_default_agents(conn: &Connection) -> Result<usize, String> {
    let mut inserted = 0usize;
    for (name, category, description, model) in DEFAULT_AGENTS {
        let changes = conn
            .execute(
                "INSERT OR IGNORE INTO agent_catalog (name, category, description, model) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![name, category, description, model],
            )
            .map_err(|e| format!("seed agent {name}: {e}"))?;
        inserted += changes;
    }
    Ok(inserted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS agent_catalog (
                name TEXT PRIMARY KEY,
                category TEXT,
                description TEXT,
                model TEXT,
                tools TEXT,
                skills TEXT,
                source_repo TEXT,
                constitution_version TEXT,
                version TEXT,
                created_at DATETIME DEFAULT (datetime('now')),
                updated_at DATETIME DEFAULT (datetime('now'))
            )",
        )
        .expect("create agent_catalog");
        conn
    }

    #[test]
    fn seed_inserts_all_default_agents() {
        let conn = setup_db();
        let count = seed_default_agents(&conn).expect("seed");
        assert_eq!(count, 10, "should insert exactly 10 default agents");
    }

    #[test]
    fn seed_is_idempotent() {
        let conn = setup_db();
        let first = seed_default_agents(&conn).expect("first seed");
        assert_eq!(first, 10);
        let second = seed_default_agents(&conn).expect("second seed");
        assert_eq!(second, 0, "re-seeding must insert zero (INSERT OR IGNORE)");
    }

    #[test]
    fn seed_populates_correct_fields() {
        let conn = setup_db();
        seed_default_agents(&conn).expect("seed");

        let (cat, desc, model): (String, String, String) = conn
            .query_row(
                "SELECT category, description, model FROM agent_catalog WHERE name = 'Convergio'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("query Convergio");
        assert_eq!(cat, "technical");
        assert_eq!(desc, "Platform control plane expert");
        assert_eq!(model, "claude-sonnet-4.6");
    }

    #[test]
    fn seed_includes_all_categories() {
        let conn = setup_db();
        seed_default_agents(&conn).expect("seed");

        let categories: Vec<String> = {
            let mut stmt = conn
                .prepare("SELECT DISTINCT category FROM agent_catalog ORDER BY category")
                .expect("prepare");
            stmt.query_map([], |row| row.get(0))
                .expect("query")
                .collect::<Result<Vec<_>, _>>()
                .expect("collect")
        };
        assert!(categories.contains(&"core".to_string()));
        assert!(categories.contains(&"technical".to_string()));
    }
}
