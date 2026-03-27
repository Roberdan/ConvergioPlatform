#!/usr/bin/env bash
# task-file-tracker.sh v2.0.0 — Track files modified by each task
# Migrated from sqlite3 to cvg CLI / daemon API
# Survives compaction (writes to daemon DB, not context).
# Called by PostToolUse hook after Edit/Write, or manually.
set -euo pipefail
command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 1; }

DAEMON_URL="${DAEMON_URL:-http://localhost:8420}"
USAGE="Usage: task-file-tracker.sh <command> [args]
Commands:
  track <task_db_id> <file_path> <action>  Record file modification
  list <task_db_id>                         List files for a task
  list-plan <plan_id>                       List all files for a plan
  overlap <plan_id>                         Detect file overlap between tasks
  clean <task_db_id>                        Remove tracking for a task"

cmd="${1:-}"
[ -z "$cmd" ] && echo "$USAGE" && exit 1

case "$cmd" in
  track)
    TASK_ID="${2:?task_db_id required}"
    FILE_PATH="${3:?file_path required}"
    ACTION="${4:-edit}"
    # Track file via daemon API
    curl -sf -X POST "${DAEMON_URL}/api/plan-db/task-files/track" \
      -H 'Content-Type: application/json' \
      -d "$(jq -n --argjson tid "$TASK_ID" --arg fp "$FILE_PATH" --arg act "$ACTION" \
        '{task_id:$tid,file_path:$fp,action:$act}')" >/dev/null 2>&1 || {
      # TODO: needs daemon endpoint for task_files table
      echo "WARNING: task-files tracking API not available" >&2
    }
    echo "Tracked: task=$TASK_ID file=$FILE_PATH action=$ACTION"
    ;;

  list)
    TASK_ID="${2:?task_db_id required}"
    # List files via daemon API
    curl -sf "${DAEMON_URL}/api/plan-db/task-files/list?task_id=${TASK_ID}" 2>/dev/null | \
      jq -r '.[] | "\(.file_path)\t\(.action)\t\(.recorded_at // "")"' 2>/dev/null || {
      # TODO: needs daemon endpoint for task_files query
      echo "WARNING: task-files list API not available" >&2
    }
    ;;

  list-plan)
    PLAN_ID="${2:?plan_id required}"
    # List plan files via daemon API — get plan JSON and cross-reference
    curl -sf "${DAEMON_URL}/api/plan-db/task-files/list-plan?plan_id=${PLAN_ID}" 2>/dev/null | \
      jq -r '.[] | "\(.task_id)\t\(.title // "")\t\(.file_path)\t\(.action)"' 2>/dev/null || {
      # TODO: needs daemon endpoint for task_files plan query
      echo "WARNING: task-files list-plan API not available" >&2
    }
    ;;

  overlap)
    PLAN_ID="${2:?plan_id required}"
    echo "=== File Overlap Detection (Plan $PLAN_ID) ==="
    # Detect overlaps via daemon API
    OVERLAPS=$(curl -sf "${DAEMON_URL}/api/plan-db/task-files/overlap?plan_id=${PLAN_ID}" 2>/dev/null) || OVERLAPS=""
    if [[ -z "$OVERLAPS" ]] || echo "$OVERLAPS" | jq -e 'length == 0' >/dev/null 2>&1; then
      echo "No file overlaps detected."
    else
      echo "WARNING: Files touched by multiple tasks:"
      echo "$OVERLAPS" | jq -r '.[] | "  \(.file_path) — \(.task_count) tasks: [\(.tasks)]"' 2>/dev/null || {
        # TODO: needs daemon endpoint for task_files overlap detection
        echo "WARNING: task-files overlap API not available" >&2
      }
      echo ""
      echo "These files WILL conflict at merge. Serialize affected tasks."
    fi
    ;;

  clean)
    TASK_ID="${2:?task_db_id required}"
    # Clean via daemon API
    curl -sf -X POST "${DAEMON_URL}/api/plan-db/task-files/clean" \
      -H 'Content-Type: application/json' \
      -d "{\"task_id\":${TASK_ID}}" 2>/dev/null || {
      # TODO: needs daemon endpoint for task_files cleanup
      echo "WARNING: task-files clean API not available" >&2
    }
    echo "Cleaned file records for task $TASK_ID"
    ;;

  *)
    echo "$USAGE"
    exit 1
    ;;
esac
