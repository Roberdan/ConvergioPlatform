#!/usr/bin/env bash
# demo-hyperworker.sh — Simulates realistic task execution for hyperDemo plans
# Progresses tasks: pending → in_progress → submitted → done (Thor validated)
set -euo pipefail

_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../../config/load-config.sh
source "$_SCRIPT_DIR/../../config/load-config.sh" 2>/dev/null || true
unset _SCRIPT_DIR

command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 1; }

DAEMON_API="http://localhost:8420"
IFS=',' read -ra HOSTS <<< "${MESH_WORKERS:-worker-1,worker-2,worker-3}"
MODELS=("gpt-5-mini")

log() { echo "[$(date +%H:%M:%S)] $*"; }
rand() { echo $(( RANDOM % ($2 - $1 + 1) + $1 )); }
pick() { local arr=("$@"); echo "${arr[RANDOM % ${#arr[@]}]}"; }

# Use daemon API for task updates
_api_task_update() {
  local task_id="$1" status="$2"
  shift 2
  curl -sf -X POST "${DAEMON_API}/api/plan-db/task/update" \
    -H 'Content-Type: application/json' \
    -d "{\"task_id\":${task_id},\"status\":\"${status}\"$([ $# -gt 0 ] && echo ",\"extra\":$*" || echo '')}" 2>/dev/null || true
}

_api_plan_update() {
  local plan_id="$1" status="$2"
  curl -sf -X POST "${DAEMON_API}/api/plan-status" \
    -H 'Content-Type: application/json' \
    -d "{\"plan_id\":${plan_id},\"status\":\"${status}\"}" 2>/dev/null || true
}

TASK_CTR=0
complete_task() {
  local task_id=$1 plan_id=$2
  local tokens=$(rand 5000 45000)
  local lines=$(rand 20 350)
  local duration=$(rand 30 600)
  local model=$(pick "${MODELS[@]}")
  # Round-robin host distribution
  local host="${HOSTS[$((TASK_CTR % 3))]}"
  TASK_CTR=$((TASK_CTR + 1))

  log "  Task $task_id → in_progress ($host/$model)"
  cvg task update "$task_id" in_progress "Started by hyperworker on $host" 2>/dev/null || true

  sleep $(rand 4 10)

  log "  Task $task_id → submitted (${lines}L, ${tokens}tok)"
  cvg task update "$task_id" submitted "Done: ${lines}L, ${tokens}tok" 2>/dev/null || true

  sleep $(rand 2 5)

  log "  Task $task_id → done"
  cvg task update "$task_id" done "Validated by thor" 2>/dev/null || true
}

process_wave() {
  local plan_id=$1 wave=$2
  local tasks
  tasks="$(cvg plan show "$plan_id" 2>/dev/null | jq -r --arg w "$wave" '.tasks[] | select(.wave_id == $w and (.status == "pending" or .status == "in_progress")) | .id' 2>/dev/null || echo '')"
  [ -z "$tasks" ] && return
  log "Plan $plan_id / $wave — $(echo "$tasks" | wc -l | tr -d ' ') tasks"
  for tid in $tasks; do
    complete_task "$tid" "$plan_id" &
  done
  wait
}

main() {
  log "HyperDemo Worker starting..."

  local plan_ids
  plan_ids="$(curl -sf "${DAEMON_API}/api/plan-db/list" 2>/dev/null | jq -r '.[] | select(.project_id == "hyperDemo" and .status == "doing") | .id' 2>/dev/null || echo '')"
  [ -z "$plan_ids" ] && { log "No active plans"; exit 0; }

  # Process in staggered batches of 5 plans — more realistic pacing
  local batch_size=5
  local plan_arr=($plan_ids)
  local total=${#plan_arr[@]}

  for wave in W1; do
    log "━━━ $wave across $total plans ━━━"
    local i=0
    while [ $i -lt $total ]; do
      local end=$((i + batch_size))
      [ $end -gt $total ] && end=$total
      log "  Batch $((i/batch_size + 1)): plans ${plan_arr[$i]}..${plan_arr[$((end-1))]}"
      for j in $(seq $i $((end - 1))); do
        process_wave "${plan_arr[$j]}" "$wave" &
      done
      wait
      i=$end
      sleep 3
    done
    log "✓ $wave complete"
  done

  # Complete plans one by one with small delay
  for pid in $plan_ids; do
    local rem
    rem="$(cvg plan show "$pid" 2>/dev/null | jq '[.tasks[] | select(.status != "done")] | length' 2>/dev/null || echo 1)"
    if [ "${rem:-0}" -eq 0 ]; then
      _api_plan_update "$pid" "done"
      log "Plan $pid COMPLETE"
      sleep 1
    fi
  done

  log "Done!"
}

main "$@"
