#!/usr/bin/env bash
set -euo pipefail
# migrate-ipc-intelligence.sh — Add 6 IPC Intelligence Layer tables
# Idempotent: safe to run multiple times. All tables CRR-registered for CRDT sync.

DB_PATH="${1:-${HOME}/.claude/data/dashboard.db}"

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'
log_ok()  { echo -e "${GREEN}[OK]${NC} $1"; }
log_err() { echo -e "${RED}[ERR]${NC} $1"; }

if [[ ! -f "$DB_PATH" ]]; then
  log_err "Database not found: $DB_PATH"
  exit 1
fi

table_exists() {
  sqlite3 "$DB_PATH" "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='$1';" | grep -q '^1$'
}

sqlite3 "$DB_PATH" <<'SQL'
CREATE TABLE IF NOT EXISTS ipc_auth_tokens (
  id INTEGER PRIMARY KEY NOT NULL,
  service TEXT NOT NULL DEFAULT '',
  encrypted_token BLOB NOT NULL,
  nonce BLOB NOT NULL,
  host TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE (service, host)
);

CREATE TABLE IF NOT EXISTS ipc_model_registry (
  id INTEGER PRIMARY KEY NOT NULL,
  host TEXT NOT NULL DEFAULT '',
  provider TEXT NOT NULL DEFAULT '',
  model TEXT NOT NULL DEFAULT '',
  size_gb REAL NOT NULL DEFAULT 0.0,
  quantization TEXT NOT NULL DEFAULT '',
  last_seen TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE (host, provider, model)
);

CREATE TABLE IF NOT EXISTS ipc_node_capabilities (
  host TEXT PRIMARY KEY NOT NULL,
  provider TEXT NOT NULL DEFAULT '',
  models TEXT NOT NULL DEFAULT '[]',
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS ipc_subscriptions (
  name TEXT PRIMARY KEY NOT NULL,
  provider TEXT NOT NULL DEFAULT '',
  plan TEXT NOT NULL DEFAULT '',
  budget_usd REAL NOT NULL DEFAULT 0.0,
  reset_day INTEGER NOT NULL DEFAULT 1,
  models TEXT NOT NULL DEFAULT '[]'
);

CREATE TABLE IF NOT EXISTS ipc_budget_log (
  id INTEGER PRIMARY KEY NOT NULL,
  subscription TEXT NOT NULL DEFAULT '',
  date TEXT NOT NULL DEFAULT (date('now')),
  tokens_in INTEGER NOT NULL DEFAULT 0,
  tokens_out INTEGER NOT NULL DEFAULT 0,
  estimated_cost_usd REAL NOT NULL DEFAULT 0.0,
  model TEXT NOT NULL DEFAULT '',
  task_ref TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS ipc_agent_skills (
  id INTEGER PRIMARY KEY NOT NULL,
  agent TEXT NOT NULL DEFAULT '',
  host TEXT NOT NULL DEFAULT '',
  skill TEXT NOT NULL DEFAULT '',
  confidence REAL NOT NULL DEFAULT 0.5,
  last_used TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE (agent, host, skill)
);
SQL

log_ok "IPC Intelligence tables created/verified"

TABLES=(ipc_auth_tokens ipc_model_registry ipc_node_capabilities ipc_subscriptions ipc_budget_log ipc_agent_skills)
for tbl in "${TABLES[@]}"; do
  if table_exists "$tbl"; then
    log_ok "Table $tbl exists"
  else
    log_err "Table $tbl missing!"
    exit 1
  fi
done

# CRR registration for CRDT sync (6 explicit calls — one per table)
# At runtime, Rust mark_required_tables() handles this automatically.
# These calls support standalone migration when crsqlite extension is loaded.
if sqlite3 "$DB_PATH" "SELECT 1 FROM pragma_function_list WHERE name LIKE 'crsql_as_%';" 2>/dev/null | grep -q '1'; then
  sqlite3 "$DB_PATH" "SELECT crsql_as_crr('ipc_auth_tokens');" 2>/dev/null || true
  sqlite3 "$DB_PATH" "SELECT crsql_as_crr('ipc_model_registry');" 2>/dev/null || true
  sqlite3 "$DB_PATH" "SELECT crsql_as_crr('ipc_node_capabilities');" 2>/dev/null || true
  sqlite3 "$DB_PATH" "SELECT crsql_as_crr('ipc_subscriptions');" 2>/dev/null || true
  sqlite3 "$DB_PATH" "SELECT crsql_as_crr('ipc_budget_log');" 2>/dev/null || true
  sqlite3 "$DB_PATH" "SELECT crsql_as_crr('ipc_agent_skills');" 2>/dev/null || true
  log_ok "CRR registration complete"
else
  log_ok "crsqlite not loaded — CRR deferred to Rust runtime"
fi

log_ok "All 6 IPC Intelligence tables ready"
