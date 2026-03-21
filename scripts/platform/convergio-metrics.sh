#!/usr/bin/env bash
# convergio-metrics.sh — Telemetry collector + per-run analytics
# Collects system + agent metrics and writes to metrics_history table
# Run via: convergio-autopilot.sh watch (includes metrics) or cron
set -euo pipefail

PLATFORM_DIR="${CONVERGIO_PLATFORM_DIR:-$HOME/GitHub/ConvergioPlatform}"
DB="${DASHBOARD_DB:-$PLATFORM_DIR/data/dashboard.db}"
DAEMON_URL="${CONVERGIO_DAEMON_URL:-http://localhost:8420}"

_db() { sqlite3 "$DB" "$1" 2>/dev/null; }
_validate_id() { [[ "$1" =~ ^[0-9]+$ ]] || { echo "Invalid ID: $1 (must be numeric)" >&2; exit 1; }; }
_validate_days() { [[ "$1" =~ ^[0-9]+$ ]] || { echo "Invalid days value: $1 (must be numeric)" >&2; exit 1; }; }

collect_system() {
  local cpu mem
  cpu=$(ps -A -o %cpu | awk '{s+=$1} END {printf "%.1f", s}')
  mem=$(vm_stat 2>/dev/null | awk '/Pages active/ {gsub(/\./,"",$3); print $3*4096/1048576}' || echo "0")

  _db "INSERT INTO metrics_history (project_id, metric_name, metric_value) VALUES
    ('system', 'cpu.percent', $cpu),
    ('system', 'memory.active_mb', ${mem:-0});"
  echo "  cpu=${cpu}% mem=${mem:-0}MB"
}

collect_agents() {
  local count=0
  if curl -sf --connect-timeout 1 "$DAEMON_URL/api/ipc/status" > /dev/null 2>&1; then
    count=$(curl -sf "$DAEMON_URL/api/ipc/agents" | python3 -c "import sys,json; print(len(json.load(sys.stdin).get('agents',[])))" 2>/dev/null || echo "0")
  fi
  _db "INSERT INTO metrics_history (project_id, metric_name, metric_value) VALUES
    ('agents', 'active_count', $count);"
  echo "  active_agents=$count"
}

collect_plans() {
  local active done total_tasks done_tasks
  active=$(_db "SELECT count(*) FROM plans WHERE status = 'doing';")
  done=$(_db "SELECT count(*) FROM plans WHERE status = 'done';")
  total_tasks=$(_db "SELECT count(*) FROM tasks;")
  done_tasks=$(_db "SELECT count(*) FROM tasks WHERE status = 'done';")

  _db "INSERT INTO metrics_history (project_id, metric_name, metric_value) VALUES
    ('plans', 'active', ${active:-0}),
    ('plans', 'completed', ${done:-0}),
    ('tasks', 'total', ${total_tasks:-0}),
    ('tasks', 'done', ${done_tasks:-0});"
  echo "  plans_active=${active:-0} plans_done=${done:-0} tasks=${done_tasks:-0}/${total_tasks:-0}"
}

collect_mesh() {
  local peers=0
  if [ -f "$HOME/.claude/config/peers.conf" ]; then
    peers=$(grep -c '^\[node' "$HOME/.claude/config/peers.conf" 2>/dev/null || echo "0")
  fi
  _db "INSERT INTO metrics_history (project_id, metric_name, metric_value) VALUES
    ('mesh', 'peer_count', $peers);"
  echo "  mesh_peers=$peers"
}

collect_learnings() {
  local kb_count skill_count learning_count
  kb_count=$(_db "SELECT count(*) FROM knowledge_base;")
  skill_count=$(_db "SELECT count(*) FROM agent_skills;")
  learning_count=$(_db "SELECT count(*) FROM plan_learnings;")

  _db "INSERT INTO metrics_history (project_id, metric_name, metric_value) VALUES
    ('knowledge', 'kb_entries', ${kb_count:-0}),
    ('knowledge', 'skills', ${skill_count:-0}),
    ('knowledge', 'learnings', ${learning_count:-0});"
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
  _db "SELECT metric_name, round(avg(metric_value),1) as avg, round(max(metric_value),1) as max, count(*) as samples
       FROM metrics_history
       WHERE recorded_at > datetime('now', '-1 day')
       GROUP BY metric_name
       ORDER BY metric_name;" | while IFS='|' read -r name avg max samples; do
    printf "  %-30s avg=%-8s max=%-8s (%s samples)\n" "$name" "$avg" "$max" "$samples"
  done
}

cmd_clean() {
  local days="${1:-30}"
  _validate_days "$days"
  _db "DELETE FROM metrics_history WHERE recorded_at < datetime('now', '-$days days');"
  echo "Cleaned metrics older than $days days"
}

cmd_run() {
  local run_id="${1:-}"
  if [ -z "$run_id" ]; then
    echo "Usage: convergio-metrics.sh run <run_id>" >&2
    exit 1
  fi
  _validate_id "$run_id"

  # Fetch the execution run row
  local row
  row=$(_db "SELECT id, goal, status, plan_id, started_at, completed_at, duration_minutes FROM execution_runs WHERE id = $run_id;")
  if [ -z "$row" ]; then
    echo "Run $run_id not found." >&2
    exit 1
  fi

  local run_goal run_status run_plan_id run_started run_completed run_duration
  IFS='|' read -r _ run_goal run_status run_plan_id run_started run_completed run_duration <<< "$row"

  # Duration: prefer stored value, else compute from timestamps
  local duration_str="N/A"
  if [ -n "$run_duration" ] && [ "$run_duration" != "NULL" ]; then
    duration_str="${run_duration}m"
  elif [ -n "$run_started" ] && [ -n "$run_completed" ] && [ "$run_completed" != "NULL" ]; then
    local mins
    mins=$(_db "SELECT round((julianday('$run_completed') - julianday('$run_started')) * 1440, 1) FROM execution_runs WHERE id = $run_id;")
    duration_str="${mins}m"
  fi

  # Cost: sum delegation_log.cost_estimate for same plan_id within run window
  local cost="0.00"
  if [ -n "$run_plan_id" ] && [ "$run_plan_id" != "NULL" ]; then
    local end_bound="${run_completed:-$(date -u '+%Y-%m-%d %H:%M:%S')}"
    cost=$(_db "SELECT printf('%.4f', coalesce(sum(cost_estimate),0))
      FROM delegation_log
      WHERE plan_id = $run_plan_id
        AND created_at >= '$run_started'
        AND created_at <= '$end_bound';")
  fi

  # Distinct agents used (executor_agent from tasks with the same plan_id)
  local agents="N/A"
  if [ -n "$run_plan_id" ] && [ "$run_plan_id" != "NULL" ]; then
    agents=$(_db "SELECT coalesce(group_concat(DISTINCT executor_agent), 'none')
      FROM tasks WHERE plan_id = $run_plan_id AND executor_agent IS NOT NULL;")
  fi

  # Tasks completed / total for the plan
  local tasks_total="0" tasks_done="0" val_pass="N/A"
  if [ -n "$run_plan_id" ] && [ "$run_plan_id" != "NULL" ]; then
    tasks_total=$(_db "SELECT count(*) FROM tasks WHERE plan_id = $run_plan_id;")
    tasks_done=$(_db "SELECT count(*) FROM tasks WHERE plan_id = $run_plan_id AND status IN ('done','submitted');")
    # Validation pass rate: tasks with non-null validation_report containing 'PASS'
    local val_total val_passed
    val_total=$(_db "SELECT count(*) FROM tasks WHERE plan_id = $run_plan_id AND validation_report IS NOT NULL;")
    val_passed=$(_db "SELECT count(*) FROM tasks WHERE plan_id = $run_plan_id AND validation_report LIKE '%PASS%';")
    if [ "${val_total:-0}" -gt 0 ]; then
      val_pass=$(_db "SELECT printf('%d%%', round(100.0 * $val_passed / $val_total));" )
    fi
  fi

  echo "=== Run #${run_id} ==="
  echo "  Goal:       ${run_goal:-N/A}"
  echo "  Status:     ${run_status:-N/A}"
  echo "  Plan ID:    ${run_plan_id:-N/A}"
  echo "  Started:    ${run_started:-N/A}"
  echo "  Completed:  ${run_completed:-N/A}"
  echo "  Duration:   ${duration_str}"
  echo "  Cost:       \$${cost} USD"
  echo "  Agents:     ${agents}"
  echo "  Tasks:      ${tasks_done}/${tasks_total} completed"
  echo "  Val pass:   ${val_pass}"
}

cmd_runs() {
  echo "=== Execution Runs ==="
  local rows
  rows=$(_db "SELECT id, goal, status, plan_id, started_at, completed_at, duration_minutes FROM execution_runs ORDER BY id DESC LIMIT 20;")

  if [ -z "$rows" ]; then
    echo "  No execution runs found."
    return
  fi

  printf "  %-5s %-10s %-8s %-40s %s\n" "ID" "PLAN" "STATUS" "GOAL" "STARTED"
  echo "  ───── ────────── ──────── ──────────────────────────────────────── ───────────────────"
  while IFS='|' read -r id goal status plan_id started completed duration; do
    local short_goal="${goal:0:40}"
    printf "  %-5s %-10s %-8s %-40s %s\n" "$id" "${plan_id:-N/A}" "$status" "$short_goal" "${started:-N/A}"
  done <<< "$rows"
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
