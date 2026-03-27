#!/bin/bash
set -euo pipefail
trap 'echo "ERROR at line $LINENO" >&2' ERR
# Verify task was updated in DB after task-executor completion
# Version: 2.0.0 — migrated from sqlite3 to cvg CLI / daemon API
# Usage: verify-task-update.sh <db_task_id> [expected_status]
#
# Returns:
#   0 = Task properly updated
#   1 = Task NOT updated (still pending/in_progress when should be done)
#   2 = Task not found
command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 1; }

DAEMON_URL="${DAEMON_URL:-http://localhost:8420}"
TASK_ID="${1:?Usage: verify-task-update.sh <db_task_id> [expected_status]}"
EXPECTED_STATUS="${2:-done}"

# Fetch task info via daemon API — search across plans
TASK_JSON=""
PLAN_LIST=$(curl -sf "${DAEMON_URL}/api/plan-db/list" 2>/dev/null | jq -r '.[].id' 2>/dev/null) || PLAN_LIST=""
for pid in $PLAN_LIST; do
	TASK_JSON=$(curl -sf "${DAEMON_URL}/api/plan-db/json/${pid}" | jq -c ".tasks[]? | select(.id==${TASK_ID} or .db_id==${TASK_ID})" 2>/dev/null)
	[[ -n "$TASK_JSON" ]] && break
done

if [ -z "$TASK_JSON" ]; then
	echo "ERROR: Task $TASK_ID not found in database"
	exit 2
fi

# Parse fields
STATUS=$(echo "$TASK_JSON" | jq -r '.status // ""')
NOTES=$(echo "$TASK_JSON" | jq -r '.notes // ""')
TASK_CODE=$(echo "$TASK_JSON" | jq -r '.task_id // ""')

# Check status
if [ "$STATUS" = "pending" ]; then
	echo "FAILED: Task $TASK_CODE (id=$TASK_ID) still PENDING"
	echo "  -> Executor forgot to update status"
	echo "  -> Run: cvg task update $TASK_ID $EXPECTED_STATUS \"Summary\""
	exit 1
fi

if [ "$STATUS" = "in_progress" ] && [ "$EXPECTED_STATUS" = "done" ]; then
	echo "FAILED: Task $TASK_CODE (id=$TASK_ID) stuck IN_PROGRESS"
	echo "  -> Executor started but forgot to mark done"
	echo "  -> Run: cvg task update $TASK_ID done \"Summary\""
	exit 1
fi

if [ "$STATUS" != "$EXPECTED_STATUS" ]; then
	echo "WARNING: Task $TASK_CODE status is '$STATUS', expected '$EXPECTED_STATUS'"
	exit 0 # Not a hard failure, just warning
fi

# Check notes (soft warning)
if [ -z "$NOTES" ] || [ "$NOTES" = "null" ]; then
	echo "WARNING: Task $TASK_CODE has no completion notes"
fi

echo "OK: Task $TASK_CODE (id=$TASK_ID) status=$STATUS"
exit 0
