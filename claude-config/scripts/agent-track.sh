#!/usr/bin/env bash
set -euo pipefail
# agent-track.sh — Lightweight agent activity tracker for brain visualization
# Version: 2.0.0 — migrated from sqlite3 to cvg CLI / daemon API
# Usage: agent-track.sh start|complete|list|stats (see case branches below)
# Standalone — no dependency on plan-db.sh being in PATH.
command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 1; }

DAEMON_URL="${DAEMON_URL:-http://localhost:8420}"

case "${1:-}" in
  start) # <id> <type> <desc> [--task N] [--plan N] [--model M] [--host H] [--parent S]
    shift; [[ $# -lt 3 ]] && { echo "Usage: agent-track.sh start <id> <type> <desc>" >&2; exit 2; }
    AID="$1"; shift; TYPE="$1"; shift; DESC="$1"; shift
    TID="" PID="" MDL="" HOST="$(hostname -s)" PSESS=""
    while [[ $# -gt 0 ]]; do case "$1" in
      --task) TID="$2"; shift 2;; --plan) PID="$2"; shift 2;;
      --model) MDL="$2"; shift 2;; --host) HOST="$2"; shift 2;;
      --parent) PSESS="$2"; shift 2;; *) shift;; esac; done
    # Register agent via daemon API
    cvg agent start "$AID" 2>/dev/null || \
    curl -sf -X POST "${DAEMON_URL}/api/ipc/agent/start" \
      -H 'Content-Type: application/json' \
      -d "$(jq -n \
        --arg aid "$AID" --arg type "$TYPE" --arg desc "$DESC" \
        --arg tid "${TID:-}" --arg pid "${PID:-}" \
        --arg mdl "${MDL:-unknown}" --arg host "$HOST" --arg psess "${PSESS:-}" \
        '{agent_id:$aid,agent_type:$type,description:$desc,task_db_id:($tid|if .=="" then null else tonumber end),plan_id:($pid|if .=="" then null else tonumber end),model:$mdl,host:$host,parent_session:($psess|if .=="" then null else . end)}'
      )" >/dev/null 2>&1 || true
    echo "{\"ok\":true,\"agent_id\":\"$AID\"}" ;;

  complete) # <agent_id> [--tokens-in N] [--tokens-out N] [--cost N] [--status S]
    shift; [[ $# -lt 1 ]] && { echo "Usage: agent-track.sh complete <id>" >&2; exit 2; }
    AID="$1"; shift; ST="completed" TIN=0 TOUT=0 COST=0
    while [[ $# -gt 0 ]]; do case "$1" in
      --status) ST="$2"; shift 2;; --tokens-in) TIN="$2"; shift 2;;
      --tokens-out) TOUT="$2"; shift 2;; --cost) COST="$2"; shift 2;; *) shift;; esac; done
    # Complete agent via daemon API
    cvg agent complete "$AID" 2>/dev/null || \
    curl -sf -X POST "${DAEMON_URL}/api/ipc/agent/complete" \
      -H 'Content-Type: application/json' \
      -d "$(jq -n \
        --arg aid "$AID" --arg st "$ST" \
        --argjson tin "$TIN" --argjson tout "$TOUT" --argjson cost "$COST" \
        '{agent_id:$aid,status:$st,tokens_in:$tin,tokens_out:$tout,cost_usd:$cost}'
      )" >/dev/null 2>&1 || true
    echo "{\"ok\":true,\"agent_id\":\"$AID\",\"status\":\"$ST\"}" ;;

  list) # [--running] [--plan N]
    shift; RUNNING="" PLAN_FILTER=""
    while [[ $# -gt 0 ]]; do case "$1" in
      --running) RUNNING="true"; shift;; --plan) PLAN_FILTER="$2"; shift 2;; *) shift;; esac; done
    # List agents via daemon API
    local_result=$(curl -sf "${DAEMON_URL}/api/ipc/agents" 2>/dev/null) || local_result='[]'
    if [[ -n "$RUNNING" ]]; then
      local_result=$(echo "$local_result" | jq -c '[.[] | select(.status == "running")]')
    fi
    if [[ -n "$PLAN_FILTER" ]]; then
      local_result=$(echo "$local_result" | jq -c --argjson pid "$PLAN_FILTER" '[.[] | select(.plan_id == $pid)]')
    fi
    echo "$local_result" | jq -c '.[0:50]' ;;

  stats) # [--plan N]
    shift; PF=""
    [[ "${1:-}" == "--plan" ]] && PF="$2"
    # Get stats via daemon API
    local_result=$(curl -sf "${DAEMON_URL}/api/ipc/agents" 2>/dev/null) || local_result='[]'
    if [[ -n "$PF" ]]; then
      local_result=$(echo "$local_result" | jq -c --argjson pid "$PF" '[.[] | select(.plan_id == $pid)]')
    fi
    echo "$local_result" | jq -c '{total:length,running:[.[]|select(.status=="running")]|length,tokens:([.[].tokens_total//0]|add//0),cost:([.[].cost_usd//0]|add//0|.*10000|round/10000)}' ;;

  *) echo "Usage: agent-track.sh start|complete|list|stats" >&2; exit 2;;
esac
