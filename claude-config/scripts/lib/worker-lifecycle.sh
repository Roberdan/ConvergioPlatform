#!/usr/bin/env bash
# worker-lifecycle.sh — Lifecycle functions for copilot-worker.
# Sourced by copilot-worker.sh. Requires: DAEMON_API, SCRIPT_DIR.

_WORKER_TMPFILES=()
_WORKER_CHILD_PIDS=()
AGENT_COMPLETED=0

complete_agent_tracking() {
	if [[ "${AGENT_COMPLETED:-0}" -eq 1 ]]; then return 0; fi
	if [[ -n "${AGENT_ID:-}" ]]; then
		local _status="failed"
		[[ "${FINAL_EXIT_CODE:-1}" -eq 0 ]] && _status="completed"
		"$SCRIPT_DIR/plan-db.sh" agent-complete "$AGENT_ID" \
			--tokens-in "${TOKENS_USED:-0}" --tokens-out 0 --status "$_status" 2>/dev/null || true
	fi
	AGENT_COMPLETED=1
}

_worker_cleanup() {
	set +u
	_emit_mesh_event "agent_finished" \
		"{\"task_id\":${TASK_ID:-0},\"exit_code\":${FINAL_EXIT_CODE:-1},\"agent_id\":\"${AGENT_ID:-}\"}" 2>/dev/null || true
	complete_agent_tracking
	if [[ ${#_WORKER_CHILD_PIDS[@]} -gt 0 ]]; then
		for pid in "${_WORKER_CHILD_PIDS[@]}"; do
			kill -9 "$pid" 2>/dev/null || true
			pkill -9 -P "$pid" 2>/dev/null || true
		done
	fi
	for f in "${_WORKER_TMPFILES[@]}"; do
		[[ -f "$f" ]] && rm -f "$f"
	done
	set -u
	pkill -9 -P $$ 2>/dev/null || true
}

_emit_mesh_event() {
	local etype="${1:?event_type}" payload="${2:-{}}"
	local host
	host="$(hostname -s 2>/dev/null || echo 'unknown')"
	curl -sf -X POST "${DAEMON_API}/api/coordinator/emit" \
		-H 'Content-Type: application/json' \
		-d "{\"event_type\":\"${etype}\",\"source_peer\":\"${host}\",\"plan_id\":${PLAN_ID:-0},\"payload\":${payload}}" 2>/dev/null || true
}

_poll_messages() {
	local agent_name="$1"
	while true; do
		sleep 60
		local msgs count
		msgs="$(curl -sf "${DAEMON_API}/api/ipc/messages?to_agent=${agent_name}&limit=10" 2>/dev/null || echo '{}')"
		count="$(echo "$msgs" | jq -r '(.messages // []) | length' 2>/dev/null || echo 0)"
		if [[ "$count" -gt 0 ]]; then
			echo "[IPC] ${count} message(s) for ${agent_name}:" >&2
			echo "$msgs" | jq -r '.messages[] | "[IPC] from=\(.from_agent // "?") \(.content)"' 2>/dev/null >&2 || true
		fi
	done
}
