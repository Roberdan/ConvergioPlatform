#!/bin/bash
# plan-db-http.sh — HTTP client for plan-db API (claude-core daemon)
# Provides API-first access with sqlite3 fallback when daemon is down.
# Source this in plan-db.sh or other scripts that need plan-db access.
# Version: 1.0.0
set -euo pipefail

: "${CLAUDE_API_PORT:=8421}"
: "${CLAUDE_API_HOST:=127.0.0.1}"
_PLAN_DB_API_BASE="http://${CLAUDE_API_HOST}:${CLAUDE_API_PORT}"

# Check if the daemon HTTP API is reachable (cached for 30s)
_API_CHECK_CACHE=""
_API_CHECK_TS=0

_api_available() {
	local now
	now=$(date +%s)
	if [[ $((now - _API_CHECK_TS)) -lt 30 && -n "$_API_CHECK_CACHE" ]]; then
		[[ "$_API_CHECK_CACHE" == "1" ]]
		return $?
	fi
	if curl -sf --max-time 2 "${_PLAN_DB_API_BASE}/api/health" >/dev/null 2>&1; then
		_API_CHECK_CACHE="1"
		_API_CHECK_TS=$now
		return 0
	else
		_API_CHECK_CACHE="0"
		_API_CHECK_TS=$now
		return 1
	fi
}

# GET /api/plan-db/context/:plan_id
_api_get_context() {
	local plan_id="${1:?plan_id required}"
	if _api_available; then
		curl -sf --max-time 10 \
			"${_PLAN_DB_API_BASE}/api/plan-db/context/${plan_id}" 2>/dev/null
		return $?
	fi
	# Fallback: direct sqlite3
	_fallback_get_context "$plan_id"
}

# POST /api/plan-db/task/update
_api_update_task() {
	local task_id="${1:?task_id required}"
	local status="${2:?status required}"
	local notes="${3:-}"
	local tokens="${4:-0}"
	if _api_available; then
		curl -sf --max-time 10 \
			-X POST -H "Content-Type: application/json" \
			-d "{\"task_id\":${task_id},\"status\":\"${status}\",\"notes\":\"${notes}\",\"tokens\":${tokens}}" \
			"${_PLAN_DB_API_BASE}/api/plan-db/task/update" 2>/dev/null
		return $?
	fi
	# Fallback: direct sqlite3
	local db="${DB_FILE:-$HOME/.claude/data/dashboard.db}"
	sqlite3 -cmd ".timeout 5000" "$db" \
		"UPDATE tasks SET status=:status WHERE id=:tid;" \
		":status=$status" ":tid=$task_id"
	echo "{\"ok\":true,\"task_id\":${task_id},\"status\":\"${status}\",\"fallback\":true}"
}

# POST /api/plan-db/agent/start
_api_agent_start() {
	local agent_id="${1:?agent_id required}"
	local agent_type="${2:?agent_type required}"
	local description="${3:-}"
	local task_db_id="${4:-null}"
	local plan_id="${5:-null}"
	local model="${6:-unknown}"
	local host="${7:-$(hostname -s)}"
	if _api_available; then
		curl -sf --max-time 10 \
			-X POST -H "Content-Type: application/json" \
			-d "{\"agent_id\":\"${agent_id}\",\"agent_type\":\"${agent_type}\",\"description\":\"${description}\",\"task_db_id\":${task_db_id},\"plan_id\":${plan_id},\"model\":\"${model}\",\"host\":\"${host}\"}" \
			"${_PLAN_DB_API_BASE}/api/plan-db/agent/start" 2>/dev/null
		return $?
	fi
	# Fallback: direct sqlite3
	local db="${DB_FILE:-$HOME/.claude/data/dashboard.db}"
	sqlite3 -cmd ".timeout 5000" "$db" <<-EOSQL
		INSERT OR REPLACE INTO agent_activity
			(agent_id, agent_type, description, task_db_id, plan_id, model, host, status, started_at)
		VALUES (
			$(printf "'%s'" "${agent_id//\'/\'\'}"),
			$(printf "'%s'" "${agent_type//\'/\'\'}"),
			$(printf "'%s'" "${description//\'/\'\'}"),
			${task_db_id}, ${plan_id},
			$(printf "'%s'" "${model//\'/\'\'}"),
			$(printf "'%s'" "${host//\'/\'\'}"),
			'running', datetime('now')
		);
	EOSQL
	echo "{\"ok\":true,\"agent_id\":\"${agent_id}\",\"fallback\":true}"
}

# POST /api/plan-db/agent/complete
_api_agent_complete() {
	local agent_id="${1:?agent_id required}"
	local tokens_in="${2:-0}"
	local tokens_out="${3:-0}"
	local cost="${4:-0}"
	local status="${5:-completed}"
	if _api_available; then
		curl -sf --max-time 10 \
			-X POST -H "Content-Type: application/json" \
			-d "{\"agent_id\":\"${agent_id}\",\"tokens_in\":${tokens_in},\"tokens_out\":${tokens_out},\"cost_usd\":${cost},\"status\":\"${status}\"}" \
			"${_PLAN_DB_API_BASE}/api/plan-db/agent/complete" 2>/dev/null
		return $?
	fi
	# Fallback: direct sqlite3
	local db="${DB_FILE:-$HOME/.claude/data/dashboard.db}"
	sqlite3 -cmd ".timeout 5000" "$db" <<-EOSQL
		UPDATE agent_activity
		SET status=$(printf "'%s'" "${status//\'/\'\'}"),
		    tokens_in=${tokens_in}, tokens_out=${tokens_out},
		    tokens_total=$((tokens_in + tokens_out)),
		    completed_at=datetime('now')
		WHERE agent_id=$(printf "'%s'" "${agent_id//\'/\'\'}");
	EOSQL
	echo "{\"ok\":true,\"agent_id\":\"${agent_id}\",\"fallback\":true}"
}

# Fallback: get-context via sqlite3 (matches plan-db.sh get-context output)
_fallback_get_context() {
	local plan_id="$1"
	local db="${DB_FILE:-$HOME/.claude/data/dashboard.db}"
	sqlite3 -json -cmd ".timeout 5000" "$db" "
		SELECT
			p.id as plan_id, p.name, p.status as plan_status,
			p.project_id, p.execution_host, p.worktree_path as plan_worktree,
			p.description as plan_description,
			t.id as task_db_id, t.task_id, t.title, t.description,
			t.status as task_status, t.type, t.priority,
			w.id as wave_db_id, w.wave_id, w.name as wave_name,
			w.worktree_path as wave_worktree
		FROM tasks t
		JOIN plans p ON t.plan_id = p.id
		LEFT JOIN waves w ON t.wave_id_fk = w.id
		WHERE t.plan_id = cast(${plan_id} as integer)
		ORDER BY w.id, t.id;
	" 2>/dev/null || echo "[]"
}
