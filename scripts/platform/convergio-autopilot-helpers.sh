#!/usr/bin/env bash
# convergio-autopilot-helpers.sh — Helper functions for convergio-autopilot.sh
# Sourced by convergio-autopilot.sh — do NOT execute directly.
# Provides: plan discovery, wave state machine, trigger_*, execution_runs wiring
set -euo pipefail

_validate_id() { [[ "$1" =~ ^[0-9]+$ ]] || { log "SECURITY: invalid ID '$1' — aborting"; return 1; }; }

_api_get() { curl -sf --connect-timeout 2 "${DAEMON_URL}${1}" 2>/dev/null; }
_api_post() { curl -sf -X POST "${DAEMON_URL}${1}" -H 'Content-Type: application/json' -d "$2" 2>/dev/null; }
_api_put() { curl -sf -X PUT "${DAEMON_URL}${1}" -H 'Content-Type: application/json' -d "$2" 2>/dev/null; }

# ─── Plan Discovery ─────────────────────────────────────────────────

find_actionable_plan() {
  if [ -n "${PLAN_ID:-}" ]; then
    echo "$PLAN_ID"
  else
    # Get actionable plan via daemon API
    local json
    json=$(_api_get "/api/overview") || return
    echo "$json" | jq -r '.active_plans[]? | select(.status == "doing") | .id' 2>/dev/null | head -1
  fi
}

# Pause bridge handled by daemon background task (Plan 679 T2-02)

# ─── Wave State Machine ─────────────────────────────────────────────

get_current_wave() {
  local pid="$1"
  _validate_id "$pid" || return 1
  # Get wave info from plan show JSON
  local json
  json=$(cvg plan show "$pid" 2>/dev/null) || json=$(_api_get "/api/plan-db/json/${pid}") || return
  # Return format: wave_db_id|wave_id|status|tasks_done|tasks_total (first actionable wave)
  echo "$json" | jq -r '[.waves[]? | select(.status == "pending" or .status == "in_progress" or .status == "merging")] | sort_by(.position) | first | [(.db_id // .id // ""), (.wave_id // ""), (.status // ""), (.tasks_done // 0), (.tasks_total // 0)] | join("|")' 2>/dev/null | grep -v '^||||0$' || echo ""
}

count_pending_tasks() {
  local wave_db_id="$1"
  _validate_id "$wave_db_id" || return 1
  local json
  json=$(cvg plan show "$PLAN_ID" 2>/dev/null) || json=$(_api_get "/api/plan-db/json/${PLAN_ID}") || { echo "0"; return; }
  echo "$json" | jq -r "[.tasks[]? | select(.wave_id_fk == ${wave_db_id} and .status == \"pending\")] | length" 2>/dev/null || echo "0"
}

count_submitted_tasks() {
  local wave_db_id="$1"
  _validate_id "$wave_db_id" || return 1
  local json
  json=$(cvg plan show "$PLAN_ID" 2>/dev/null) || json=$(_api_get "/api/plan-db/json/${PLAN_ID}") || { echo "0"; return; }
  echo "$json" | jq -r "[.tasks[]? | select(.wave_id_fk == ${wave_db_id} and .status == \"submitted\")] | length" 2>/dev/null || echo "0"
}

count_in_progress_tasks() {
  local wave_db_id="$1"
  _validate_id "$wave_db_id" || return 1
  local json
  json=$(cvg plan show "$PLAN_ID" 2>/dev/null) || json=$(_api_get "/api/plan-db/json/${PLAN_ID}") || { echo "0"; return; }
  echo "$json" | jq -r "[.tasks[]? | select(.wave_id_fk == ${wave_db_id} and .status == \"in_progress\")] | length" 2>/dev/null || echo "0"
}

all_tasks_submitted_or_done() {
  local wave_db_id="$1"
  _validate_id "$wave_db_id" || return 1
  local json remaining
  json=$(cvg plan show "$PLAN_ID" 2>/dev/null) || json=$(_api_get "/api/plan-db/json/${PLAN_ID}") || { return 1; }
  remaining=$(echo "$json" | jq -r "[.tasks[]? | select(.wave_id_fk == ${wave_db_id} and (.status != \"submitted\" and .status != \"done\" and .status != \"skipped\" and .status != \"cancelled\"))] | length" 2>/dev/null || echo "1")
  [ "${remaining:-1}" -eq 0 ]
}

wave_all_done() {
  local wave_db_id="$1"
  _validate_id "$wave_db_id" || return 1
  local json remaining
  json=$(cvg plan show "$PLAN_ID" 2>/dev/null) || json=$(_api_get "/api/plan-db/json/${PLAN_ID}") || { return 1; }
  remaining=$(echo "$json" | jq -r "[.tasks[]? | select(.wave_id_fk == ${wave_db_id} and (.status != \"done\" and .status != \"skipped\" and .status != \"cancelled\"))] | length" 2>/dev/null || echo "1")
  [ "${remaining:-1}" -eq 0 ]
}

# ─── execution_runs wiring (daemon API) ───────────────────────────────

_daemon_up() { curl -sf --max-time 2 "${DAEMON_URL}/api/health" >/dev/null 2>&1; }

# Resolve active run_id for a plan via daemon API
_active_run_id() {
  local pid="$1"
  local json
  json=$(_api_get "/api/runs") || { echo ""; return; }
  echo "$json" | jq -r "[.runs[]? | select(.plan_id == ${pid} and .status == \"running\")] | sort_by(.started_at) | last | .id // empty" 2>/dev/null || echo ""
}

# INSERT a new run row when a plan starts; idempotent if already running.
execution_runs_start() {
  local pid="$1"
  _validate_id "$pid" || return 1
  local goal="${PLAN_GOAL:-plan $pid}"

  _api_post "/api/runs" "{\"goal\":\"$goal\",\"plan_id\":$pid}" || {
    log "EXEC_RUNS: failed to start run for plan $pid"
    return 1
  }
  log "EXEC_RUNS: run started via API for plan $pid"
}

# UPDATE cost + agents after each wave
execution_runs_update_wave() {
  local pid="$1"
  _validate_id "$pid" || return 1

  local run_id
  run_id=$(_active_run_id "$pid")
  [ -z "$run_id" ] && { log "EXEC_RUNS: no active run for plan $pid — skip update"; return 0; }
  _validate_id "$run_id" || return 1

  # Get cost and agents from metrics API
  local metrics_json
  metrics_json=$(_api_get "/api/metrics/run/${run_id}") || metrics_json=""
  local cost agents
  cost=$(echo "$metrics_json" | jq -r '.cost_usd // 0' 2>/dev/null || echo "0")
  agents=$(echo "$metrics_json" | jq -r '.agents // ""' 2>/dev/null || echo "")

  _api_put "/api/runs/$run_id" "{\"cost_usd\":${cost:-0},\"agents_used\":\"${agents:-}\"}" || true
  log "EXEC_RUNS: wave metrics updated via API for plan $pid (run $run_id)"
}

# SET status='completed' when plan finishes.
execution_runs_complete() {
  local pid="$1"
  _validate_id "$pid" || return 1
  local completed_at
  completed_at=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

  local run_id
  run_id=$(_active_run_id "$pid")
  [ -z "$run_id" ] && { log "EXEC_RUNS: no active run to complete for plan $pid"; return 0; }
  _validate_id "$run_id" || return 1
  _api_put "/api/runs/$run_id" "{\"status\":\"completed\",\"completed_at\":\"$completed_at\"}" || true
  log "EXEC_RUNS: run completed via API for plan $pid (run $run_id)"
}

# ─── Trigger Actions ─────────────────────────────────────────────────

trigger_execution() {
  local pid="$1" wave_db_id="$2" wave_id="$3"
  log "DISPATCH: Executing wave $wave_id (plan $pid)"
  "$BUS" register "autopilot" "auto-executor" "system" 2>/dev/null || true
  "$BUS" send "autopilot" "general" "Auto-dispatching wave $wave_id for plan $pid" 2>/dev/null || true
  # Update wave status via daemon API
  _api_post "/api/plan-db/wave/update" "{\"wave_id\":${wave_db_id},\"status\":\"in_progress\"}" 2>/dev/null || true
  execution_runs_start "$pid"
  log "  Spawning executor for plan $pid..."
  if command -v claude &>/dev/null; then
    claude -p "Esegui /execute $pid. Focus su wave $wave_id. Modalita autonoma." &
    log "  Executor PID: $!"
  else
    warn "Claude CLI not found — manual execution needed"
  fi
}

trigger_thor() {
  local pid="$1" wave_db_id="$2" wave_id="$3"
  log "THOR: Validating wave $wave_id (plan $pid)"
  "$BUS" send "autopilot" "general" "Thor validation starting for wave $wave_id" 2>/dev/null || true
  execution_runs_update_wave "$pid"
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
  cvg plan calibrate-estimates 2>/dev/null || true
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
