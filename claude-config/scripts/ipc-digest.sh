#!/usr/bin/env bash
# ipc-digest.sh — Compact JSON IPC status (core + intelligence layer)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IPC_BIN="${IPC_BIN:-claude-core}"
DB_PATH="${HOME}/.claude/data/dashboard.db"

# Check binary
if ! command -v "$IPC_BIN" &>/dev/null; then
  local_bin="$HOME/.claude/rust/claude-core/target/release/claude-core"
  [[ -x "$local_bin" ]] && IPC_BIN="$local_bin" || {
    echo '{"error":"claude-core binary not found"}'
    exit 1
  }
fi

[[ ! -f "$DB_PATH" ]] && echo '{"error":"db not found"}' && exit 1

# --- Core IPC status (Plan 633/634) ---
get_status() {
  local result
  result=$("$IPC_BIN" ipc status --json 2>/dev/null) && echo "$result" && return 0

  # SQLite fallback
  if [[ -f "$DB_PATH" ]]; then
    local agents messages channels ctx
    agents=$(sqlite3 "$DB_PATH" "SELECT count(*) FROM ipc_agents;" 2>/dev/null || echo 0)
    messages=$(sqlite3 "$DB_PATH" "SELECT count(*) FROM ipc_messages WHERE created_at > datetime('now','-1 day');" 2>/dev/null || echo 0)
    channels=$(sqlite3 "$DB_PATH" "SELECT count(*) FROM ipc_channels;" 2>/dev/null || echo 0)
    ctx=$(sqlite3 "$DB_PATH" "SELECT count(*) FROM ipc_shared_context;" 2>/dev/null || echo 0)
    printf '{"kind":"stats","agents":%d,"messages":%d,"channels":%d,"context_keys":%d}' \
      "$agents" "$messages" "$channels" "$ctx"
  else
    echo '{"error":"no IPC database found"}'
    return 1
  fi
}

get_agents() {
  "$IPC_BIN" ipc who --json 2>/dev/null || {
    if [[ -f "$DB_PATH" ]]; then
      sqlite3 -json "$DB_PATH" "SELECT name,host,agent_type,last_seen FROM ipc_agents ORDER BY name;" 2>/dev/null || echo '[]'
    else
      echo '[]'
    fi
  }
}

# --- Intelligence Layer (Plan 635) ---
echo "{"

# Core status
echo '"core":'
status_json=$(get_status 2>/dev/null || echo '{}')
echo "$status_json,"

# Agents
echo '"agents":'
agents_json=$(get_agents 2>/dev/null || echo '[]')
echo "$agents_json,"

# Budget summary
echo '"budget":['
sqlite3 -json "$DB_PATH" "
  SELECT s.name, s.provider, s.budget_usd,
    COALESCE(SUM(b.estimated_cost_usd),0) as spent,
    s.budget_usd - COALESCE(SUM(b.estimated_cost_usd),0) as remaining
  FROM ipc_subscriptions s
  LEFT JOIN ipc_budget_log b ON b.subscription=s.name
  GROUP BY s.name
" 2>/dev/null || echo '[]'
echo '],'

# Model inventory
echo '"models":['
sqlite3 -json "$DB_PATH" "
  SELECT host, provider, model, size_gb, quantization, last_seen
  FROM ipc_model_registry ORDER BY host, provider
" 2>/dev/null || echo '[]'
echo '],'

# Skill pool
echo '"skills":['
sqlite3 -json "$DB_PATH" "
  SELECT skill, agent, host, confidence, last_used
  FROM ipc_agent_skills WHERE agent != '' ORDER BY skill, confidence DESC
" 2>/dev/null || echo '[]'
echo '],'

# Auth status
echo '"auth":['
sqlite3 -json "$DB_PATH" "
  SELECT service, host, updated_at FROM ipc_auth_tokens ORDER BY service
" 2>/dev/null || echo '[]'
echo ']'

echo "}"
