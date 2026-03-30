#!/bin/bash
# execute-plan-runner.sh - DB helpers, prompt builder, and per-task run logic
# Sourced by execute-plan-engine.sh
# Version: 1.0.0

# ============================================================================
# DB helpers
# ============================================================================
get_waves() {
	db_query "$DB_FILE" "SELECT id, wave_id, name, status FROM waves WHERE plan_id=$PLAN_ID ORDER BY id;"
}

get_wave_tasks() {
	local wave_db_id="$1"
	db_query "$DB_FILE" "SELECT id, task_id, status, title FROM tasks
		WHERE wave_id_fk=$wave_db_id
		ORDER BY id;"
}

# ============================================================================
# Generate task prompt for execution
# ============================================================================
build_task_prompt() {
	local task_db_id="$1"
	# Use existing prompt generator if available
	if [[ -x "${SCRIPT_DIR}/copilot-task-prompt.sh" ]]; then
		"${SCRIPT_DIR}/copilot-task-prompt.sh" "$task_db_id" 2>/dev/null
	else
		# Fallback: query DB directly
		db_query "$DB_FILE" "SELECT 'Task ID: '||task_id||char(10)||
			'Title: '||title||char(10)||
			'Description: '||COALESCE(description,'')||char(10)||
			'Test Criteria: '||COALESCE(test_criteria,'')
			FROM tasks WHERE id=$task_db_id;"
	fi
}

# ============================================================================
# Execute a single task via the selected engine
# ============================================================================
run_task() {
	local task_db_id="$1"
	local task_code="$2"

	# Resolve worktree: wave-level first (new model), fallback plan-level (old model)
	local worktree
	worktree=$(db_query "$DB_FILE" "SELECT COALESCE(w.worktree_path, p.worktree_path, '')
		FROM tasks t
		JOIN plans p ON t.plan_id = p.id
		LEFT JOIN waves w ON t.wave_id_fk = w.id
		WHERE t.id=$task_db_id;")
	worktree="${worktree/#\~/$HOME}"

	if [[ "$DRY_RUN" -eq 1 ]]; then
		step "DRY-RUN: would execute $task_code via $ENGINE"
		return 0
	fi

	# Build the prompt
	local prompt
	prompt="$(build_task_prompt "$task_db_id")"

	local exit_code=0

	# Resolve model: per-task model from DB, or global override, with engine compatibility
	local effective_model="${MODEL:-}"
	if [[ -z "$effective_model" ]]; then
		effective_model=$(db_query "$DB_FILE" "SELECT COALESCE(model,'') FROM tasks WHERE id=$task_db_id;")
	fi
	# Engine-model compatibility: remap incompatible models
	case "$ENGINE" in
		claude)
			if [[ "$effective_model" == gpt-* ]]; then
				effective_model="claude-sonnet-4-6"  # safe default for Claude engine
			fi
			;;
		copilot)
			# Copilot supports both claude-* and gpt-* models — no remapping needed
			;;
	esac

	# --- Strategy 1: delegate.sh (preferred) ---
	if [[ -x "$DELEGATE_SH" ]]; then
		step "Executing via delegate.sh (engine: $ENGINE)"
		local model_flag=""
		[[ -n "$effective_model" ]] && model_flag="--model $effective_model"
		# delegate.sh accepts: delegate.sh <task_db_id> [--engine <e>] [--model <m>]
		timeout "$TASK_TIMEOUT" "$DELEGATE_SH" "$task_db_id" \
			--engine "$ENGINE" $model_flag || exit_code=$?
		return $exit_code
	fi

	# --- Strategy 2: engine-specific fallback ---
	case "$ENGINE" in

	copilot)
		step "Executing via copilot-worker.sh"
		local model_arg="${effective_model:-}"
		if [[ -x "$COPILOT_WORKER" ]]; then
			local worker_args=("$task_db_id")
			[[ -n "$model_arg" ]] && worker_args+=(--model "$model_arg")
			worker_args+=(--timeout "$TASK_TIMEOUT")
			timeout "$TASK_TIMEOUT" "$COPILOT_WORKER" "${worker_args[@]}" || exit_code=$?
		else
			# Direct copilot invocation
			local model_flag=""
			[[ -n "$MODEL" ]] && model_flag="--model $MODEL"
			local dir_flag=""
			[[ -n "$worktree" && -d "$worktree" ]] && dir_flag="--add-dir $worktree"
			timeout "$TASK_TIMEOUT" copilot \
				--allow-all \
				--no-ask-user \
				--disable-mcp-server codegraph \
				$dir_flag \
				$model_flag \
				-p "$prompt" || exit_code=$?
		fi
		;;

	opencode)
		step "Executing via opencode"
		local model_flag=""
		[[ -n "$MODEL" ]] && model_flag="--model $MODEL"
		local cwd_flag=""
		[[ -n "$worktree" && -d "$worktree" ]] && cwd_flag="--cwd $worktree"
		timeout "$TASK_TIMEOUT" opencode \
			$cwd_flag \
			$model_flag \
			--prompt "$prompt" || exit_code=$?
		;;

	claude | *)
		step "Executing via claude CLI"
		local model_flag=""
		[[ -n "$MODEL" ]] && model_flag="--model $MODEL"
		local dir_flag=""
		[[ -n "$worktree" && -d "$worktree" ]] && dir_flag="--add-dir $worktree"
		# cd to worktree so file operations target correct directory
		[[ -n "$worktree" && -d "$worktree" ]] && cd "$worktree"
		# Generate per-worktree settings.json (replaces --dangerously-skip-permissions)
		if [[ -n "$worktree" && -d "$worktree" ]]; then
			local _sdir="${worktree}/.claude"
			mkdir -p "$_sdir"
			printf '{"permissions":{"allow":["Bash(cargo check:*)","Bash(cargo build:*)","Bash(cargo test:*)","Bash(git add:*)","Bash(git commit:*)","Bash(git diff:*)","Bash(git status:*)","Bash(curl http://localhost:*)","Bash(curl http://127.0.0.1:*)"]}}' \
				> "${_sdir}/settings.json"
		fi
		timeout "$TASK_TIMEOUT" claude \
			$dir_flag \
			$model_flag \
			-p "$prompt" || exit_code=$?
		;;
	esac

	return $exit_code
}
