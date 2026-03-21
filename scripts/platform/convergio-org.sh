#!/usr/bin/env bash
# convergio-org.sh — Organizational telemetry: flows, relationships, teams
# Tracks every agent interaction and visualizes as org structure
set -uo pipefail

PLATFORM_DIR="${CONVERGIO_PLATFORM_DIR:-$HOME/GitHub/ConvergioPlatform}"
DB="${DASHBOARD_DB:-$PLATFORM_DIR/data/dashboard.db}"
DAEMON_URL="${CONVERGIO_DAEMON_URL:-http://localhost:8420}"

_db() { sqlite3 "$DB" "$1" 2>/dev/null; }

# Data sources:
# - ipc_messages: from_agent, to_agent, channel, content, created_at (daemon core)
# - ipc_agents: name, host, agent_type, last_seen (daemon core)
# - agent_events: enriched events with type classification (local supplement)
# - execution_runs: run metadata (goal, team, cost)

# ─── Live Activity ──────────────────────────────────────────────────

cmd_live() {
  local limit="${1:-20}"
  echo "=== Live Agent Activity (last $limit events) ==="
  echo ""
  printf "  %-8s %-12s %-4s %-12s %s\n" "TIME" "FROM" "" "TO" "CONTENT"
  echo "  ──────── ──────────── ──── ──────────── ──────────────────────────────"

  # Primary source: ipc_messages (daemon core)
  _db "SELECT strftime('%H:%M:%S', created_at), from_agent, '→', channel, substr(content,1,50)
       FROM ipc_messages ORDER BY created_at DESC LIMIT $limit;" 2>/dev/null | while IFS='|' read -r time from arrow to detail; do
    printf "  %-8s %-12s %-4s %-12s %s\n" "$time" "$from" "$arrow" "$to" "$detail"
  done

  # Supplement: agent_events (if populated)
  local extra
  extra=$(_db "SELECT count(*) FROM agent_events;" 2>/dev/null)
  if [ "${extra:-0}" -gt 0 ]; then
    echo ""
    echo "  + $extra enriched events in agent_events"
  fi
}

# ─── Org Chart ──────────────────────────────────────────────────────

cmd_org() {
  local run_id="${1:-}"
  echo "=== Organization Chart ==="
  echo ""

  # Build hierarchy from spawn/delegate events
  local where=""
  [ -n "$run_id" ] && where="AND run_id = $run_id"

  # Find root (agent that spawns but is never spawned)
  local root
  root=$(_db "SELECT DISTINCT from_agent FROM agent_events
              WHERE event_type IN ('spawn','delegate') $where
              AND from_agent NOT IN (SELECT COALESCE(to_agent,'') FROM agent_events WHERE event_type = 'spawn' $where)
              LIMIT 1;")
  [ -z "$root" ] && root="ali"

  echo "  $root (orchestrator)"

  # Level 1: direct reports
  _db "SELECT DISTINCT to_agent FROM agent_events
       WHERE from_agent = '$root' AND event_type IN ('spawn','delegate') $where;" | while read -r l1; do
    [ -z "$l1" ] && continue
    local msg_count
    msg_count=$(_db "SELECT count(*) FROM agent_events WHERE (from_agent='$l1' OR to_agent='$l1') AND event_type='message' $where;")
    local task_count
    task_count=$(_db "SELECT count(*) FROM agent_events WHERE from_agent='$l1' AND event_type IN ('complete','validate') $where;")
    echo "  ├── $l1 ($msg_count msgs, $task_count tasks)"

    # Level 2: sub-delegates
    _db "SELECT DISTINCT to_agent FROM agent_events
         WHERE from_agent = '$l1' AND event_type IN ('spawn','delegate') $where;" | while read -r l2; do
      [ -z "$l2" ] && continue
      echo "  │   └── $l2"
    done
  done
}

# ─── Communication Matrix ──────────────────────────────────────────

cmd_matrix() {
  echo "=== Communication Matrix ==="
  echo ""
  printf "  %-14s %-14s %6s  %s\n" "FROM" "CHANNEL" "COUNT" "LATEST"
  echo "  ────────────── ────────────── ──────  ──────────────────"

  # Primary: ipc_messages (daemon core — all agent communication)
  _db "SELECT from_agent, channel, count(*), max(strftime('%H:%M', created_at))
       FROM ipc_messages
       GROUP BY from_agent, channel
       ORDER BY count(*) DESC
       LIMIT 30;" 2>/dev/null | while IFS='|' read -r from ch count latest; do
    printf "  %-14s %-14s %6s  %s\n" "$from" "$ch" "$count" "$latest"
  done
}

# ─── Team View ──────────────────────────────────────────────────────

cmd_teams() {
  local run_id="${1:-}"
  echo "=== Active Teams ==="
  echo ""

  local where=""
  [ -n "$run_id" ] && where="AND run_id = $run_id"

  # Group by run
  _db "SELECT run_id, count(DISTINCT from_agent) as agents, count(*) as events,
       min(created_at) as started, max(created_at) as latest
       FROM agent_events
       WHERE run_id IS NOT NULL $where
       GROUP BY run_id
       ORDER BY run_id DESC LIMIT 10;" | while IFS='|' read -r rid agents events started latest; do
    local goal
    goal=$(_db "SELECT substr(goal,1,50) FROM execution_runs WHERE id=$rid;" 2>/dev/null)
    echo "  Run #$rid: $agents agents, $events events ($started → $latest)"
    [ -n "$goal" ] && echo "    Goal: $goal"

    # List agents in this run
    _db "SELECT DISTINCT from_agent FROM agent_events WHERE run_id=$rid;" | while read -r agent; do
      local role
      role=$(_db "SELECT category FROM agent_catalog WHERE name='$agent';")
      echo "    ├── $agent ($role)"
    done
    echo ""
  done
}

# ─── Flow Timeline ─────────────────────────────────────────────────

cmd_flow() {
  local run_id="${1:?Usage: flow <run_id>}"
  echo "=== Execution Flow — Run #$run_id ==="
  echo ""

  local goal
  goal=$(_db "SELECT goal FROM execution_runs WHERE id=$run_id;")
  [ -n "$goal" ] && echo "  Goal: $goal"
  echo ""

  _db "SELECT strftime('%H:%M:%S', created_at), event_type, from_agent, COALESCE(to_agent,''), substr(COALESCE(payload,''),1,50)
       FROM agent_events WHERE run_id=$run_id ORDER BY id;" | while IFS='|' read -r time type from to detail; do
    case "$type" in
      spawn)    echo "  $time  ⊕ $from spawned $to" ;;
      delegate) echo "  $time  → $from delegated to $to: $detail" ;;
      message)  echo "  $time  ✉ $from → $to: $detail" ;;
      validate) echo "  $time  ✓ $from validated $to: $detail" ;;
      complete) echo "  $time  ✔ $from completed: $detail" ;;
      fail)     echo "  $time  ✗ $from failed: $detail" ;;
      escalate) echo "  $time  ⚠ $from escalated to $to: $detail" ;;
      *)        echo "  $time  · $from [$type] $detail" ;;
    esac
  done
}

# ─── Stats ──────────────────────────────────────────────────────────

cmd_stats() {
  echo "=== Organizational Stats ==="
  echo ""
  echo "  Total events: $(_db "SELECT count(*) FROM agent_events;")"
  echo "  Unique agents: $(_db "SELECT count(DISTINCT from_agent) FROM agent_events;")"
  echo "  Runs tracked: $(_db "SELECT count(DISTINCT run_id) FROM agent_events WHERE run_id IS NOT NULL;")"
  echo ""
  echo "  Events by type:"
  _db "SELECT event_type, count(*) FROM agent_events GROUP BY event_type ORDER BY count(*) DESC;" | while IFS='|' read -r type count; do
    printf "    %-12s %s\n" "$type" "$count"
  done
  echo ""
  echo "  Most active agents:"
  _db "SELECT from_agent, count(*) FROM agent_events GROUP BY from_agent ORDER BY count(*) DESC LIMIT 10;" | while IFS='|' read -r agent count; do
    printf "    %-14s %s events\n" "$agent" "$count"
  done
}

case "${1:-help}" in
  log)      shift; cmd_log_event "$@" ;;
  live)     shift; cmd_live "${1:-20}" ;;
  org)      shift; cmd_org "${1:-}" ;;
  matrix)   shift; cmd_matrix "${1:-}" ;;
  teams)    shift; cmd_teams "${1:-}" ;;
  flow)     shift; cmd_flow "$@" ;;
  stats)    cmd_stats ;;
  *)
    echo "convergio-org.sh — Organizational telemetry"
    echo ""
    echo "  log <type> <from> [to] [payload] [run_id]  Record event"
    echo "  live [N]                                   Last N events"
    echo "  org [run_id]                               Org chart from flows"
    echo "  matrix [run_id]                            Communication matrix"
    echo "  teams [run_id]                             Active teams"
    echo "  flow <run_id>                              Timeline of a run"
    echo "  stats                                      Overall statistics"
    echo ""
    echo "  Event types: spawn, delegate, message, validate, complete,"
    echo "               fail, escalate, cross_repo, heartbeat"
    ;;
esac
