use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConflictInfo {
    pub pattern: String,
    pub agents: Vec<String>,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

/// Detect conflicts by finding file patterns claimed by more than one distinct agent.
pub fn detect_conflicts(conn: &Connection) -> rusqlite::Result<Vec<ConflictInfo>> {
    let mut stmt = conn.prepare(
        "SELECT file_pattern, GROUP_CONCAT(DISTINCT agent) as agents, COUNT(DISTINCT agent) as cnt
         FROM ipc_file_locks
         GROUP BY file_pattern
         HAVING cnt > 1
         ORDER BY cnt DESC",
    )?;

    let rows = stmt.query_map([], |row| {
        let pattern: String = row.get(0)?;
        let agents_str: String = row.get(1)?;
        let count: i64 = row.get(2)?;
        let agents: Vec<String> = agents_str.split(',').map(|s| s.to_string()).collect();
        let risk_level = match count {
            2 => RiskLevel::Medium,
            _ => RiskLevel::High,
        };
        Ok(ConflictInfo {
            pattern,
            agents,
            risk_level,
        })
    })?;

    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE ipc_file_locks (
                file_pattern TEXT NOT NULL,
                agent TEXT NOT NULL,
                host TEXT NOT NULL,
                pid INTEGER NOT NULL,
                locked_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (file_pattern, agent, host)
            );",
        )
        .unwrap();
        conn
    }

    #[test]
    fn two_agents_same_pattern_detected() {
        let conn = setup_db();
        conn.execute(
            "INSERT INTO ipc_file_locks(file_pattern, agent, host, pid)
             VALUES ('src/*.rs', 'agent-a', 'host1', 1000)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO ipc_file_locks(file_pattern, agent, host, pid)
             VALUES ('src/*.rs', 'agent-b', 'host2', 2000)",
            [],
        )
        .unwrap();

        let conflicts = detect_conflicts(&conn).unwrap();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].pattern, "src/*.rs");
        assert_eq!(conflicts[0].agents.len(), 2);
        assert!(conflicts[0].agents.contains(&"agent-a".to_string()));
        assert!(conflicts[0].agents.contains(&"agent-b".to_string()));
        assert_eq!(conflicts[0].risk_level, RiskLevel::Medium);
    }

    #[test]
    fn no_overlap_returns_empty() {
        let conn = setup_db();
        conn.execute(
            "INSERT INTO ipc_file_locks(file_pattern, agent, host, pid)
             VALUES ('src/*.rs', 'agent-a', 'host1', 1000)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO ipc_file_locks(file_pattern, agent, host, pid)
             VALUES ('tests/*.rs', 'agent-b', 'host2', 2000)",
            [],
        )
        .unwrap();

        let conflicts = detect_conflicts(&conn).unwrap();
        assert!(conflicts.is_empty());
    }

    #[test]
    fn three_agents_returns_high_risk() {
        let conn = setup_db();
        for (agent, host) in [("a", "h1"), ("b", "h2"), ("c", "h3")] {
            conn.execute(
                "INSERT INTO ipc_file_locks(file_pattern, agent, host, pid)
                 VALUES ('lib/*.rs', ?1, ?2, 100)",
                params![agent, host],
            )
            .unwrap();
        }
        let conflicts = detect_conflicts(&conn).unwrap();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].risk_level, RiskLevel::High);
    }
}
