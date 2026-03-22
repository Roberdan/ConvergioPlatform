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

# Pause bridge handled by daemon background task (Plan 679 T2-02)

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

# ─── execution_runs wiring (daemon API + sqlite3 fallback) ───────────

_daemon_up() { curl -sf --max-time 2 http://localhost:8420/api/health >/dev/null 2>&1; }

# Resolve active run_id for a plan (local DB lookup shared by update + complete)
_active_run_id() {
  local pid="$1"
  _db "SELECT id FROM execution_runs
       WHERE plan_id = $pid AND status = 'running'
       ORDER BY started_at DESC LIMIT 1;" 2>/dev/null || true
}

# INSERT a new run row when a plan starts; idempotent if already running.
execution_runs_start() {
  local pid="$1"
  _validate_id "$pid" || return 1
  local goal="${PLAN_GOAL:-plan $pid}"

  if _daemon_up; then
    curl -s -X POST http://localhost:8420/api/runs \
      -H 'Content-Type: application/json' \
      -d "{\"goal\":\"$goal\",\"plan_id\":$pid}" 2>/dev/null || true
    log "EXEC_RUNS: run started via API for plan $pid"
  else
    local existing
    existing=$(_db "SELECT count(*) FROM execution_runs
                    WHERE plan_id = $pid AND status = 'running';" 2>/dev/null || echo "0")
    if [ "${existing:-0}" -eq 0 ]; then
      _db "INSERT INTO execution_runs (plan_id, status, started_at)
           VALUES ($pid, 'running', datetime('now'));" 2>/dev/null || true
      log "EXEC_RUNS: run started (fallback sqlite3) for plan $pid"
    fi
  fi
}

# UPDATE cost + agents after each wave; uses delegation_log since run started_at.
execution_runs_update_wave() {
  local pid="$1"
  _validate_id "$pid" || return 1

  # SQL subquery reused in both daemon and fallback paths
  local _run_start_sql="SELECT started_at FROM execution_runs
                        WHERE plan_id = $pid AND status = 'running'
                        ORDER BY started_at DESC LIMIT 1"

  if _daemon_up; then
    local run_id cost agents
    run_id=$(_active_run_id "$pid")
    [ -z "$run_id" ] && { log "EXEC_RUNS: no active run for plan $pid — skip update"; return 0; }
    _validate_id "$run_id" || return 1
    cost=$(_db "SELECT COALESCE(SUM(cost_usd),0) FROM delegation_log
                WHERE plan_id=$pid AND created_at>=($_run_start_sql);" 2>/dev/null || echo "0")
    agents=$(_db "SELECT COALESCE(GROUP_CONCAT(DISTINCT agent_name),'') FROM delegation_log
                  WHERE plan_id=$pid AND created_at>=($_run_start_sql);" 2>/dev/null || echo "")
    curl -s -X PUT "http://localhost:8420/api/runs/$run_id" \
      -H 'Content-Type: application/json' \
      -d "{\"cost_usd\":${cost:-0},\"agents_used\":\"${agents:-}\"}" 2>/dev/null || true
    log "EXEC_RUNS: wave metrics updated via API for plan $pid (run $run_id)"
  else
    _db "UPDATE execution_runs SET
         cost_usd=(SELECT COALESCE(SUM(cost_usd),0) FROM delegation_log
                   WHERE plan_id=$pid AND created_at>=($_run_start_sql)),
         agents_used=(SELECT COALESCE(GROUP_CONCAT(DISTINCT agent_name),'') FROM delegation_log
                      WHERE plan_id=$pid AND created_at>=($_run_start_sql))
         WHERE plan_id=$pid AND status='running';" 2>/dev/null || true
    log "EXEC_RUNS: wave metrics updated (fallback sqlite3) for plan $pid"
  fi
}

# SET status='completed' when plan finishes.
execution_runs_complete() {
  local pid="$1"
  _validate_id "$pid" || return 1
  local completed_at
  completed_at=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

  if _daemon_up; then
    local run_id
    run_id=$(_active_run_id "$pid")
    [ -z "$run_id" ] && { log "EXEC_RUNS: no active run to complete for plan $pid"; return 0; }
    _validate_id "$run_id" || return 1
    curl -s -X PUT "http://localhost:8420/api/runs/$run_id" \
      -H 'Content-Type: application/json' \
      -d "{\"status\":\"completed\",\"completed_at\":\"$completed_at\"}" 2>/dev/null || true
    log "EXEC_RUNS: run completed via API for plan $pid (run $run_id)"
  else
    _db "UPDATE execution_runs SET status='completed', completed_at=datetime('now')
         WHERE plan_id=$pid AND status='running';" 2>/dev/null || true
    log "EXEC_RUNS: run completed (fallback sqlite3) for plan $pid"
  fi
}

# ─── Trigger Actions ─────────────────────────────────────────────────

trigger_execution() {
  local pid="$1" wave_db_id="$2" wave_id="$3"
  log "DISPATCH: Executing wave $wave_id (plan $pid)"
  "$BUS" register "autopilot" "auto-executor" "system" 2>/dev/null || true
  "$BUS" send "autopilot" "general" "Auto-dispatching wave $wave_id for plan $pid" 2>/dev/null || true
  _db "UPDATE waves SET status = 'in_progress' WHERE id = $wave_db_id AND status = 'pending';"
  execution_runs_start "$pid"
  log "  Spawning executor for plan $pid..."
  if command -v claude &>/dev/null; then
    claude -p "Esegui /execute $pid. Focus su wave $wave_id. Modalità autonoma." &
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
