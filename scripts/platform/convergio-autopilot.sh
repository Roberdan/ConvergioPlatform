#!/usr/bin/env bash
# convergio-autopilot.sh — Autonomous plan execution loop
# Watches plan state via daemon API and auto-triggers: execution -> Thor -> merge
# Usage: convergio-autopilot.sh [plan_id] [--interval 30]
set -euo pipefail

PLATFORM_DIR="${CONVERGIO_PLATFORM_DIR:-$HOME/GitHub/ConvergioPlatform}"
SCRIPTS="$PLATFORM_DIR/claude-config/scripts"
BUS="$PLATFORM_DIR/scripts/platform/convergio-bus.sh"
DAEMON_URL="${CONVERGIO_DAEMON_URL:-http://localhost:8420}"
INTERVAL="${2:-30}"
PLAN_ID="${1:-}"

MAX_BUDGET="${CONVERGIO_MAX_BUDGET:-10.00}"  # F2: daily budget cap in USD
# Scope retry state to the daemon URL so multiple autopilot instances don't collide
URL_HASH="$(printf '%s' "$DAEMON_URL" | shasum 2>/dev/null | cut -c1-8 || printf '%s' "$DAEMON_URL" | md5 2>/dev/null | cut -c1-8 || echo "global")"
RETRY_FILE="/tmp/convergio-retry-${URL_HASH}"

command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 1; }
command -v curl &>/dev/null || { echo "ERROR: curl required" >&2; exit 1; }

# Source helpers (plan discovery, wave state machine, trigger_*, execution_runs)
HELPERS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=convergio-autopilot-helpers.sh
source "$HELPERS_DIR/convergio-autopilot-helpers.sh"

log()  { echo "[$(date '+%H:%M:%S')] $*"; }
warn() { echo "[$(date '+%H:%M:%S')] WARN: $*" >&2; }

_api_get() { curl -sf --connect-timeout 2 "${DAEMON_URL}${1}" 2>/dev/null; }
_api_post() { curl -sf -X POST "${DAEMON_URL}${1}" -H 'Content-Type: application/json' -d "$2" 2>/dev/null; }

# ─── F1-F3: Cost Tracking ───────────────────────────────────────────

get_daily_cost() {
  local json
  json=$(_api_get "/api/metrics/cost") || { echo "0"; return; }
  echo "$json" | jq -r '.daily_cost // 0' 2>/dev/null || echo "0"
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

# ─── Main Loop ───────────────────────────────────────────────────────

run_once() {
  # B3 fix: apply pause_run events BEFORE checking plan state
  apply_pause_events

  local pid
  pid=$(find_actionable_plan)

  if [ -z "$pid" ]; then
    return 1  # No actionable plan
  fi

  # B3: check if this plan's run is paused via daemon API
  local run_status
  run_status=$(_api_get "/api/runs" | jq -r "[.runs[]? | select(.plan_id == ${pid} and (.status == \"running\" or .status == \"paused\"))] | sort_by(.started_at) | last | .status // empty" 2>/dev/null || echo "")
  if [ "${run_status:-}" = "paused" ]; then
    log "PAUSED: Plan $pid execution is paused — skipping"
    return 0
  fi

  local wave_info
  wave_info=$(get_current_wave "$pid")

  if [ -z "$wave_info" ]; then
    # All waves done — plan complete
    log "COMPLETE: Plan $pid — all waves done"
    cvg plan complete "$pid" 2>/dev/null || \
      _api_post "/api/plan-db/complete/${pid}" "{}" || true
    execution_runs_complete "$pid"
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
      trigger_execution "$pid" "$wave_db_id" "$wave_id"
      ;;
    in_progress)
      if all_tasks_submitted_or_done "$wave_db_id"; then
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
  local json
  json=$(_api_get "/api/overview") || { echo "  Daemon not reachable"; return 0; }

  local active
  active=$(echo "$json" | jq -r '.active_plans | length // 0' 2>/dev/null || echo "0")
  echo "  Active plans: ${active:-0}"

  echo "$json" | jq -r '.active_plans[]? | [(.id // ""), (.name // ""), (.tasks_done // 0), (.tasks_total // 0)] | @tsv' 2>/dev/null | head -5 | while IFS=$'\t' read -r id name tasks_done tasks_total; do
    echo "  Plan $id: $name ($tasks_done/$tasks_total)"
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
