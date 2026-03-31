// IPC schema definitions aligned with actual DB (Plan 633 schema.rs + CRDT).
// Extracted from mod.rs to stay under the 250-line limit.

use super::super::state::{ApiError, ServerState};

/// IPC table DDL — local-only, not replicated via CRDT.
const IPC_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS ipc_agents (
    name        TEXT NOT NULL,
    host        TEXT NOT NULL,
    agent_type  TEXT NOT NULL DEFAULT 'claude',
    pid         INTEGER,
    metadata    TEXT,
    parent_agent TEXT,
    registered_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now')),
    last_seen   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now')),
    PRIMARY KEY (name, host)
);
CREATE TABLE IF NOT EXISTS ipc_messages (
    id TEXT PRIMARY KEY NOT NULL,
    from_agent TEXT NOT NULL DEFAULT '',
    to_agent TEXT,
    channel TEXT,
    content TEXT NOT NULL DEFAULT '',
    msg_type TEXT NOT NULL DEFAULT 'text',
    priority INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now')),
    read_at TEXT
);
CREATE TABLE IF NOT EXISTS ipc_channels (
    name TEXT PRIMARY KEY NOT NULL,
    description TEXT,
    created_by TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now'))
);
CREATE TABLE IF NOT EXISTS ipc_file_locks (
    file_path TEXT PRIMARY KEY NOT NULL,
    locked_by TEXT NOT NULL DEFAULT '',
    lock_type TEXT NOT NULL DEFAULT 'exclusive',
    acquired_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now')),
    expires_at TEXT
);
CREATE TABLE IF NOT EXISTS ipc_worktrees (
    path TEXT PRIMARY KEY NOT NULL,
    plan_id INTEGER,
    branch TEXT,
    owner_agent TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now'))
);
CREATE TABLE IF NOT EXISTS ipc_orgs (
    id TEXT PRIMARY KEY NOT NULL,
    mission TEXT NOT NULL,
    objectives TEXT NOT NULL,
    ceo_agent TEXT NOT NULL,
    budget REAL NOT NULL DEFAULT 0,
    daily_budget_tokens INTEGER NOT NULL DEFAULT 1000,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now'))
);
CREATE TABLE IF NOT EXISTS ipc_org_members (
    id TEXT PRIMARY KEY NOT NULL,
    org_id TEXT NOT NULL,
    agent TEXT NOT NULL,
    role TEXT NOT NULL,
    department TEXT,
    joined_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now')),
    UNIQUE(org_id, agent),
    FOREIGN KEY(org_id) REFERENCES ipc_orgs(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS ipc_org_services (
    id TEXT PRIMARY KEY NOT NULL,
    org_id TEXT NOT NULL,
    name TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    metadata TEXT,
    registered_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now')),
    UNIQUE(org_id, name),
    FOREIGN KEY(org_id) REFERENCES ipc_orgs(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS ipc_decisions (
    id TEXT PRIMARY KEY NOT NULL,
    org_id TEXT NOT NULL,
    decision TEXT NOT NULL,
    rationale TEXT,
    decided_by TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now')),
    FOREIGN KEY(org_id) REFERENCES ipc_orgs(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS ipc_org_telemetry (
    id TEXT PRIMARY KEY NOT NULL,
    org_id TEXT NOT NULL,
    metric TEXT NOT NULL,
    value REAL NOT NULL,
    tags TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now')),
    FOREIGN KEY(org_id) REFERENCES ipc_orgs(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS ipc_org_digests (
    id TEXT PRIMARY KEY NOT NULL,
    org_id TEXT NOT NULL,
    type TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now')),
    FOREIGN KEY(org_id) REFERENCES ipc_orgs(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_ipc_messages_channel ON ipc_messages(channel, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_ipc_messages_interorg
ON ipc_messages(channel, created_at)
WHERE channel LIKE 'inter-org:%';
CREATE INDEX IF NOT EXISTS idx_ipc_org_members_org ON ipc_org_members(org_id, agent);
CREATE INDEX IF NOT EXISTS idx_ipc_org_services_org ON ipc_org_services(org_id, status);
CREATE INDEX IF NOT EXISTS idx_ipc_decisions_org ON ipc_decisions(org_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_ipc_org_telemetry_org ON ipc_org_telemetry(org_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_ipc_org_digests_org ON ipc_org_digests(org_id, created_at DESC);
CREATE TRIGGER IF NOT EXISTS trg_ipc_messages_org_isolation
BEFORE INSERT ON ipc_messages
WHEN NEW.channel LIKE 'org:%'
  AND NEW.from_agent IS NOT NULL
  AND NEW.from_agent != ''
  AND NOT EXISTS (
      SELECT 1 FROM ipc_org_members m
      WHERE m.org_id = substr(NEW.channel, 5)
        AND m.agent = NEW.from_agent
  )
BEGIN
    SELECT RAISE(ABORT, 'org isolation violation');
END;
CREATE TRIGGER IF NOT EXISTS trg_ipc_messages_interorg_source_guard
BEFORE INSERT ON ipc_messages
WHEN NEW.channel LIKE 'inter-org:%:%'
  AND NEW.from_agent IS NOT NULL
  AND NEW.from_agent != ''
  AND NOT EXISTS (
      SELECT 1 FROM ipc_org_members m
      WHERE m.org_id = substr(NEW.channel, 11, instr(substr(NEW.channel, 11), ':') - 1)
        AND m.agent = NEW.from_agent
  )
BEGIN
    SELECT RAISE(ABORT, 'inter-org source membership required');
END;
";

pub fn ensure_ipc_schema(state: &ServerState) -> Result<(), ApiError> {
    let conn = state.get_conn()?;
    conn.execute_batch(IPC_SCHEMA).map_err(|err| {
        ApiError::internal(format!("ipc schema init failed: {err}"))
    })?;
    let _ = conn.execute_batch(
        "ALTER TABLE ipc_orgs
         ADD COLUMN daily_budget_tokens INTEGER NOT NULL DEFAULT 1000;",
    );
    // Migration: add parent_agent column to existing tables (idempotent)
    let _ = conn.execute_batch("ALTER TABLE ipc_agents ADD COLUMN parent_agent TEXT;");
    // Drop CRDT triggers on IPC tables — IPC is local-only, not replicated
    if let Err(e) = conn.execute_batch(
        "DROP TRIGGER IF EXISTS ipc_agents__crsql_itrig;
         DROP TRIGGER IF EXISTS ipc_agents__crsql_utrig;
         DROP TRIGGER IF EXISTS ipc_agents__crsql_dtrig;
         DROP TRIGGER IF EXISTS ipc_messages__crsql_itrig;
         DROP TRIGGER IF EXISTS ipc_messages__crsql_utrig;
         DROP TRIGGER IF EXISTS ipc_messages__crsql_dtrig;
         DROP TRIGGER IF EXISTS ipc_context__crsql_itrig;
         DROP TRIGGER IF EXISTS ipc_context__crsql_utrig;
         DROP TRIGGER IF EXISTS ipc_context__crsql_dtrig;
         DROP TRIGGER IF EXISTS ipc_channels__crsql_itrig;
         DROP TRIGGER IF EXISTS ipc_channels__crsql_utrig;
         DROP TRIGGER IF EXISTS ipc_channels__crsql_dtrig;
         DROP TRIGGER IF EXISTS ipc_file_locks__crsql_itrig;
         DROP TRIGGER IF EXISTS ipc_file_locks__crsql_utrig;
         DROP TRIGGER IF EXISTS ipc_file_locks__crsql_dtrig;
         DROP TRIGGER IF EXISTS ipc_worktrees__crsql_itrig;
         DROP TRIGGER IF EXISTS ipc_worktrees__crsql_utrig;
         DROP TRIGGER IF EXISTS ipc_worktrees__crsql_dtrig;",
    ) {
        tracing::warn!("ipc crdt trigger cleanup failed: {e}");
    }
    Ok(())
}
