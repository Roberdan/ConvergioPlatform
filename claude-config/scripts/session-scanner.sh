#!/usr/bin/env bash
set -euo pipefail
# session-scanner.sh — Detects active Claude/Copilot CLI sessions
# Writes to agent_activity table for brain visualization as "consciousness nodes"
# Usage: session-scanner.sh [scan|list]

DB="${PLAN_DB:-$HOME/.claude/data/dashboard.db}"
HOST="$(hostname -s 2>/dev/null || echo local)"

# CRR tables need crsqlite loaded for trigger functions
CRSQL=""
for p in "$HOME/.claude/lib/crsqlite/crsqlite" "/opt/homebrew/lib/crsqlite/crsqlite" "/usr/local/lib/crsqlite/crsqlite"; do
  [ -f "$p.dylib" ] || [ -f "$p.so" ] || [ -f "$p" ] && { CRSQL="$p"; break; }
done
LOAD_EXT=""
[ -n "$CRSQL" ] && LOAD_EXT=".load $CRSQL"

# macOS system sqlite3 has .load disabled; prefer homebrew
SQLITE3="sqlite3"
[ -x /opt/homebrew/opt/sqlite/bin/sqlite3 ] && SQLITE3="/opt/homebrew/opt/sqlite/bin/sqlite3"

_sql() {
  if [ -n "$CRSQL" ]; then
    "$SQLITE3" -cmd ".load $CRSQL" "$DB" "$@"
  else
    "$SQLITE3" "$DB" "$@"
  fi
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

    _sql <<-SQL 2>/dev/null || true
INSERT INTO agent_activity (agent_id, agent_type, description, model, host, status, region, metadata)
VALUES ('${SESSION_ID}', '${TYPE}', '${SAFE_DESC}', '${SAFE_MODEL}', '${HOST}', 'running', 'prefrontal',
  '{"pid":${PID},"tty":"${SAFE_TTY}","cpu":${CPU},"mem":${MEM},"cwd":"${SAFE_CWD}"}')
ON CONFLICT(agent_id) DO UPDATE SET
  description=CASE WHEN '${SAFE_DESC}' <> '' AND '${SAFE_DESC}' <> excluded.agent_type THEN '${SAFE_DESC}' ELSE agent_activity.description END,
  model=CASE WHEN '${SAFE_MODEL}' <> '' AND '${SAFE_MODEL}' <> excluded.agent_type THEN '${SAFE_MODEL}' ELSE agent_activity.model END,
  metadata='{"pid":${PID},"tty":"${SAFE_TTY}","cpu":${CPU},"mem":${MEM},"cwd":"${SAFE_CWD}"}',
  status='running';
SQL
    echo "$SESSION_ID"
  done
}

cleanup_stale() {
  _sql "SELECT agent_id FROM agent_activity WHERE agent_id LIKE 'session-%' AND status='running';" 2>/dev/null | while read -r sid; do
    PID="${sid##*-}"
    if ! ps -p "$PID" > /dev/null 2>&1; then
      _sql "UPDATE agent_activity SET status='completed', completed_at=datetime('now'), \
        duration_s=CAST((julianday('now')-julianday(started_at))*86400 AS REAL) WHERE agent_id='${sid}';" 2>/dev/null || true
    fi
  done
}

case "${1:-scan}" in
  scan) scan_sessions; cleanup_stale ;;
  list)
    if [ -n "$CRSQL" ]; then
      "$SQLITE3" -json -cmd ".load $CRSQL" "$DB" "SELECT agent_id, agent_type AS type, description, status, metadata FROM agent_activity WHERE agent_id LIKE 'session-%' AND status='running';" 2>/dev/null || echo '[]'
    else
      "$SQLITE3" -json "$DB" "SELECT agent_id, agent_type AS type, description, status, metadata FROM agent_activity WHERE agent_id LIKE 'session-%' AND status='running';" 2>/dev/null || echo '[]'
    fi ;;
  *) echo "Usage: session-scanner.sh [scan|list]"; exit 2 ;;
esac
