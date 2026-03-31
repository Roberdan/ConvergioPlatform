#!/bin/bash
# execute-plan-runner.sh - API-based helpers, prompt builder, and per-task run logic
# Sourced by execute-plan-engine.sh
# Version: 2.0.0 — ALL access via daemon HTTP API (zero sqlite3)
# API: GET /api/plan-db/execution-tree/:plan_id returns {plan, tree[{wave, tasks[]}]}

DAEMON_URL="${DAEMON_URL:-http://localhost:8420}"

# ============================================================================
# Fetch full plan data once and cache (avoids repeated API calls)
# ============================================================================
_PLAN_CACHE=""
_fetch_plan() {
	if [[ -z "$_PLAN_CACHE" ]]; then
		_PLAN_CACHE=$(curl -sf "${DAEMON_URL}/api/plan-db/execution-tree/${PLAN_ID}" 2>/dev/null)
	fi
	echo "$_PLAN_CACHE"
}

# ============================================================================
# Wave/task helpers (parse cached JSON)
# ============================================================================
get_waves() {
	_fetch_plan | jq -r '.tree[]? | .tasks[0] as $t |
		"\($t.wave_id_fk)|\(.wave_id // .code // "?")|\(.name // "")|\(.status // "pending")"' 2>/dev/null
	# Fallback: try flat waves array
	if [[ ${PIPESTATUS[1]} -ne 0 ]]; then
		_fetch_plan | jq -r '.waves[]? | "\(.id)|\(.wave_id // .code)|\(.name)|\(.status)"' 2>/dev/null
	fi
}

get_wave_tasks() {
	local wave_db_id="$1"
	_fetch_plan | jq -r --argjson wid "$wave_db_id" \
		'.tree[]? | select(.tasks[0].wave_id_fk == $wid) | .tasks[]? |
		"\(.id)|\(.task_id)|\(.status)|\(.title | gsub("\n";" ") | .[0:80])"' 2>/dev/null
	if [[ ${PIPESTATUS[1]} -ne 0 ]]; then
		_fetch_plan | jq -r --argjson wid "$wave_db_id" \
			'.tasks[]? | select(.wave_id_fk == $wid) |
			"\(.id)|\(.task_id)|\(.status)|\(.title | gsub("\n";" ") | .[0:80])"' 2>/dev/null
	fi
}

# Force re-fetch (after task status change)
refresh_plan_cache() {
	_PLAN_CACHE=""
	_fetch_plan > /dev/null
}

# ============================================================================
# Generate task prompt for execution
# ============================================================================
build_task_prompt() {
	local task_db_id="$1"
	if [[ -x "${SCRIPT_DIR}/copilot-task-prompt.sh" ]]; then
		"${SCRIPT_DIR}/copilot-task-prompt.sh" "$task_db_id" 2>/dev/null
	else
		_fetch_plan | jq -r --argjson tid "$task_db_id" \
			'[.tree[].tasks[] | select(.id == $tid)][0] //
			 [.tasks[] | select(.id == $tid)][0] |
			"Task: \(.task_id)\nTitle: \(.title)\nModel: \(.model // "")\nTest: \(.test_criteria // "")"' 2>/dev/null
	fi
}

# ============================================================================
# Get task status (fresh from API, not cache)
# ============================================================================
get_task_status() {
	local task_db_id="$1"
	refresh_plan_cache
	_fetch_plan | jq -r --argjson tid "$task_db_id" \
		'[.tree[].tasks[] | select(.id == $tid)][0].status // "unknown"' 2>/dev/null
}

# ============================================================================
# Execute a single task via the selected engine
# ============================================================================
run_task() {
	local task_db_id="$1"
	local task_code="$2"

	# Resolve worktree from plan data
	local worktree
	worktree=$(_fetch_plan | jq -r '.plan.worktree_path // ""' 2>/dev/null)
	worktree="${worktree/#\~/$HOME}"

	if [[ "$DRY_RUN" -eq 1 ]]; then
		step "DRY-RUN: would execute $task_code via $ENGINE"
		return 0
	fi

	local prompt
	prompt="$(build_task_prompt "$task_db_id")"
	local exit_code=0

	# Resolve model from plan data
	local effective_model="${MODEL:-}"
	if [[ -z "$effective_model" ]]; then
		effective_model=$(_fetch_plan | jq -r --argjson tid "$task_db_id" \
			'[.tree[].tasks[] | select(.id == $tid)][0].model // ""' 2>/dev/null)
	fi
	case "$ENGINE" in
		claude) [[ "$effective_model" == gpt-* ]] && effective_model="claude-sonnet-4-6" ;;
	esac

	# --- Strategy 1: delegate.sh ---
	if [[ -n "${DELEGATE_SH:-}" && -x "$DELEGATE_SH" ]]; then
		step "Executing via delegate.sh (engine: $ENGINE)"
		local model_flag=""
		[[ -n "$effective_model" ]] && model_flag="--model $effective_model"
		timeout "$TASK_TIMEOUT" "$DELEGATE_SH" "$task_db_id" \
			--engine "$ENGINE" $model_flag || exit_code=$?
		return $exit_code
	fi

	# --- Strategy 2: engine-specific ---
	case "$ENGINE" in
	copilot)
		step "Executing via copilot-worker.sh"
		if [[ -n "${COPILOT_WORKER:-}" && -x "$COPILOT_WORKER" ]]; then
			local worker_args=("$task_db_id")
			[[ -n "$effective_model" ]] && worker_args+=(--model "$effective_model")
			worker_args+=(--timeout "$TASK_TIMEOUT")
			timeout "$TASK_TIMEOUT" "$COPILOT_WORKER" "${worker_args[@]}" || exit_code=$?
		else
			local model_flag="" dir_flag=""
			[[ -n "$effective_model" ]] && model_flag="--model $effective_model"
			[[ -n "$worktree" && -d "$worktree" ]] && dir_flag="--add-dir $worktree"
			timeout "$TASK_TIMEOUT" copilot --allow-all --no-ask-user \
				$dir_flag $model_flag -p "$prompt" || exit_code=$?
		fi ;;
	opencode)
		step "Executing via opencode"
		local model_flag="" cwd_flag=""
		[[ -n "$effective_model" ]] && model_flag="--model $effective_model"
		[[ -n "$worktree" && -d "$worktree" ]] && cwd_flag="--cwd $worktree"
		timeout "$TASK_TIMEOUT" opencode $cwd_flag $model_flag \
			--prompt "$prompt" || exit_code=$? ;;
	claude|*)
		step "Executing via claude CLI"
		local model_flag="" dir_flag=""
		[[ -n "$effective_model" ]] && model_flag="--model $effective_model"
		[[ -n "$worktree" && -d "$worktree" ]] && dir_flag="--add-dir $worktree"
		[[ -n "$worktree" && -d "$worktree" ]] && cd "$worktree"
		timeout "$TASK_TIMEOUT" claude $dir_flag $model_flag \
			-p "$prompt" || exit_code=$? ;;
	esac

	return $exit_code
}
