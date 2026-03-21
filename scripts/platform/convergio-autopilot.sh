#!/usr/bin/env bash
# convergio-autopilot.sh — Autonomous plan execution loop
# Watches plan state in DB and auto-triggers: execution → Thor → merge
# Usage: convergio-autopilot.sh [plan_id] [--interval 30]
set -uo pipefail

PLATFORM_DIR="${CONVERGIO_PLATFORM_DIR:-$HOME/GitHub/ConvergioPlatform}"
DB="${DASHBOARD_DB:-$PLATFORM_DIR/data/dashboard.db}"
SCRIPTS="$PLATFORM_DIR/claude-config/scripts"
BUS="$PLATFORM_DIR/scripts/platform/convergio-bus.sh"
DAEMON_URL="${CONVERGIO_DAEMON_URL:-http://localhost:8420}"
INTERVAL="${2:-30}"
PLAN_ID="${1:-}"

MAX_BUDGET="${CONVERGIO_MAX_BUDGET:-10.00}"  # F2: daily budget cap in USD
RETRY_FILE="/tmp/convergio-retry-state"

log() { echo "[$(date '+%H:%M:%S')] $*"; }
warn() { echo "[$(date '+%H:%M:%S')] WARN: $*" >&2; }

# ─── F1-F3: Cost Tracking ───────────────────────────────────────────

get_daily_cost() {
  _db "SELECT COALESCE(SUM(cost_usd), 0) FROM execution_runs
       WHERE started_at > datetime('now', '-1 day');"
}

check_budget() {
  local spent
  spent=$(get_daily_cost)
  local over
  over=$(echo "$spent >= $MAX_BUDGET" | bc -l 2>/dev/null || echo "0")
  if [ "${over:-0}" -eq 1 ]; then
    warn "BUDGET CAP reached: \$$spent / \$$MAX_BUDGET daily. Pausing execution."
    "$BUS" broadcast "autopilot" "BUDGET CAP: \$$spent spent today. Execution paused." 2>/dev/null || true
    return 1
  fi
  return 0
}

# ─── G2: Agent Health Monitoring ────────────────────────────────────

AGENT_TIMEOUT=600  # 10 minutes silence = zombie

check_agent_health() {
  if ! curl -sf --connect-timeout 1 "$DAEMON_URL/api/ipc/agents" > /dev/null 2>&1; then
    return 0  # daemon not running, skip
  fi

  local agents
  agents=$(curl -sf "$DAEMON_URL/api/ipc/agents" 2>/dev/null)
  [ -z "$agents" ] && return 0

  echo "$agents" | python3 -c "
import sys, json, datetime
try:
    d = json.load(sys.stdin)
    now = datetime.datetime.utcnow()
    for a in d.get('agents', []):
        last = a.get('last_seen', '')
        if not last: continue
        try:
            ts = datetime.datetime.fromisoformat(last.replace('Z',''))
            delta = (now - ts).total_seconds()
            if delta > $AGENT_TIMEOUT:
                print(f'ZOMBIE: {a[\"name\"]} (silent {int(delta)}s)')
        except: pass
except: pass
" 2>/dev/null | while read -r line; do
    warn "$line"
  done
}

# ─── G3: Retry with Backoff ─────────────────────────────────────────

get_retry_count() {
  local task_id="$1"
  grep -c "^$task_id:" "$RETRY_FILE" 2>/dev/null || echo "0"
}

record_retry() {
  local task_id="$1"
  echo "$task_id:$(date +%s)" >> "$RETRY_FILE"
}

backoff_seconds() {
  local attempt="$1"
  # Exponential: 30, 60, 120
  echo $(( 30 * (2 ** (attempt - 1)) ))
}

_db() { sqlite3 "$DB" "$1" 2>/dev/null; }

# ─── Plan Discovery ─────────────────────────────────────────────────

find_actionable_plan() {
  if [ -n "$PLAN_ID" ]; then
    echo "$PLAN_ID"
  else
    # Find plans in 'doing' status with pending waves
    _db "SELECT p.id FROM plans p
         WHERE p.status = 'doing'
         AND EXISTS (SELECT 1 FROM waves w WHERE w.plan_id = p.id AND w.status IN ('pending','in_progress'))
         ORDER BY p.id DESC LIMIT 1;"
  fi
}

# ─── Wave State Machine ─────────────────────────────────────────────

get_current_wave() {
  local pid="$1"
  _db "SELECT id, wave_id, status, tasks_done, tasks_total
       FROM waves WHERE plan_id = $pid AND status IN ('pending','in_progress','merging')
       ORDER BY position LIMIT 1;"
}

count_pending_tasks() {
  local wave_db_id="$1"
  _db "SELECT count(*) FROM tasks WHERE wave_id_fk = $wave_db_id AND status = 'pending';"
}

count_submitted_tasks() {
  local wave_db_id="$1"
  _db "SELECT count(*) FROM tasks WHERE wave_id_fk = $wave_db_id AND status = 'submitted';"
}

count_in_progress_tasks() {
  local wave_db_id="$1"
  _db "SELECT count(*) FROM tasks WHERE wave_id_fk = $wave_db_id AND status = 'in_progress';"
}

all_tasks_submitted_or_done() {
  local wave_db_id="$1"
  local remaining
  remaining=$(_db "SELECT count(*) FROM tasks WHERE wave_id_fk = $wave_db_id AND status NOT IN ('submitted','done','skipped','cancelled');")
  [ "${remaining:-1}" -eq 0 ]
}

wave_all_done() {
  local wave_db_id="$1"
  local remaining
  remaining=$(_db "SELECT count(*) FROM tasks WHERE wave_id_fk = $wave_db_id AND status NOT IN ('done','skipped','cancelled');")
  [ "${remaining:-1}" -eq 0 ]
}

# ─── Actions ─────────────────────────────────────────────────────────

trigger_execution() {
  local pid="$1" wave_db_id="$2" wave_id="$3"
  log "DISPATCH: Executing wave $wave_id (plan $pid)"

  # Register autopilot as agent
  "$BUS" register "autopilot" "auto-executor" "system" 2>/dev/null || true

  # Notify via IPC
  "$BUS" send "autopilot" "general" "Auto-dispatching wave $wave_id for plan $pid" 2>/dev/null || true

  # Update wave status
  _db "UPDATE waves SET status = 'in_progress' WHERE id = $wave_db_id AND status = 'pending';"

  # Launch executor (this spawns Claude/Copilot with /execute)
  log "  Spawning executor for plan $pid..."
  if command -v claude &>/dev/null; then
    claude -p "Esegui /execute $pid. Focus su wave $wave_id. Modalità autonoma." &
    local executor_pid=$!
    log "  Executor PID: $executor_pid"
  else
    warn "Claude CLI not found — manual execution needed"
  fi
}

trigger_thor() {
  local pid="$1" wave_db_id="$2" wave_id="$3"
  log "THOR: Validating wave $wave_id (plan $pid)"

  "$BUS" send "autopilot" "general" "Thor validation starting for wave $wave_id" 2>/dev/null || true

  if command -v claude &>/dev/null; then
    claude -p "Sei Thor. Valida wave $wave_id (db_id: $wave_db_id) del plan $pid. Tutti i 10 gate. Se PASS: plan-db.sh validate-wave $wave_db_id" &
    log "  Thor spawned"
  else
    warn "Claude CLI not found — run manually: plan-db.sh validate-wave $wave_db_id"
  fi
}

trigger_merge() {
  local pid="$1" wave_db_id="$2" wave_id="$3"
  log "MERGE: Wave $wave_id (plan $pid)"

  "$BUS" send "autopilot" "general" "Merging wave $wave_id" 2>/dev/null || true

  bash "$SCRIPTS/wave-worktree.sh" merge "$pid" "$wave_db_id" 2>/dev/null && {
    log "  Merge complete"
  } || {
    warn "Merge failed — manual intervention needed"
  }
}

trigger_calibration() {
  local pid="$1"
  log "CALIBRATE: Post-plan calibration for plan $pid"
  bash "$SCRIPTS/plan-db.sh" calibrate-estimates 2>/dev/null || true
  log "  Calibration done"
}

trigger_postmortem() {
  local pid="$1"
  log "POSTMORTEM: Analyzing plan $pid"

  if command -v claude &>/dev/null; then
    claude -p "Sei plan-post-mortem. Analizza plan $pid: plan-db.sh get-learnings $pid. Estrai pattern, scrivi learnings con plan-db.sh add-learning." &
    log "  Post-mortem spawned"
  fi
}

# ─── Main Loop ───────────────────────────────────────────────────────

run_once() {
  local pid
  pid=$(find_actionable_plan)

  if [ -z "$pid" ]; then
    return 1  # No actionable plan
  fi

  local wave_info
  wave_info=$(get_current_wave "$pid")

  if [ -z "$wave_info" ]; then
    # All waves done — plan complete
    log "COMPLETE: Plan $pid — all waves done"
    _db "UPDATE plans SET status = 'done', completed_at = datetime('now') WHERE id = $pid AND status = 'doing';"
    trigger_calibration "$pid"
    trigger_postmortem "$pid"
    return 0
  fi

  # Parse wave info
  local wave_db_id wave_id wave_status tasks_done tasks_total
  IFS='|' read -r wave_db_id wave_id wave_status tasks_done tasks_total <<< "$wave_info"

  log "Plan $pid | Wave $wave_id ($wave_status) | Tasks $tasks_done/$tasks_total"

  case "$wave_status" in
    pending)
      # Wave needs execution
      trigger_execution "$pid" "$wave_db_id" "$wave_id"
      ;;
    in_progress)
      if all_tasks_submitted_or_done "$wave_db_id"; then
        # All tasks submitted → trigger Thor
        local submitted
        submitted=$(count_submitted_tasks "$wave_db_id")
        if [ "${submitted:-0}" -gt 0 ]; then
          trigger_thor "$pid" "$wave_db_id" "$wave_id"
        fi
      else
        local in_progress pending
        in_progress=$(count_in_progress_tasks "$wave_db_id")
        pending=$(count_pending_tasks "$wave_db_id")
        log "  Waiting: $in_progress in_progress, $pending pending"
      fi
      ;;
    merging)
      if wave_all_done "$wave_db_id"; then
        trigger_merge "$pid" "$wave_db_id" "$wave_id"
      else
        log "  Wave merging but tasks not all done yet"
      fi
      ;;
  esac

  return 0
}

cmd_once() {
  log "=== Convergio Autopilot (single run) ==="
  run_once || log "No actionable plans found"
}

cmd_watch() {
  log "=== Convergio Autopilot (watching every ${INTERVAL}s) ==="
  log "Press Ctrl+C to stop"

  "$BUS" register "autopilot" "autonomous-executor" "system" 2>/dev/null || true

  trap '"$BUS" unregister autopilot 2>/dev/null; exit 0' INT TERM

  while true; do
    # F2: Check budget before each cycle
    check_budget || { sleep 300; continue; }
    # G2: Check agent health
    check_agent_health
    # Run main loop
    run_once || true
    # Collect metrics every cycle
    bash "$PLATFORM_DIR/scripts/platform/convergio-metrics.sh" collect > /dev/null 2>&1
    sleep "$INTERVAL"
  done
}

cmd_status() {
  echo "=== Autopilot Status ==="
  local active
  active=$(_db "SELECT count(*) FROM plans WHERE status = 'doing';")
  echo "  Active plans: ${active:-0}"

  _db "SELECT p.id, p.name, p.status, p.tasks_done || '/' || p.tasks_total as progress
       FROM plans p WHERE p.status = 'doing' ORDER BY p.id DESC LIMIT 5;" | while IFS='|' read -r id name status progress; do
    echo "  Plan $id: $name ($progress)"
  done
}

case "${1:-once}" in
  once)    cmd_once ;;
  watch)   cmd_watch ;;
  status)  cmd_status ;;
  *)
    echo "convergio-autopilot.sh — Autonomous plan execution"
    echo "  once            Run one check cycle"
    echo "  watch           Watch continuously (every ${INTERVAL}s)"
    echo "  status          Show active plans"
    ;;
esac
