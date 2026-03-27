#!/bin/bash
set -euo pipefail
trap 'echo "ERROR at line $LINENO" >&2' ERR
# Quick Task Operations - Reduces token usage for common operations
# Version: 2.0.0 — migrated from sqlite3 to cvg CLI / daemon API
# Usage: task-quick.sh <command> [args]
#
# Commands:
#   start <task_id>     - Mark task in_progress
#   done <task_id>      - Mark task done
#   status              - Show current task status
#   next                - Show next pending task
command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 1; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DAEMON_URL="${DAEMON_URL:-http://localhost:8420}"

case "${1:-help}" in
    start)
        TASK_ID="${2:?task_id required}"
        "$SCRIPT_DIR/plan-db.sh" update-task "$TASK_ID" in_progress "Started via task-quick"
        # Show task info via daemon API
        PLAN_LIST=$(curl -sf "${DAEMON_URL}/api/plan-db/list" 2>/dev/null | jq -r '.[].id' 2>/dev/null) || PLAN_LIST=""
        for pid in $PLAN_LIST; do
            TASK_JSON=$(curl -sf "${DAEMON_URL}/api/plan-db/json/${pid}" | jq -c ".tasks[]? | select(.id==${TASK_ID} or .db_id==${TASK_ID})" 2>/dev/null)
            if [[ -n "$TASK_JSON" ]]; then
                echo "$TASK_JSON" | jq -r '"\(.task_id)|\(.title // "")"'
                break
            fi
        done
        ;;
    done)
        TASK_ID="${2:?task_id required}"
        NOTES="${3:-Completed via task-quick}"
        "$SCRIPT_DIR/plan-db.sh" update-task "$TASK_ID" done "$NOTES"
        # Show task info via daemon API
        PLAN_LIST=$(curl -sf "${DAEMON_URL}/api/plan-db/list" 2>/dev/null | jq -r '.[].id' 2>/dev/null) || PLAN_LIST=""
        for pid in $PLAN_LIST; do
            TASK_JSON=$(curl -sf "${DAEMON_URL}/api/plan-db/json/${pid}" | jq -c ".tasks[]? | select(.id==${TASK_ID} or .db_id==${TASK_ID})" 2>/dev/null)
            if [[ -n "$TASK_JSON" ]]; then
                echo "$TASK_JSON" | jq -r '"\(.task_id)|\(.title // "")|\(.status // "")"'
                break
            fi
        done
        ;;
    status)
        echo "=== In Progress ==="
        # Get all active plans and find in_progress tasks
        PLAN_LIST=$(curl -sf "${DAEMON_URL}/api/plan-db/list" 2>/dev/null | jq -r '.[] | select(.status == "doing") | .id' 2>/dev/null) || PLAN_LIST=""
        for pid in $PLAN_LIST; do
            curl -sf "${DAEMON_URL}/api/plan-db/json/${pid}" 2>/dev/null | \
                jq -r '.tasks[]? | select(.status == "in_progress") | "\(.id // .db_id)\t\(.task_id)\t\(.title // "")\t\(.wave_id // "")"' 2>/dev/null || true
        done | head -5
        ;;
    next)
        echo "=== Next Pending ==="
        # Get all active plans and find pending tasks
        PLAN_LIST=$(curl -sf "${DAEMON_URL}/api/plan-db/list" 2>/dev/null | jq -r '.[] | select(.status == "doing") | .id' 2>/dev/null) || PLAN_LIST=""
        for pid in $PLAN_LIST; do
            PLAN_NAME=$(curl -sf "${DAEMON_URL}/api/plan-db/json/${pid}" 2>/dev/null | jq -r '.name // ""')
            curl -sf "${DAEMON_URL}/api/plan-db/json/${pid}" 2>/dev/null | \
                jq -r --arg pname "$PLAN_NAME" '.tasks[]? | select(.status == "pending") | "\(.id // .db_id)\t\(.task_id)\t\(.title // "")\t\(.wave_id // "")\t\($pname)"' 2>/dev/null || true
        done | head -3
        ;;
    *)
        echo "Usage: task-quick.sh <start|done|status|next> [args]"
        ;;
esac
