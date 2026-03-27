#!/bin/bash
# circuit-breaker.sh - Track consecutive Thor rejections, auto-block after threshold
# Version: 2.0.0 — migrated from sqlite3 to cvg CLI / daemon API
# Sourced by plan-db-safe.sh
#
# Requires: DATA_DIR, AUDIT_LOG, REJECTION_COUNTER_DIR, MAX_REJECTIONS
command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 1; }

DAEMON_URL="${DAEMON_URL:-http://localhost:8420}"

circuit_breaker_track_rejection() {
	local task_db_id="$1"
	local plan_id="${2:-}"

	mkdir -p "$REJECTION_COUNTER_DIR"
	local counter_file="$REJECTION_COUNTER_DIR/task-${task_db_id}.count"

	local count=1
	if [[ -f "$counter_file" ]]; then
		count=$(cat "$counter_file")
		count=$((count + 1))
	fi
	echo "$count" >"$counter_file"

	if [[ $count -ge $MAX_REJECTIONS ]]; then
		local task_id_text
		# Read task_id from plan JSON via daemon API
		if [[ -n "$plan_id" ]]; then
			task_id_text=$(curl -sf "${DAEMON_URL}/api/plan-db/json/${plan_id}" | jq -r ".tasks[] | select(.id==${task_db_id} or .db_id==${task_db_id}) | .task_id // \"unknown\"" 2>/dev/null || echo "unknown")
		else
			task_id_text="unknown"
		fi

		echo "CIRCUIT BREAKER: Task $task_id_text rejected $count times - AUTO-BLOCKING" >&2

		# Update task status to blocked via daemon API
		curl -sf -X POST "${DAEMON_URL}/api/plan-db/task/update" \
			-H 'Content-Type: application/json' \
			-d "{\"task_id\":${task_db_id},\"status\":\"blocked\",\"notes\":\"AUTO-BLOCKED: ${count} consecutive Thor rejections (circuit breaker)\"}" >/dev/null 2>&1 || \
			cvg task update "$task_db_id" blocked "AUTO-BLOCKED: ${count} consecutive Thor rejections (circuit breaker)" 2>/dev/null || true

		local timestamp
		timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
		# Get wave_id from plan JSON
		local wave_id="unknown"
		if [[ -n "$plan_id" ]]; then
			wave_id=$(curl -sf "${DAEMON_URL}/api/plan-db/json/${plan_id}" | jq -r ".tasks[] | select(.id==${task_db_id} or .db_id==${task_db_id}) | .wave_id // \"unknown\"" 2>/dev/null || echo "unknown")
		fi

		local audit_entry="{\"timestamp\":\"$timestamp\",\"event\":\"circuit_breaker_triggered\",\"task_db_id\":$task_db_id,\"task_id\":\"$task_id_text\",\"plan_id\":${plan_id:-null},\"wave_id\":\"$wave_id\",\"consecutive_rejections\":$count,\"max_rejections\":$MAX_REJECTIONS,\"action\":\"auto_blocked\"}"

		mkdir -p "$DATA_DIR"
		if command -v flock >/dev/null 2>&1; then
			(
				flock -x 200
				echo "$audit_entry" >>"$AUDIT_LOG"
			) 200>"$AUDIT_LOG.lock"
		else
			echo "$audit_entry" >>"$AUDIT_LOG"
		fi

		rm -f "$counter_file"
		return 1
	fi

	return 0
}

circuit_breaker_reset() {
	local task_db_id="$1"
	local counter_file="$REJECTION_COUNTER_DIR/task-${task_db_id}.count"
	rm -f "$counter_file"
}
