#!/bin/bash
# copilot-worker.sh - Launch Copilot CLI worker for a plan task
# Usage: copilot-worker.sh --plan <plan_id> --task <db_task_id> [--model <model>] [--timeout <secs>] [--no-auto-validate]
#        copilot-worker.sh <db_task_id> [options]  (deprecated: scans all plans)
# Version: 4.0.0 - Add --plan/--task flags + execution-context API + IPC registration
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONTEXT_LOADER="${SCRIPT_DIR}/lib/agent-context-loader.sh"
DAEMON_API="http://localhost:8420"

command -v jq &>/dev/null || { echo '{"error":"jq required"}' >&2; exit 1; }

# PATH hardening: ensure copilot CLI is findable in non-login SSH shells
export PATH="$HOME/.local/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:$HOME/.claude/scripts:$PATH"

# Worktree build isolation: use a separate CARGO_TARGET_DIR so cargo
# check/build hooks don't lock the main repo's target/ directory, which
# would crash the running daemon binary loaded from there.
if [ "$(git rev-parse --is-inside-work-tree 2>/dev/null)" = "true" ]; then
	_wt_root="$(git rev-parse --show-toplevel 2>/dev/null || true)"
	_main_wt="$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null | sed 's|/\.git$||' || true)"
	if [ -n "$_main_wt" ] && [ "$_wt_root" != "$_main_wt" ]; then
		export CARGO_TARGET_DIR="/tmp/convergio-target-$(basename "$_wt_root")"
	fi
fi

source "${SCRIPT_DIR}/lib/delegate-utils.sh"
source "${SCRIPT_DIR}/lib/agent-protocol.sh"
source "${SCRIPT_DIR}/lib/worker-lifecycle.sh"
trap _worker_cleanup EXIT INT TERM

TASK_ID="${1:-}"
shift || true

MODEL="claude-opus-4-6"
TIMEOUT=1200
MAX_RETRIES=3
RETRY_DELAYS=(5 15 30)
AUTO_VALIDATE=true
PLAN_ARG=""
TASK_ARG=""

# Parse optional flags
while [[ $# -gt 0 ]]; do
	case $1 in
	--plan)
		PLAN_ARG="$2"
		shift 2
		;;
	--task)
		TASK_ARG="$2"
		shift 2
		;;
	--model)
		MODEL="$2"
		shift 2
		;;
	--timeout)
		TIMEOUT="$2"
		shift 2
		;;
	--no-auto-validate)
		AUTO_VALIDATE=false
		shift
		;;
	*) shift ;;
	esac
done

# --task flag overrides positional argument
[[ -n "$TASK_ARG" ]] && TASK_ID="$TASK_ARG"

if [[ -z "$TASK_ID" ]]; then
	echo "Usage: copilot-worker.sh --plan <plan_id> --task <db_task_id> [--model <model>] [--timeout <secs>] [--no-auto-validate]" >&2
	echo "       copilot-worker.sh <db_task_id> [options]  (deprecated: scans all plans)" >&2
	exit 1
fi

# Preflight checks — support both claude and copilot CLI
if command -v claude &>/dev/null; then
	CLI="claude"
	CLI_ARGS="-p"
elif command -v copilot &>/dev/null; then
	CLI="copilot"
	CLI_ARGS="-p"
else
	echo '{"error":"Neither claude nor copilot CLI found"}' >&2
	exit 1
fi

# Resolve task context: prefer execution-context API (--plan + --task), fall back to legacy scan
CTX_PROMPT=""
if [[ -n "$PLAN_ARG" && -n "$TASK_ARG" ]]; then
	# Fast path: use execution-context API — no plan scan needed
	_exec_ctx="$(curl -sf "${DAEMON_API}/api/plan-db/execution-context/${PLAN_ARG}?task_id=${TASK_ARG}" 2>/dev/null || echo '{}')"
	if [[ -z "$_exec_ctx" || "$_exec_ctx" == '{}' ]]; then
		echo '{"error":"execution-context API returned empty for plan '"${PLAN_ARG}"' task '"${TASK_ARG}"'"}' >&2
		exit 1
	fi
	_found_plan_id="$PLAN_ARG"
	# execution-context returns .next_task (not .task) for the target task
	_task_key=".next_task"
	STATUS="$(echo "$_exec_ctx" | jq -r "${_task_key}.status // .status // \"pending\"" 2>/dev/null || echo 'pending')"
	WT="$(echo "$_exec_ctx" | jq -r '.worktree_path // ""' 2>/dev/null || echo '')"
	WT="${WT/#\~/$HOME}"
	PLAN_ID="$PLAN_ARG"
	WAVE_DB_ID="$(echo "$_exec_ctx" | jq -r "${_task_key}.wave_id_fk // 0" 2>/dev/null || echo '0')"
	WAVE_ID="$(echo "$_exec_ctx" | jq -r "${_task_key}.wave_id // .current_wave.id // \"\"" 2>/dev/null || echo '')"
	PROJECT_ID="$(echo "$_exec_ctx" | jq -r '.project_id // ""' 2>/dev/null || echo '')"
	TASK_TYPE="$(echo "$_exec_ctx" | jq -r "${_task_key}.type // \"code\"" 2>/dev/null || echo 'code')"
	TASK_TITLE="$(echo "$_exec_ctx" | jq -r "${_task_key}.title // \"\"" 2>/dev/null || echo '')"
	TASK_DESC="$(echo "$_exec_ctx" | jq -r "${_task_key}.description // \"\"" 2>/dev/null || echo '')"
	CTX_PROMPT="$(echo "$_exec_ctx" | jq -r '.prompt // ""' 2>/dev/null || echo '')"
	AGENT_SESSION_NAME="worker-${PLAN_ID}-${TASK_ID}"
else
	echo '{"error":"--plan and --task flags required (legacy scan removed)"}' >&2
	exit 1
fi

# Fail-loud guard: task must have a title or description to execute
if [[ -z "$TASK_TITLE" && -z "$TASK_DESC" ]]; then
	echo '{"error":"task has no title or description — cannot execute without instructions"}' >&2
	exit 78
fi

# Validate status allows execution
if [[ "$STATUS" != "pending" && "$STATUS" != "in_progress" && "$STATUS" != "submitted" ]]; then
	echo "{\"error\":\"task status is ${STATUS}, expected pending/in_progress\"}" >&2
	exit 1
fi

# Agent activity tracking — register this worker in brain visualization
AGENT_ID="${AGENT_SESSION_NAME}"
"$SCRIPT_DIR/plan-db.sh" agent-start "$AGENT_ID" "copilot" "${TASK_TITLE:-task-$TASK_ID}" \
	--task "$TASK_ID" --plan "$PLAN_ID" --model "$MODEL" --host "$(hostname -s)" 2>/dev/null || true

# IPC registration so the agent appears in platform dashboards
curl -sf -X POST "${DAEMON_API}/api/ipc/agents/register" \
	-H 'Content-Type: application/json' \
	-d "{\"name\":\"${AGENT_SESSION_NAME}\",\"type\":\"copilot\",\"model\":\"${MODEL}\",\"capabilities\":[\"code\",\"test\",\"review\"]}" \
	>/dev/null 2>&1 || true

# Start background inbox polling — receives messages sent to this agent while idle
_poll_messages "${AGENT_SESSION_NAME}" &
_WORKER_CHILD_PIDS+=("$!")

# Emit agent_started mesh event for coordinator
_emit_mesh_event "agent_started" \
	"{\"task_id\":${TASK_ID},\"agent_id\":\"${AGENT_ID}\",\"model\":\"${MODEL}\"}" 2>/dev/null || true

# Generate prompt: use execution-context API prompt when available, else call prompt script
if [[ -n "$CTX_PROMPT" ]]; then
	PROMPT="$CTX_PROMPT"
else
	if [[ ! -x "$CONTEXT_LOADER" ]]; then
		echo "Warning: agent-context-loader.sh not executable, continuing with default prompt context." >&2
	fi
	PROMPT=$("$SCRIPT_DIR/copilot-task-prompt.sh" "$TASK_ID" "$AGENT_ROLE")
fi
PROMPT_TOKENS="$(_ap_tokens "$PROMPT" 2>/dev/null || echo 0)"

echo "Launching Copilot worker for task $TASK_ID (timeout: ${TIMEOUT}s, max retries: $MAX_RETRIES)..."

# Set CWD to worktree BEFORE entering execution loop (subshells inherit this)
if [[ -n "$WT" && -d "$WT" ]]; then
	echo "CWD: $WT"
	cd "$WT"
fi

if [[ -n "$WT" && -d "$WT" && -x "${SCRIPT_DIR}/execution-preflight.sh" ]]; then
	PRECHECK_JSON="$("${SCRIPT_DIR}/execution-preflight.sh" --plan-id "$PLAN_ID" "$WT" 2>/dev/null || echo '{}')"
	if echo "$PRECHECK_JSON" | jq -e '.warnings | index("dirty_worktree")' >/dev/null 2>&1; then
		echo '{"error":"dirty worktree detected by execution-preflight"}' >&2
		exit 1
	fi
fi

# Execute with retry logic for timeout (exit 124)
execute_copilot() {
	local attempt="${1:-1}"
	local exit_code=0
	local start_ts copilot_stdout_file

	start_ts="$(date +%s)"
	copilot_stdout_file="$(mktemp)"
	_WORKER_TMPFILES+=("$copilot_stdout_file")

	# Pipe copilot output to tee: file + stderr (visible to user)
	# Track child PID for cleanup on parent exit
	timeout "$TIMEOUT" $CLI $CLI_ARGS --add-dir "$WT" \
		--model "$MODEL" "$PROMPT" 2>&1 | tee "$copilot_stdout_file" >&2 &
	local copilot_bg_pid=$!
	_WORKER_CHILD_PIDS+=("$copilot_bg_pid")
	wait "$copilot_bg_pid" || true
	exit_code=$?

	echo "$exit_code|$copilot_stdout_file|$(($(date +%s) - start_ts))"
}

# Main execution loop with retry logic
ATTEMPT=1
TOTAL_DURATION=0
FINAL_EXIT_CODE=0
COPILOT_OUTPUT=""

while [[ $ATTEMPT -le $MAX_RETRIES ]]; do
	echo "Attempt $ATTEMPT/$MAX_RETRIES..."

	EXEC_RESULT=$(execute_copilot "$ATTEMPT")
	EXEC_EXIT_CODE="${EXEC_RESULT%%|*}"
	EXEC_STDOUT_FILE="${EXEC_RESULT#*|}"
	EXEC_STDOUT_FILE="${EXEC_STDOUT_FILE%|*}"
	EXEC_DURATION="${EXEC_RESULT##*|}"
	TOTAL_DURATION=$((TOTAL_DURATION + EXEC_DURATION))
	COPILOT_OUTPUT="$(<"$EXEC_STDOUT_FILE")"

	# Exit codes: 0=success, 1=error, 124=timeout, 130=interrupted

	if [[ "$EXEC_EXIT_CODE" -eq 0 ]]; then
		FINAL_EXIT_CODE=0
		rm -f "$EXEC_STDOUT_FILE"
		break
	elif [[ "$EXEC_EXIT_CODE" -eq 124 ]]; then
		rm -f "$EXEC_STDOUT_FILE"
		if [[ $ATTEMPT -lt $MAX_RETRIES ]]; then
			RETRY_DELAY="${RETRY_DELAYS[$((ATTEMPT - 1))]}"
			echo "Timeout (exit 124). Retrying in ${RETRY_DELAY}s..." >&2
			sleep "$RETRY_DELAY"
			((ATTEMPT++))
		else
			echo "Timeout after $MAX_RETRIES attempts. Giving up." >&2
			FINAL_EXIT_CODE=124
			break
		fi
	elif [[ "$EXEC_EXIT_CODE" -eq 130 ]]; then
		echo "Interrupted by user (exit 130)." >&2
		FINAL_EXIT_CODE=130
		rm -f "$EXEC_STDOUT_FILE"
		break
	else
		echo "Copilot failed with exit code $EXEC_EXIT_CODE." >&2
		FINAL_EXIT_CODE="$EXEC_EXIT_CODE"
		rm -f "$EXEC_STDOUT_FILE"
		break
	fi
done

# Post-processing: token extraction, status update, delegation log, auto-validate
source "${SCRIPT_DIR}/lib/worker-postprocess.sh"
