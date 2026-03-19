#!/usr/bin/env bash
# agent-bridge.sh — Bridge between Claude Code/Copilot agents and ConvergioPlatform IPC daemon
# Called by SubagentStart hook, or manually for agent lifecycle management
# Library+CLI: source for functions, execute directly for CLI dispatch
set -euo pipefail

CONVERGIO_DAEMON_URL="${CONVERGIO_DAEMON_URL:-http://localhost:8420}"
_CURL_OPTS=(-s --max-time 3 -H 'Content-Type: application/json')

# --- Library functions (sourceable) ---

_post() {
  local endpoint="$1" body="$2"
  curl "${_CURL_OPTS[@]}" -X POST "${CONVERGIO_DAEMON_URL}${endpoint}" -d "$body" 2>/dev/null
}

agent_register() {
  local name="${1:?agent name required}"
  local type="${2:-claude}"
  local pid="${3:-$$}"
  local body
  body=$(printf '{"agent_id":"%s","type":"%s","host":"%s","pid":%d,"status":"active"}' \
    "$name" "$type" "$(hostname)" "$pid")
  if _post "/api/ipc/agents/register" "$body" >/dev/null; then
    echo "Registered agent: $name (type=$type, pid=$pid)" >&2
  else
    echo "Warning: daemon unreachable, agent $name not registered (continuing)" >&2
  fi
  return 0
}

agent_unregister() {
  local name="${1:?agent name required}"
  local body
  body=$(printf '{"agent_id":"%s","host":"%s"}' "$name" "$(hostname)")
  if _post "/api/ipc/agents/unregister" "$body" >/dev/null; then
    echo "Unregistered agent: $name" >&2
  else
    echo "Warning: daemon unreachable, agent $name not unregistered" >&2
  fi
  return 0
}

agent_heartbeat() {
  local name="${1:?agent name required}"
  local current_task="${2:-idle}"
  local body
  body=$(printf '{"agent_id":"%s","host":"%s","current_task":"%s"}' \
    "$name" "$(hostname)" "$current_task")
  if _post "/api/ipc/agents/heartbeat" "$body" >/dev/null; then
    echo "Heartbeat sent: $name (task=$current_task)" >&2
  else
    echo "Warning: daemon unreachable, heartbeat for $name skipped" >&2
  fi
  return 0
}

agent_checkpoint() {
  local name="${1:?agent name required}"
  local key="${2:?checkpoint key required}"
  local value="${3:?checkpoint value required}"
  local body
  body=$(printf '{"sender_name":"%s","channel":"checkpoint","content":"%s=%s"}' \
    "$name" "$key" "$value")
  if _post "/api/ipc/send" "$body" >/dev/null; then
    echo "Checkpoint sent: $name ($key=$value)" >&2
  else
    echo "Warning: daemon unreachable, checkpoint for $name skipped" >&2
  fi
  return 0
}

# --- CLI dispatch (only when executed directly) ---

_usage() {
  cat >&2 <<'USAGE'
Usage: agent-bridge.sh <mode> [options]

Modes:
  --register    --name NAME [--type TYPE] [--pid PID]
  --unregister  --name NAME
  --heartbeat   --name NAME [--task TASK]
  --checkpoint  --name NAME --key KEY --value VALUE

Env: CONVERGIO_DAEMON_URL (default: http://localhost:8420)
Stdin: reads JSON from SubagentStart hook automatically.
USAGE
  exit 1
}

_parse_hook_stdin() {
  local input name type
  input=$(cat)
  name=$(echo "$input" | jq -r '.subagent_type // .description // "unknown"' 2>/dev/null || echo "unknown")
  type="claude"
  agent_register "$name" "$type"
}

main() {
  # Hook mode: stdin is not a terminal, read JSON
  if [[ ! -t 0 ]]; then
    _parse_hook_stdin
    return 0
  fi

  local mode="" name="" type="claude" pid="" task="" key="" value=""
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --register)    mode="register"; shift ;;
      --unregister)  mode="unregister"; shift ;;
      --heartbeat)   mode="heartbeat"; shift ;;
      --checkpoint)  mode="checkpoint"; shift ;;
      --name)        name="${2:?--name requires a value}"; shift 2 ;;
      --type)        type="${2:?--type requires a value}"; shift 2 ;;
      --pid)         pid="${2:?--pid requires a value}"; shift 2 ;;
      --task)        task="${2:?--task requires a value}"; shift 2 ;;
      --key)         key="${2:?--key requires a value}"; shift 2 ;;
      --value)       value="${2:?--value requires a value}"; shift 2 ;;
      --help|-h)     _usage ;;
      *)             echo "Unknown option: $1" >&2; _usage ;;
    esac
  done

  [[ -z "$mode" ]] && { echo "Error: no mode specified" >&2; _usage; }

  case "$mode" in
    register)
      [[ -z "$name" ]] && { echo "Error: --name required" >&2; exit 1; }
      agent_register "$name" "$type" "${pid:-}"
      ;;
    unregister)
      [[ -z "$name" ]] && { echo "Error: --name required" >&2; exit 1; }
      agent_unregister "$name"
      ;;
    heartbeat)
      [[ -z "$name" ]] && { echo "Error: --name required" >&2; exit 1; }
      agent_heartbeat "$name" "${task:-idle}"
      ;;
    checkpoint)
      [[ -z "$name" ]] && { echo "Error: --name required" >&2; exit 1; }
      [[ -z "$key" ]] && { echo "Error: --key required" >&2; exit 1; }
      [[ -z "$value" ]] && { echo "Error: --value required" >&2; exit 1; }
      agent_checkpoint "$name" "$key" "$value"
      ;;
  esac
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  main "$@"
fi
