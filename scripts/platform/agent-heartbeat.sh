#!/usr/bin/env bash
# agent-heartbeat.sh — Send periodic heartbeat for a registered IPC agent
# Usage: agent-heartbeat.sh --name <agent> [--task <current_task>]
set -euo pipefail

CONVERGIO_DAEMON_URL="${CONVERGIO_DAEMON_URL:-http://localhost:8420}"

usage() {
  echo "Usage: agent-heartbeat.sh --name <agent> [--task <current_task>]" >&2
  exit 1
}

main() {
  local name="${AGENT_IPC_NAME:-}"
  local task=""

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --name) name="$2"; shift 2 ;;
      --task) task="$2"; shift 2 ;;
      *) usage ;;
    esac
  done

  if [[ -z "$name" ]]; then
    echo "error: --name or AGENT_IPC_NAME required" >&2
    exit 1
  fi

  # Collect current state
  local branch
  branch="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")"

  local worktree
  worktree="$(git rev-parse --show-toplevel 2>/dev/null || echo "unknown")"

  # Resolve task: explicit arg > plan-db.sh lookup > "idle"
  if [[ -z "$task" ]]; then
    if command -v plan-db.sh &>/dev/null; then
      task="$(plan-db.sh current-task 2>/dev/null || echo "idle")"
    else
      task="idle"
    fi
  fi

  # POST heartbeat to daemon — use jq to prevent JSON injection via field values
  local heartbeat_body
  heartbeat_body=$(jq -n --arg id "$name" --arg h "$(hostname)" --arg t "$task" \
    '{agent_id:$id,host:$h,current_task:$t}')

  if ! curl -s --max-time 3 -X POST \
    "${CONVERGIO_DAEMON_URL}/api/ipc/agents/heartbeat" \
    -H "Content-Type: application/json" \
    -d "$heartbeat_body" >/dev/null 2>&1; then
    echo "warn: daemon not reachable at ${CONVERGIO_DAEMON_URL}" >&2
    exit 0
  fi

  # Send info message on heartbeat channel for dashboard visibility
  local info_body
  info_body=$(jq -n --arg sn "$name" --arg ch "heartbeat" \
    --arg ct "branch=${branch} worktree=${worktree} task=${task}" \
    '{sender_name:$sn,channel:$ch,content:$ct}')

  if ! curl -s --max-time 3 -X POST \
    "${CONVERGIO_DAEMON_URL}/api/ipc/send" \
    -H "Content-Type: application/json" \
    -d "$info_body" >/dev/null 2>&1; then
    echo "warn: failed to send heartbeat info message" >&2
  fi
}

main "$@"
