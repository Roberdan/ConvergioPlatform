#!/usr/bin/env bash
set -euo pipefail
# session-scanner.sh — Detects active Claude/Copilot CLI sessions
# Writes to agent_activity table for brain visualization as "consciousness nodes"
# Usage: session-scanner.sh [scan|list]

DAEMON_URL="${DAEMON_URL:-http://localhost:8420}"
HOST="$(hostname -s 2>/dev/null || echo local)"

command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 2; }

_api() {
  local endpoint="${1:?endpoint required}"
  shift
  curl -sf "${DAEMON_URL}/${endpoint}" "$@"
}

_api_post() {
  local endpoint="${1:?endpoint required}"
  local data="${2:?data required}"
  curl -sf -X POST "${DAEMON_URL}/${endpoint}" \
    -H 'Content-Type: application/json' \
    -d "$data"
}

sanitize() { echo "$1" | tr "'" "_" | cut -c1-200; }

# Extract model from CLI flags — tool-agnostic, works with any CLI that uses --model/-m
extract_model() {
  local cmd="$1" type="$2"
  local model=""
  # --model VALUE or -m VALUE (universal flag across claude, copilot, opencode, etc.)
  model=$(echo "$cmd" | grep -oE '(--model|-m)\s+[^ ]+' | head -1 | awk '{print $2}')
  if [ -n "$model" ]; then echo "$model"; return; fi
  # OpenCode: --provider/model syntax in config, not always on CLI
  # Fall back to type so brain-canvas shortModel filters it
  echo "$type"
}

# Build a human-friendly description from CLI args — tool-agnostic
extract_description() {
  local cmd="$1" type="$2"
  local desc=""
  # -p "prompt" or --prompt "prompt" (double-quoted) — universal across tools
  desc=$(echo "$cmd" | sed -n 's/.*\(-p\|--prompt\) "\([^"]*\)".*/\2/p' | head -1 | cut -c1-60)
  if [ -n "$desc" ]; then echo "$desc"; return; fi
  # -p 'prompt' (single-quoted)
  desc=$(echo "$cmd" | sed -n "s/.*\\(-p\\|--prompt\\) '\\([^']*\\)'.*/\\2/p" | head -1 | cut -c1-60)
  if [ -n "$desc" ]; then echo "$desc"; return; fi
  # /skill invocation (claude: /planner, /execute, etc.)
  desc=$(echo "$cmd" | grep -oE '/[a-z][-a-z]+' | head -1)
  if [ -n "$desc" ]; then echo "skill: $desc"; return; fi
  # --add-dir project name
  desc=$(echo "$cmd" | grep -oE '\-\-add-dir\s+[^ ]+' | head -1 | awk '{print $2}' | xargs basename 2>/dev/null)
  if [ -n "$desc" ]; then echo "project: $desc"; return; fi
  # OpenCode: -d/--dir working directory
  desc=$(echo "$cmd" | grep -oE '(-d|--dir)\s+[^ ]+' | head -1 | awk '{print $2}' | xargs basename 2>/dev/null)
  if [ -n "$desc" ]; then echo "project: $desc"; return; fi
  echo ""
}

scan_sessions() {
  # Tool-agnostic: detect claude, copilot, opencode, and future CLI tools
  ps aux 2>/dev/null | grep -E '(claude|copilot|opencode)' | grep -v -E 'grep|hook|scanner|plan-db|track|\.sh' | while IFS= read -r line; do
    PID=$(echo "$line" | awk '{print $2}')
    CPU=$(echo "$line" | awk '{print $3}')
    MEM=$(echo "$line" | awk '{print $4}')
    TTY=$(echo "$line" | awk '{print $7}')
    CMD=$(echo "$line" | awk '{for(i=11;i<=NF;i++) printf "%s ", $i; print ""}')

    # Only match main CLI processes, not node subprocesses or workers
    case "$CMD" in
      *copilot*agent*|*copilot*worker*|*node*copilot*) continue ;;
      *claude*hook*|*claude*plan-db*|*claude*script*) continue ;;
      *claude-core*|*claude-co*) continue ;;
      *"Cursor Helper"*|*extension-host*) continue ;;
      *opencode*lsp*|*opencode*worker*) continue ;;
    esac

    # Determine type — tool-agnostic detection
    TYPE="unknown"
    if echo "$CMD" | grep -qi "copilot"; then TYPE="copilot-cli"
    elif echo "$CMD" | grep -qi "opencode"; then TYPE="opencode"
    elif echo "$CMD" | grep -qi "claude"; then TYPE="claude-cli"
    else continue
    fi

    # Extract model and description from CLI args
    MODEL=$(extract_model "$CMD" "$TYPE")
    DESC=$(extract_description "$CMD" "$TYPE")

    # Get working directory (fail-silent)
    CWD=$(lsof -p "$PID" 2>/dev/null | awk '/cwd/{print $NF}' || echo "unknown")

    SESSION_ID="session-${TYPE}-${PID}"
    SAFE_CMD=$(sanitize "$CMD")
    SAFE_CWD=$(sanitize "$CWD")
    SAFE_TTY=$(sanitize "$TTY")
    SAFE_DESC=$(sanitize "${DESC:-$SAFE_CMD}")
    SAFE_MODEL=$(sanitize "$MODEL")

    _api_post "api/agents/activity" "$(jq -n \
      --arg id "$SESSION_ID" \
      --arg type "$TYPE" \
      --arg desc "$SAFE_DESC" \
      --arg model "$SAFE_MODEL" \
      --arg host "$HOST" \
      --argjson pid "$PID" \
      --arg tty "$SAFE_TTY" \
      --arg cpu "$CPU" \
      --arg mem "$MEM" \
      --arg cwd "$SAFE_CWD" \
      '{agent_id: $id, agent_type: $type, description: $desc, model: $model,
        host: $host, status: "running", region: "prefrontal",
        metadata: {pid: $pid, tty: $tty, cpu: ($cpu | tonumber), mem: ($mem | tonumber), cwd: $cwd}}')" 2>/dev/null || true
    echo "$SESSION_ID"
  done
}

cleanup_stale() {
  local agents
  agents=$(_api "api/agents/activity" 2>/dev/null || echo '[]')
  echo "$agents" | jq -r '.[]? | select(.agent_id | startswith("session-")) | select(.status == "running") | .agent_id' 2>/dev/null | while read -r sid; do
    PID="${sid##*-}"
    if ! ps -p "$PID" > /dev/null 2>&1; then
      _api_post "api/agents/activity/complete" "$(jq -n --arg id "$sid" '{agent_id: $id}')" 2>/dev/null || true
    fi
  done
}

case "${1:-scan}" in
  scan) scan_sessions; cleanup_stale ;;
  list)
    _api "api/agents/activity" 2>/dev/null | jq '[.[]? | select(.agent_id | startswith("session-")) | select(.status == "running") | {agent_id, type: .agent_type, description, status, metadata}]' 2>/dev/null || echo '[]'
    ;;
  *) echo "Usage: session-scanner.sh [scan|list]"; exit 2 ;;
esac
