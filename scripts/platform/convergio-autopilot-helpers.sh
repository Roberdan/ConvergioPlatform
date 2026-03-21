#!/usr/bin/env bash
# convergio-autopilot-helpers.sh — Helper functions for convergio-autopilot.sh
# Sourced by convergio-autopilot.sh — do NOT execute directly.
# Provides: plan discovery, wave state machine, trigger_*, execution_runs wiring
set -uo pipefail

_validate_id() { [[ "$1" =~ ^[0-9]+$ ]] || { log "SECURITY: invalid ID '$1' — aborting"; return 1; }; }

# ─── Plan Discovery ─────────────────────────────────────────────────

find_actionable_plan() {
  if [ -n "${PLAN_ID:-}" ]; then
    echo "$PLAN_ID"
  else
    _db "SELECT p.id FROM plans p
         WHERE p.status = 'doing'
         AND EXISTS (
           SELECT 1 FROM waves w
           WHERE w.plan_id = p.id AND w.status IN ('pending','in_progress')
         )
         ORDER BY p.id DESC LIMIT 1;"
  fi
}

# ─── Pause Bridge (B3 fix) ───────────────────────────────────────────
# Reads coordinator_events with event_type='pause_run' and flips
# execution_runs.status='paused' for the targeted plan.

apply_pause_events() {
  local unprocessed
  unprocessed=$(_db "SELECT id, plan_id FROM coordinator_events
                     WHERE event_type = 'pause_run'
                     AND processed = 0
                     ORDER BY created_at ASC;" 2>/dev/null || true)
  [ -z "$unprocessed" ] && return 0

  while IFS='|' read -r ev_id ev_plan_id; do
    [ -z "$ev_id" ] && continue
    _validate_id "$ev_id" || continue
    _validate_id "$ev_plan_id" || continue
    log "PAUSE_BRIDGE: pause_run event $ev_id for plan $ev_plan_id"
    _db "UPDATE execution_runs SET status = 'paused'
         WHERE plan_id = $ev_plan_id AND status = 'running';"
    _db "UPDATE coordinator_events SET processed = 1
         WHERE id = $ev_id;"
  done <<< "$unprocessed"
}

# ─── Wave State Machine ─────────────────────────────────────────────

get_current_wave() {
  local pid="$1"
  _validate_id "$pid" || return 1
  _db "SELECT id, wave_id, status, tasks_done, tasks_total
       FROM waves WHERE plan_id = $pid AND status IN ('pending','in_progress','merging')
       ORDER BY position LIMIT 1;"
}

count_pending_tasks() {
  local wave_db_id="$1"
  _validate_id "$wave_db_id" || return 1
  _db "SELECT count(*) FROM tasks WHERE wave_id_fk = $wave_db_id AND status = 'pending';"
}

count_submitted_tasks() {
  local wave_db_id="$1"
  _validate_id "$wave_db_id" || return 1
  _db "SELECT count(*) FROM tasks WHERE wave_id_fk = $wave_db_id AND status = 'submitted';"
}

count_in_progress_tasks() {
  local wave_db_id="$1"
  _validate_id "$wave_db_id" || return 1
  _db "SELECT count(*) FROM tasks WHERE wave_id_fk = $wave_db_id AND status = 'in_progress';"
}

all_tasks_submitted_or_done() {
  local wave_db_id="$1"
  _validate_id "$wave_db_id" || return 1
  local remaining
  remaining=$(_db "SELECT count(*) FROM tasks
                   WHERE wave_id_fk = $wave_db_id
                   AND status NOT IN ('submitted','done','skipped','cancelled');")
  [ "${remaining:-1}" -eq 0 ]
}

wave_all_done() {
  local wave_db_id="$1"
  _validate_id "$wave_db_id" || return 1
  local remaining
  remaining=$(_db "SELECT count(*) FROM tasks
                   WHERE wave_id_fk = $wave_db_id
                   AND status NOT IN ('done','skipped','cancelled');")
  [ "${remaining:-1}" -eq 0 ]
}

# ─── execution_runs wiring ───────────────────────────────────────────

# INSERT a new run row when a plan starts execution.
# Idempotent: skips if a 'running' row already exists for this plan.
execution_runs_start() {
  local pid="$1"
  _validate_id "$pid" || return 1
  local existing
  existing=$(_db "SELECT count(*) FROM execution_runs
                  WHERE plan_id = $pid AND status = 'running';" 2>/dev/null || echo "0")
  if [ "${existing:-0}" -eq 0 ]; then
    _db "INSERT INTO execution_runs (plan_id, status, started_at)
         VALUES ($pid, 'running', datetime('now'));" 2>/dev/null || true
    log "EXEC_RUNS: run started for plan $pid"
  fi
}

# UPDATE cost + agents from delegation_log using plan_id + timestamp window.
# B4: no run_id FK — join by plan_id and rows since run started_at.
execution_runs_update_wave() {
  local pid="$1"
  _validate_id "$pid" || return 1
  _db "UPDATE execution_runs
       SET cost_usd = (
         SELECT COALESCE(SUM(d.cost_usd), 0)
         FROM delegation_log d
         WHERE d.plan_id = $pid
           AND d.created_at >= (
             SELECT started_at FROM execution_runs
             WHERE plan_id = $pid AND status = 'running'
             ORDER BY started_at DESC LIMIT 1
           )
       ),
       agents_used = (
         SELECT COALESCE(GROUP_CONCAT(DISTINCT d.agent_name), '')
         FROM delegation_log d
         WHERE d.plan_id = $pid
           AND d.created_at >= (
             SELECT started_at FROM execution_runs
             WHERE plan_id = $pid AND status = 'running'
             ORDER BY started_at DESC LIMIT 1
           )
       )
       WHERE plan_id = $pid AND status = 'running';" 2>/dev/null || true
}

# SET status='completed' when plan finishes.
execution_runs_complete() {
  local pid="$1"
  _validate_id "$pid" || return 1
  _db "UPDATE execution_runs
       SET status = 'completed', completed_at = datetime('now')
       WHERE plan_id = $pid AND status = 'running';" 2>/dev/null || true
  log "EXEC_RUNS: run completed for plan $pid"
}

# ─── Trigger Actions ─────────────────────────────────────────────────

trigger_execution() {
  local pid="$1" wave_db_id="$2" wave_id="$3"
  log "DISPATCH: Executing wave $wave_id (plan $pid)"

  "$BUS" register "autopilot" "auto-executor" "system" 2>/dev/null || true
  "$BUS" send "autopilot" "general" "Auto-dispatching wave $wave_id for plan $pid" 2>/dev/null || true

  _db "UPDATE waves SET status = 'in_progress' WHERE id = $wave_db_id AND status = 'pending';"

  # Wire execution_runs: INSERT on plan start
  execution_runs_start "$pid"

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

  # UPDATE cost/agents on wave complete (join delegation_log by plan_id + timestamp window)
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
