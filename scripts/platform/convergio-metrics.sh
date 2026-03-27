#!/usr/bin/env bash
# convergio-metrics.sh — Telemetry collector + per-run analytics
# Collects system + agent metrics and writes via daemon API
# Run via: convergio-autopilot.sh watch (includes metrics) or cron
set -euo pipefail

PLATFORM_DIR="${CONVERGIO_PLATFORM_DIR:-$HOME/GitHub/ConvergioPlatform}"
DAEMON_URL="${CONVERGIO_DAEMON_URL:-http://localhost:8420}"

command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 1; }
command -v curl &>/dev/null || { echo "ERROR: curl required" >&2; exit 1; }

_validate_id() { [[ "$1" =~ ^[0-9]+$ ]] || { echo "Invalid ID: $1 (must be numeric)" >&2; exit 1; }; }
_validate_days() { [[ "$1" =~ ^[0-9]+$ ]] || { echo "Invalid days value: $1 (must be numeric)" >&2; exit 1; }; }

# _api PATH — curl daemon; returns JSON; empty on error
_api_get() { curl -sf --connect-timeout 2 "${DAEMON_URL}${1}" 2>/dev/null; }
_api_post() { curl -sf -X POST "${DAEMON_URL}${1}" -H 'Content-Type: application/json' -d "$2" 2>/dev/null; }

# _daemon_up — true if daemon responds to health check
_daemon_up() { curl -sf --connect-timeout 2 "${DAEMON_URL}/api/metrics/summary" > /dev/null 2>&1; }

collect_system() {
  local cpu mem
  cpu=$(ps -A -o %cpu | awk '{s+=$1} END {printf "%.1f", s}')
  mem=$(vm_stat 2>/dev/null | awk '/Pages active/ {gsub(/\./,"",$3); print $3*4096/1048576}' || echo "0")

  _api_post "/api/tracking/tokens" \
    "[{\"project_id\":\"system\",\"metric_name\":\"cpu.percent\",\"metric_value\":${cpu}},{\"project_id\":\"system\",\"metric_name\":\"memory.active_mb\",\"metric_value\":${mem:-0}}]" 2>/dev/null || true
  echo "  cpu=${cpu}% mem=${mem:-0}MB"
}

collect_agents() {
  local count=0
  curl -sf --connect-timeout 1 "$DAEMON_URL/api/ipc/status" > /dev/null 2>&1 && \
    count=$(curl -sf "$DAEMON_URL/api/ipc/agents" | jq '[.agents // []] | first | length' 2>/dev/null || echo "0")
  _api_post "/api/tracking/tokens" \
    "[{\"project_id\":\"agents\",\"metric_name\":\"active_count\",\"metric_value\":${count}}]" 2>/dev/null || true
  echo "  active_agents=$count"
}

collect_plans() {
  local json
  json=$(_api_get "/api/overview") || { echo "  (daemon not reachable)"; return 0; }

  local active done total_tasks done_tasks
  active=$(echo "$json" | jq -r '.active_plans | length // 0' 2>/dev/null || echo "0")
  done=$(echo "$json" | jq -r '.completed_plans_count // 0' 2>/dev/null || echo "0")
  total_tasks=$(echo "$json" | jq -r '.total_tasks // 0' 2>/dev/null || echo "0")
  done_tasks=$(echo "$json" | jq -r '.done_tasks // 0' 2>/dev/null || echo "0")

  _api_post "/api/tracking/tokens" \
    "[{\"project_id\":\"plans\",\"metric_name\":\"active\",\"metric_value\":${active:-0}},{\"project_id\":\"plans\",\"metric_name\":\"completed\",\"metric_value\":${done:-0}},{\"project_id\":\"tasks\",\"metric_name\":\"total\",\"metric_value\":${total_tasks:-0}},{\"project_id\":\"tasks\",\"metric_name\":\"done\",\"metric_value\":${done_tasks:-0}}]" 2>/dev/null || true
  echo "  plans_active=${active:-0} plans_done=${done:-0} tasks=${done_tasks:-0}/${total_tasks:-0}"
}

collect_mesh() {
  local peers=0
  if [ -f "$HOME/.claude/config/peers.conf" ]; then
    peers=$(grep -c '^\[node' "$HOME/.claude/config/peers.conf" 2>/dev/null || echo "0")
  fi
  _api_post "/api/tracking/tokens" \
    "[{\"project_id\":\"mesh\",\"metric_name\":\"peer_count\",\"metric_value\":${peers}}]" 2>/dev/null || true
  echo "  mesh_peers=$peers"
}

collect_learnings() {
  local json
  json=$(_api_get "/api/overview") || { echo "  (daemon not reachable)"; return 0; }

  local kb_count skill_count learning_count
  kb_count=$(echo "$json" | jq -r '.knowledge_base_count // 0' 2>/dev/null || echo "0")
  skill_count=$(echo "$json" | jq -r '.agent_skills_count // 0' 2>/dev/null || echo "0")
  learning_count=$(echo "$json" | jq -r '.plan_learnings_count // 0' 2>/dev/null || echo "0")

  _api_post "/api/tracking/tokens" \
    "[{\"project_id\":\"knowledge\",\"metric_name\":\"kb_entries\",\"metric_value\":${kb_count:-0}},{\"project_id\":\"knowledge\",\"metric_name\":\"skills\",\"metric_value\":${skill_count:-0}},{\"project_id\":\"knowledge\",\"metric_name\":\"learnings\",\"metric_value\":${learning_count:-0}}]" 2>/dev/null || true
  echo "  kb=${kb_count:-0} skills=${skill_count:-0} learnings=${learning_count:-0}"
}

cmd_collect() {
  echo "[$(date '+%H:%M:%S')] Collecting metrics..."
  collect_system
  collect_agents
  collect_plans
  collect_mesh
  collect_learnings
  echo "  Done."
}

cmd_report() {
  echo "=== Metrics Report (last 24h) ==="
  local json
  if _daemon_up; then
    json=$(_api_get "/api/metrics/summary") || json=""
    if [ -n "$json" ] && command -v jq > /dev/null 2>&1; then
      printf "  %-30s %-10s %-10s %s\n" "METRIC" "AVG" "MAX" "SAMPLES"
      echo "  ────────────────────────────── ────────── ────────── ───────"
      printf '%s' "$json" | jq -r '
        .metrics[]? |
        [.name // "?", (.avg // "N/A" | tostring), (.max // "N/A" | tostring), (.samples // "N/A" | tostring)] |
        @tsv' 2>/dev/null | while IFS=$'\t' read -r name avg max samples; do
        printf "  %-30s %-10s %-10s %s\n" "$name" "$avg" "$max" "$samples"
      done
      return
    elif [ -n "$json" ]; then
      printf '%s\n' "$json"
      return
    fi
  fi
  echo "  Daemon not reachable — no metrics available"
}

cmd_clean() {
  local days="${1:-30}"
  _validate_days "$days"
  # TODO: needs daemon endpoint for metrics_history cleanup
  _api_post "/api/metrics/clean" "{\"older_than_days\":${days}}" 2>/dev/null || {
    echo "Cleanup endpoint not available" >&2
    return 1
  }
  echo "Cleaned metrics older than $days days"
}

cmd_run() {
  local run_id="${1:-}"
  if [ -z "$run_id" ]; then
    echo "Usage: convergio-metrics.sh run <run_id>" >&2
    exit 1
  fi
  _validate_id "$run_id"

  echo "=== Run #${run_id} ==="

  local json
  json=$(_api_get "/api/metrics/run/${run_id}") || json=""
  if [ -n "$json" ] && command -v jq > /dev/null 2>&1; then
    printf "  %-12s %s\n" "Goal:"       "$(printf '%s' "$json" | jq -r '.goal      // "N/A"')"
    printf "  %-12s %s\n" "Status:"     "$(printf '%s' "$json" | jq -r '.status    // "N/A"')"
    printf "  %-12s %s\n" "Plan ID:"    "$(printf '%s' "$json" | jq -r '.plan_id   // "N/A"')"
    printf "  %-12s %s\n" "Started:"    "$(printf '%s' "$json" | jq -r '.started_at // "N/A"')"
    printf "  %-12s %s\n" "Completed:"  "$(printf '%s' "$json" | jq -r '.completed_at // "N/A"')"
    printf "  %-12s %s\n" "Duration:"   "$(printf '%s' "$json" | jq -r '(.duration_minutes | tostring) + "m" // "N/A"')"
    printf "  %-12s %s\n" "Cost:"       "\$$(printf '%s' "$json" | jq -r '.cost_usd // "0.00"') USD"
    printf "  %-12s %s\n" "Agents:"     "$(printf '%s' "$json" | jq -r '.agents // "N/A"')"
    printf "  %-12s %s\n" "Tasks:"      "$(printf '%s' "$json" | jq -r '(.tasks_done|tostring) + "/" + (.tasks_total|tostring) + " completed"')"
    printf "  %-12s %s\n" "Val pass:"   "$(printf '%s' "$json" | jq -r '.validation_pass_rate // "N/A"')"
    return
  elif [ -n "$json" ]; then
    printf '%s\n' "$json"
    return
  fi

  echo "  Run ${run_id} not found or daemon not reachable." >&2
  exit 1
}

cmd_runs() {
  echo "=== Execution Runs ==="
  printf "  %-5s %-10s %-8s %-40s %s\n" "ID" "PLAN" "STATUS" "GOAL" "STARTED"
  echo "  ───── ────────── ──────── ──────────────────────────────────────── ───────────────────"

  local json
  json=$(_api_get "/api/runs") || json=""
  if [ -n "$json" ] && command -v jq > /dev/null 2>&1; then
    local count
    count=$(printf '%s' "$json" | jq -r '.runs // [] | length')
    if [ "$count" -eq 0 ]; then
      echo "  No execution runs found."
      return
    fi
    printf '%s' "$json" | jq -r '
      .runs[]? |
      [(.id | tostring), (.plan_id // "N/A" | tostring), (.status // "N/A"),
       ((.goal // "N/A") | .[0:40]), (.started_at // "N/A")] |
      @tsv' 2>/dev/null | while IFS=$'\t' read -r id plan_id status goal started; do
      printf "  %-5s %-10s %-8s %-40s %s\n" "$id" "$plan_id" "$status" "$goal" "$started"
    done
    return
  fi

  echo "  Daemon not reachable — no runs available"
}

case "${1:-collect}" in
  collect) cmd_collect ;;
  report)  cmd_report ;;
  clean)   shift; cmd_clean "${1:-30}" ;;
  run)     shift; cmd_run "${1:-}" ;;
  runs)    cmd_runs ;;
  *)
    echo "convergio-metrics.sh — Telemetry collector + per-run analytics"
    echo "  collect        Collect current metrics"
    echo "  report         Show 24h metric report"
    echo "  clean [days]   Remove old metrics (default: 30 days)"
    echo "  run <id>       Per-run: duration, cost, agents, tasks, validation"
    echo "  runs           List execution runs with summary stats"
    ;;
esac
