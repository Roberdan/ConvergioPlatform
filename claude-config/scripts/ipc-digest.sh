#!/usr/bin/env bash
# ipc-digest.sh — Compact JSON IPC status (core + intelligence layer)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IPC_BIN="${IPC_BIN:-claude-core}"
DAEMON_API="http://localhost:8420"

command -v jq &>/dev/null || { echo '{"error":"jq required"}'; exit 1; }

# Check binary
if ! command -v "$IPC_BIN" &>/dev/null; then
  local_bin="$HOME/.claude/rust/claude-core/target/release/claude-core"
  [[ -x "$local_bin" ]] && IPC_BIN="$local_bin" || IPC_BIN=""
fi

# --- Core IPC status (Plan 633/634) ---
get_status() {
  local result
  if [[ -n "$IPC_BIN" ]]; then
    result=$("$IPC_BIN" ipc status --json 2>/dev/null) && echo "$result" && return 0
  fi

  # Daemon API fallback
  local ipc_data
  ipc_data="$(curl -sf "${DAEMON_API}/api/ipc/agents" 2>/dev/null || echo '[]')"
  local agents messages
  agents="$(echo "$ipc_data" | jq 'length' 2>/dev/null || echo 0)"
  messages="$(curl -sf "${DAEMON_API}/api/ipc/messages" 2>/dev/null | jq 'length' 2>/dev/null || echo 0)"
  printf '{"kind":"stats","agents":%d,"messages":%d}' "$agents" "$messages"
}

get_agents() {
  if [[ -n "$IPC_BIN" ]]; then
    "$IPC_BIN" ipc who --json 2>/dev/null && return 0
  fi
  curl -sf "${DAEMON_API}/api/ipc/agents" 2>/dev/null || echo '[]'
}

# --- Intelligence Layer (Plan 635) ---
# Build the full JSON via jq to avoid broken pipes
_core="$(get_status 2>/dev/null || echo '{}')"
_agents="$(get_agents 2>/dev/null || echo '[]')"
# Budget, models, skills, auth via IPC endpoints
_budget="$(curl -sf "${DAEMON_API}/api/ipc/budget" 2>/dev/null || echo '[]')"
_models="$(curl -sf "${DAEMON_API}/api/ipc/models" 2>/dev/null || echo '[]')"
_skills="$(curl -sf "${DAEMON_API}/api/ipc/skills" 2>/dev/null || echo '[]')"
_auth="$(curl -sf "${DAEMON_API}/api/ipc/auth" 2>/dev/null || echo '[]')"

jq -nc --argjson core "$_core" --argjson agents "$_agents" \
  --argjson budget "$_budget" --argjson models "$_models" \
  --argjson skills "$_skills" --argjson auth "$_auth" \
  '{core:$core,agents:$agents,budget:$budget,models:$models,skills:$skills,auth:$auth}'
