#!/usr/bin/env bash
# agent-ipc-shell.sh — Source this in .zshrc/.bashrc for IPC agent wrappers
# Usage: source ~/.claude/scripts/agent-ipc-shell.sh

# If executed directly (not sourced), enable strict mode and error trap.
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  set -euo pipefail
  trap 'echo "ERROR at line $LINENO" >&2' ERR
fi

CLAUDE_HOME=${CLAUDE_HOME:-$HOME/.claude}

# Locate claude-core binary
_ipc_find_bin() {
  local candidates=(
    "claude-core"
    "${CLAUDE_HOME}/rust/claude-core/target/release/claude-core"
    "$HOME/bin/claude-core"
    "$HOME/.local/bin/claude-core"
  )
  for bin in "${candidates[@]}"; do
    if command -v "$bin" &>/dev/null || [[ -x "$bin" ]]; then
      echo "$bin"
      return 0
    fi
  done
  echo "claude-core" # fallback, hope it's in PATH
}

IPC_BIN="$(_ipc_find_bin)"

# ipc() — shorthand for claude-core ipc
ipc() {
  "$IPC_BIN" ipc "$@"
}

# Cleanup trap — auto-unregister on shell exit
_ipc_cleanup() {
  if [[ -n "$AGENT_IPC_NAME" ]]; then
    "$IPC_BIN" ipc unregister --name "$AGENT_IPC_NAME" 2>/dev/null
    unset AGENT_IPC_NAME
  fi
}

# ca() — launch Claude with IPC agent registration
ca() {
  local name="${1:-}"
  if [[ -z "$name" ]]; then
    printf "Agent name: "
    read -r name
  fi
  [[ -z "$name" ]] && { echo "Error: agent name required"; return 1; }

  "$IPC_BIN" ipc register --name "$name" --type claude --pid $$
  export AGENT_IPC_NAME="$name"
  trap _ipc_cleanup EXIT

  if command -v claude &>/dev/null; then
    claude "$@"
  else
    echo "claude not found in PATH"
    return 1
  fi
}

# cpa() — launch Copilot with IPC agent registration
cpa() {
  local name="${1:-}"
  if [[ -z "$name" ]]; then
    printf "Agent name: "
    read -r name
  fi
  [[ -z "$name" ]] && { echo "Error: agent name required"; return 1; }

  "$IPC_BIN" ipc register --name "$name" --type copilot --pid $$
  export AGENT_IPC_NAME="$name"
  trap _ipc_cleanup EXIT

  if command -v copilot &>/dev/null; then
    copilot "$@"
  else
    echo "copilot not found in PATH"
    return 1
  fi
}

# Source instructions
# Add to .zshrc:  source ~/.claude/scripts/agent-ipc-shell.sh

# _emit_lifecycle() — emit agent lifecycle events to mesh_events table
_emit_lifecycle() {
  local event_type="${1:?event_type required}" # agent_started | agent_finished
  local agent_name="${2:-${AGENT_IPC_NAME:-unknown}}"
  local payload="${3:-{}}"
  local db="${CLAUDE_HOME}/data/dashboard.db"
  [[ -f "$db" ]] || return 0
  local host
  host="$(hostname -s 2>/dev/null || echo 'unknown')"
  sqlite3 -cmd ".timeout 3000" "$db" \
    "INSERT INTO mesh_events (event_type, source_peer, plan_id, payload, status, created_at)
     VALUES ('${event_type}', '${host}', 0,
       '{\"agent\":\"${agent_name}\",\"host\":\"${host}\",\"ts\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",\"detail\":${payload}}',
       'pending', unixepoch());" 2>/dev/null || true
}
