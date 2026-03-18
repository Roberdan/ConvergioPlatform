use rusqlite::Connection;

pub const IPC_TABLES: [&str; 6] = [
    "ipc_agents",
    "ipc_messages",
    "ipc_channels",
    "ipc_shared_context",
    "ipc_file_locks",
    "ipc_worktrees",
];

pub fn ensure_ipc_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS ipc_agents (
            name        TEXT NOT NULL,
            host        TEXT NOT NULL,
            agent_type  TEXT NOT NULL DEFAULT 'claude',
            pid         INTEGER,
            metadata    TEXT,
            registered_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now')),
            last_seen   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now')),
            PRIMARY KEY (name, host)
        );

        CREATE TABLE IF NOT EXISTS ipc_messages (
            id          TEXT PRIMARY KEY,
            from_agent  TEXT NOT NULL,
            to_agent    TEXT,
            channel     TEXT,
            content     TEXT NOT NULL,
            msg_type    TEXT NOT NULL DEFAULT 'text',
            priority    INTEGER NOT NULL DEFAULT 0,
            created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now')),
            read_at     TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_ipc_messages_to_agent
            ON ipc_messages(to_agent);
        CREATE INDEX IF NOT EXISTS idx_ipc_messages_channel
            ON ipc_messages(channel);
        CREATE INDEX IF NOT EXISTS idx_ipc_messages_created_at
            ON ipc_messages(created_at);

        CREATE TABLE IF NOT EXISTS ipc_channels (
            name        TEXT PRIMARY KEY,
            description TEXT,
            created_by  TEXT NOT NULL,
            created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now'))
        );

        CREATE TABLE IF NOT EXISTS ipc_shared_context (
            key         TEXT PRIMARY KEY,
            value       TEXT NOT NULL,
            version     INTEGER NOT NULL DEFAULT 1,
            set_by      TEXT NOT NULL,
            updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now'))
        );

        CREATE TABLE IF NOT EXISTS ipc_file_locks (
            file_path   TEXT PRIMARY KEY,
            locked_by   TEXT NOT NULL,
            lock_type   TEXT NOT NULL DEFAULT 'exclusive',
            acquired_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now')),
            expires_at  TEXT
        );

        CREATE TABLE IF NOT EXISTS ipc_worktrees (
            path        TEXT PRIMARY KEY,
            plan_id     INTEGER,
            branch      TEXT,
            owner_agent TEXT,
            status      TEXT NOT NULL DEFAULT 'active',
            created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now'))
        );
        ",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensure_ipc_schema_creates_all_tables() {
        let conn = Connection::open_in_memory().unwrap();
        ensure_ipc_schema(&conn).unwrap();

        for table in &IPC_TABLES {
            let count: i64 = conn
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(count, 0, "table {table} should exist and be empty");
        }
    }

    #[test]
    fn test_ensure_ipc_schema_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        ensure_ipc_schema(&conn).unwrap();
        ensure_ipc_schema(&conn).unwrap(); // second call must not fail
    }

    #[test]
    fn test_agent_pk_is_name_host() {
        let conn = Connection::open_in_memory().unwrap();
        ensure_ipc_schema(&conn).unwrap();

        conn.execute(
            "INSERT INTO ipc_agents (name, host, agent_type) VALUES (?1, ?2, ?3)",
            ["planner", "mac-worker-2", "claude"],
        )
        .unwrap();

        // Same name, different host => OK
        conn.execute(
            "INSERT INTO ipc_agents (name, host, agent_type) VALUES (?1, ?2, ?3)",
            ["planner", "linux-worker", "claude"],
        )
        .unwrap();

        // Same name+host => conflict
        let err = conn.execute(
            "INSERT INTO ipc_agents (name, host, agent_type) VALUES (?1, ?2, ?3)",
            ["planner", "mac-worker-2", "copilot"],
        );
        assert!(err.is_err());
    }
}
