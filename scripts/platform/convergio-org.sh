#!/usr/bin/env bash
# convergio-org.sh — Organizational telemetry: flows, relationships, teams
# Tracks every agent interaction and visualizes as org structure
set -euo pipefail

PLATFORM_DIR="${CONVERGIO_PLATFORM_DIR:-$HOME/GitHub/ConvergioPlatform}"
DAEMON_URL="${CONVERGIO_DAEMON_URL:-http://localhost:8420}"

command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 1; }
command -v curl &>/dev/null || { echo "ERROR: curl required" >&2; exit 1; }

_api_get() { curl -sf --connect-timeout 2 "${DAEMON_URL}${1}" 2>/dev/null; }

# ─── Live Activity ──────────────────────────────────────────────────

cmd_live() {
  local limit="${1:-20}"
  echo "=== Live Agent Activity (last $limit events) ==="
  echo ""
  printf "  %-8s %-12s %-4s %-12s %s\n" "TIME" "FROM" "" "TO" "CONTENT"
  echo "  ──────── ──────────── ──── ──────────── ──────────────────────────────"

  # Primary source: ipc_messages via daemon API
  local json
  json=$(_api_get "/api/ipc/messages?limit=${limit}") || { echo "  (daemon not reachable)"; return 0; }

  echo "$json" | jq -r '.messages[]? | [(.time // ""), (.from_agent // ""), "->", (.channel // ""), (.content // "" | .[0:50])] | @tsv' 2>/dev/null | while IFS=$'\t' read -r time from arrow to detail; do
    printf "  %-8s %-12s %-4s %-12s %s\n" "$time" "$from" "$arrow" "$to" "$detail"
  done

  # Supplement: event count from events API
  local events_json
  events_json=$(_api_get "/api/events?count_only=true") || events_json=""
  local extra
  extra=$(echo "$events_json" | jq -r '.count // 0' 2>/dev/null || echo "0")
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

  # Build hierarchy from events API
  local url="/api/events?type=spawn,delegate"
  [ -n "$run_id" ] && url="${url}&run_id=${run_id}"

  local json
  json=$(_api_get "$url") || { echo "  (daemon not reachable)"; return 0; }

  # Find root (agent that spawns but is never spawned)
  local root
  root=$(echo "$json" | jq -r '
    [.events[]? | select(.event_type == "spawn" or .event_type == "delegate") | .from_agent] -
    [.events[]? | select(.event_type == "spawn") | .to_agent // empty] |
    first // "ali"' 2>/dev/null || echo "ali")

  echo "  $root (orchestrator)"

  # Level 1: direct reports
  echo "$json" | jq -r "[.events[]? | select(.from_agent == \"${root}\" and (.event_type == \"spawn\" or .event_type == \"delegate\")) | .to_agent // empty] | unique[]" 2>/dev/null | while read -r l1; do
    [ -z "$l1" ] && continue
    # Count messages and tasks for this agent
    local agent_events
    agent_events=$(_api_get "/api/events?agent=${l1}") || agent_events=""
    local msg_count task_count
    msg_count=$(echo "$agent_events" | jq -r '[.events[]? | select(.event_type == "message")] | length' 2>/dev/null || echo "0")
    task_count=$(echo "$agent_events" | jq -r '[.events[]? | select(.event_type == "complete" or .event_type == "validate")] | length' 2>/dev/null || echo "0")
    echo "  |-- $l1 ($msg_count msgs, $task_count tasks)"

    # Level 2: sub-delegates
    echo "$json" | jq -r "[.events[]? | select(.from_agent == \"${l1}\" and (.event_type == \"spawn\" or .event_type == \"delegate\")) | .to_agent // empty] | unique[]" 2>/dev/null | while read -r l2; do
      [ -z "$l2" ] && continue
      echo "  |   +-- $l2"
    done
  done
}

# ─── Communication Matrix ──────────────────────────────────────────

cmd_matrix() {
  echo "=== Communication Matrix ==="
  echo ""
  printf "  %-14s %-14s %6s  %s\n" "FROM" "CHANNEL" "COUNT" "LATEST"
  echo "  ────────────── ────────────── ──────  ──────────────────"

  # Primary: ipc_messages via daemon API
  local json
  json=$(_api_get "/api/ipc/messages?grouped=true&limit=30") || { echo "  (daemon not reachable)"; return 0; }

  echo "$json" | jq -r '.groups[]? | [(.from_agent // ""), (.channel // ""), (.count // 0), (.latest // "")] | @tsv' 2>/dev/null | while IFS=$'\t' read -r from ch count latest; do
    printf "  %-14s %-14s %6s  %s\n" "$from" "$ch" "$count" "$latest"
  done
}

# ─── Team View ──────────────────────────────────────────────────────

cmd_teams() {
  local run_id="${1:-}"
  echo "=== Active Teams ==="
  echo ""

  local url="/api/events?grouped_by_run=true"
  [ -n "$run_id" ] && url="/api/events?run_id=${run_id}&grouped_by_run=true"

  local json
  json=$(_api_get "$url") || { echo "  (daemon not reachable)"; return 0; }

  echo "$json" | jq -r '.runs[]? | [(.run_id // ""), (.agent_count // 0), (.event_count // 0), (.started // ""), (.latest // "")] | @tsv' 2>/dev/null | while IFS=$'\t' read -r rid agents events started latest; do
    [[ -z "$rid" ]] && continue
    # Get run goal
    local run_json
    run_json=$(_api_get "/api/metrics/run/${rid}") || run_json=""
    local goal
    goal=$(echo "$run_json" | jq -r '.goal // "" | .[0:50]' 2>/dev/null || echo "")
    echo "  Run #$rid: $agents agents, $events events ($started -> $latest)"
    [ -n "$goal" ] && echo "    Goal: $goal"

    # List agents in this run
    echo "$json" | jq -r ".runs[]? | select(.run_id == ${rid}) | .agents[]? | [(.name // \"\"), (.category // \"\")] | @tsv" 2>/dev/null | while IFS=$'\t' read -r agent role; do
      echo "    |-- $agent ($role)"
    done
    echo ""
  done
}

# ─── Flow Timeline ─────────────────────────────────────────────────

cmd_flow() {
  local run_id="${1:?Usage: flow <run_id>}"
  echo "=== Execution Flow — Run #$run_id ==="
  echo ""

  # Get run goal
  local run_json
  run_json=$(_api_get "/api/metrics/run/${run_id}") || run_json=""
  local goal
  goal=$(echo "$run_json" | jq -r '.goal // ""' 2>/dev/null || echo "")
  [ -n "$goal" ] && echo "  Goal: $goal"
  echo ""

  local json
  json=$(_api_get "/api/events?run_id=${run_id}") || { echo "  (daemon not reachable)"; return 0; }

  echo "$json" | jq -r '.events[]? | [(.time // ""), (.event_type // ""), (.from_agent // ""), (.to_agent // ""), (.payload // "" | .[0:50])] | @tsv' 2>/dev/null | while IFS=$'\t' read -r time type from to detail; do
    case "$type" in
      spawn)    echo "  $time  + $from spawned $to" ;;
      delegate) echo "  $time  -> $from delegated to $to: $detail" ;;
      message)  echo "  $time  @ $from -> $to: $detail" ;;
      validate) echo "  $time  V $from validated $to: $detail" ;;
      complete) echo "  $time  OK $from completed: $detail" ;;
      fail)     echo "  $time  X $from failed: $detail" ;;
      escalate) echo "  $time  ! $from escalated to $to: $detail" ;;
      *)        echo "  $time  . $from [$type] $detail" ;;
    esac
  done
}

# ─── Stats ──────────────────────────────────────────────────────────

cmd_stats() {
  echo "=== Organizational Stats ==="
  echo ""

  local json
  json=$(_api_get "/api/events?stats=true") || { echo "  (daemon not reachable)"; return 0; }

  echo "  Total events: $(echo "$json" | jq -r '.total_events // 0' 2>/dev/null)"
  echo "  Unique agents: $(echo "$json" | jq -r '.unique_agents // 0' 2>/dev/null)"
  echo "  Runs tracked: $(echo "$json" | jq -r '.runs_tracked // 0' 2>/dev/null)"
  echo ""
  echo "  Events by type:"
  echo "$json" | jq -r '.by_type[]? | [(.event_type // ""), (.count // 0)] | @tsv' 2>/dev/null | while IFS=$'\t' read -r type count; do
    printf "    %-12s %s\n" "$type" "$count"
  done
  echo ""
  echo "  Most active agents:"
  echo "$json" | jq -r '.top_agents[]? | [(.agent // ""), (.count // 0)] | @tsv' 2>/dev/null | while IFS=$'\t' read -r agent count; do
    printf "    %-14s %s events\n" "$agent" "$count"
  done
}

case "${1:-help}" in
  live)     shift; cmd_live "${1:-20}" ;;
  org)      shift; cmd_org "${1:-}" ;;
  matrix)   shift; cmd_matrix "${1:-}" ;;
  teams)    shift; cmd_teams "${1:-}" ;;
  flow)     shift; cmd_flow "$@" ;;
  stats)    cmd_stats ;;
  *)
    echo "convergio-org.sh — Organizational telemetry"
    echo ""
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
