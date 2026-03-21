#!/usr/bin/env bash
# convergio-metrics.sh — Telemetry collector
# Collects system + agent metrics and writes to metrics_history table
# Run via: convergio-autopilot.sh watch (includes metrics) or cron
set -uo pipefail

PLATFORM_DIR="${CONVERGIO_PLATFORM_DIR:-$HOME/GitHub/ConvergioPlatform}"
DB="${DASHBOARD_DB:-$PLATFORM_DIR/data/dashboard.db}"
DAEMON_URL="${CONVERGIO_DAEMON_URL:-http://localhost:8420}"

_db() { sqlite3 "$DB" "$1" 2>/dev/null; }

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
  _db "DELETE FROM metrics_history WHERE recorded_at < datetime('now', '-$days days');"
  echo "Cleaned metrics older than $days days"
}

case "${1:-collect}" in
  collect) cmd_collect ;;
  report)  cmd_report ;;
  clean)   shift; cmd_clean "${1:-30}" ;;
  *)
    echo "convergio-metrics.sh — Telemetry collector"
    echo "  collect        Collect current metrics"
    echo "  report         Show 24h metric report"
    echo "  clean [days]   Remove old metrics (default: 30 days)"
    ;;
esac
