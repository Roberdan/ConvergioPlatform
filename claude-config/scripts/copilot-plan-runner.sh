#!/bin/bash
# Version: 1.0.0
# Auto-restart copilot until plan is 100% complete
# Usage: copilot-plan-runner.sh <plan_id>
set -euo pipefail
trap 'echo "ERROR at line $LINENO" >&2' ERR

command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 1; }

PLAN_ID="$1"
MAX_RETRIES=50
RETRY=0

plan_done() {
	local plan_json pending
	plan_json="$(cvg plan show "$PLAN_ID" 2>/dev/null || echo '{}')"
	pending="$(echo "$plan_json" | jq '[.tasks[] | select(.status | IN("done","validated","skipped","cancelled") | not)] | length' 2>/dev/null || echo 1)"
	[ "$pending" -eq 0 ]
}

plan_summary() {
	cvg plan show "$PLAN_ID" 2>/dev/null | jq -r '[.tasks[] | .status] | group_by(.) | map("\(.[0])|\(length)") | .[]' 2>/dev/null || echo "unknown|0"
}

echo "=== Plan #$PLAN_ID Runner (auto-restart) ==="

while ! plan_done; do
	RETRY=$((RETRY + 1))
	if [ "$RETRY" -gt "$MAX_RETRIES" ]; then
		echo "[FAIL] Max retries ($MAX_RETRIES) reached. Plan still incomplete:"
		plan_summary
		exit 1
	fi

	_plan_json="$(cvg plan show "$PLAN_ID" 2>/dev/null || echo '{}')"
	REMAINING="$(echo "$_plan_json" | jq '[.tasks[] | select(.status | IN("done","validated","skipped","cancelled") | not)] | length' 2>/dev/null || echo '?')"
	echo ""
	echo "[Run $RETRY/$MAX_RETRIES] $REMAINING tasks remaining..."
	plan_summary
	echo ""

	# Reset any stuck in_progress tasks from previous crashed run
	_stuck_ids="$(echo "$_plan_json" | jq -r '.tasks[] | select(.status == "in_progress") | .id' 2>/dev/null || true)"
	for _sid in $_stuck_ids; do
		cvg task update "$_sid" pending "Reset stuck task from crashed run" 2>/dev/null || true
	done

	# Use claude or copilot — whichever is available
	if command -v claude &>/dev/null; then
		CLI="claude"
		CLI_ARGS="--dangerously-skip-permissions -p"
	elif command -v copilot &>/dev/null; then
		CLI="copilot"
		CLI_ARGS="--yolo -p"
	else
		echo "[FAIL] Neither claude nor copilot CLI found" >&2
		exit 1
	fi

	$CLI $CLI_ARGS "/execute $PLAN_ID" 2>&1
	EXIT_CODE=$?

	echo ""
	echo "[Run $RETRY] $CLI exited (code $EXIT_CODE). Checking progress..."
	sleep 2
done

echo ""
echo "=== Plan #$PLAN_ID COMPLETE ==="
plan_summary
