#!/bin/bash
# execute-plan-engine.sh - Wave execution loop, validation, and resume logic
# Extracted from execute-plan.sh for modularization
# Version: 1.3.0 - Split runner helpers; plan-level done gate

# Source per-task runner helpers (DB helpers, prompt builder, run_task)
# shellcheck source=execute-plan-runner.sh
source "$(dirname "${BASH_SOURCE[0]}")/execute-plan-runner.sh"

# ============================================================================
# Thor per-task validation
# ============================================================================
validate_task() {
	local task_db_id="$1"
	local task_code="$2"

	if [[ "$DRY_RUN" -eq 1 ]]; then
		step "DRY-RUN: would validate task $task_code via Thor"
		return 0
	fi

	step "Thor per-task validation: $task_code"
	if "${SCRIPT_DIR}/plan-db.sh" validate-task "$task_db_id" "$PLAN_ID" "execute-plan" 2>&1; then
		success "Thor: task $task_code PASS"
		return 0
	else
		warn "Thor: task $task_code REJECTED"
		return 1
	fi
}

# ============================================================================
# Thor per-wave validation
# ============================================================================
validate_wave() {
	local wave_db_id="$1"
	local wave_code="$2"

	if [[ "$DRY_RUN" -eq 1 ]]; then
		step "DRY-RUN: would validate wave $wave_code via Thor"
		return 0
	fi

	step "Thor per-wave validation: $wave_code"
	if "${SCRIPT_DIR}/plan-db.sh" validate-wave "$wave_db_id" "execute-plan" 2>&1; then
		success "Thor: wave $wave_code PASS"
		return 0
	else
		warn "Thor: wave $wave_code REJECTED"
		return 1
	fi
}

# ============================================================================
# Resume logic: find tasks to skip
# ============================================================================
SKIP_UNTIL_TASK=""

init_resume() {
	local from_task="$1"
	if [[ -n "$from_task" ]]; then
		SKIP_UNTIL_TASK="$from_task"
		log "Resume mode: skipping tasks before $from_task"
	fi
}

should_skip_task() {
	local task_code="$1"
	if [[ -n "$SKIP_UNTIL_TASK" ]]; then
		if [[ "$task_code" == "$SKIP_UNTIL_TASK" ]]; then
			# Found the start task — stop skipping
			SKIP_UNTIL_TASK=""
			return 1 # do NOT skip this task
		fi
		return 0 # skip
	fi
	return 1 # do NOT skip
}

# ============================================================================
# Main execution loop
# ============================================================================
execute_plan_waves() {
	local TOTAL_TASKS=0
	local DONE_TASKS=0
	local SKIPPED_TASKS=0
	local FAILED_TASKS=0

	# Plan-level done gate: skip execution if all waves already completed
	local all_done=1
	while IFS='|' read -r _wid _wcode _wname wstatus; do
		if [[ "$wstatus" != "done" ]]; then
			all_done=0
			break
		fi
	done < <(get_waves)
	if [[ "$all_done" -eq 1 ]]; then
		success "Plan $PLAN_ID already completed — all waves done"
		return 0
	fi

	while IFS='|' read -r wave_db_id wave_code wave_name wave_status; do
		echo ""
		log "=== Wave: $wave_code - $wave_name (status: $wave_status) ==="

		# Wave-per-worktree: create worktree for this wave if not exists
		if [[ -x "${SCRIPT_DIR}/wave-worktree.sh" && "$wave_status" != "done" ]]; then
			local wave_wt
			wave_wt=$(db_query "$DB_FILE" "SELECT COALESCE(worktree_path,'') FROM waves WHERE id=$wave_db_id;")
			if [[ -z "$wave_wt" ]]; then
				step "Creating wave worktree for $wave_code"
				"${SCRIPT_DIR}/wave-worktree.sh" create "$PLAN_ID" "$wave_db_id" 2>&1 || {
					warn "Failed to create wave worktree for $wave_code — using plan worktree"
				}
			fi
		fi

		# Skip already-completed waves (unless --from forces re-entry)
		if [[ "$wave_status" == "done" && -z "$FROM_TASK" ]]; then
			success "Wave $wave_code already done — skipping"
			continue
		fi

		# Track whether any tasks ran in this wave
		local wave_had_tasks=0
		local wave_failed=0

		while IFS='|' read -r task_db_id task_code task_status task_title; do
			TOTAL_TASKS=$((TOTAL_TASKS + 1))

			# Resume: skip tasks before --from target
			if should_skip_task "$task_code"; then
				step "Skipping $task_code (before resume point)"
				SKIPPED_TASKS=$((SKIPPED_TASKS + 1))
				continue
			fi

			# Skip already-done tasks
			if [[ "$task_status" == "done" && -z "$FROM_TASK" ]]; then
				step "$task_code: already done — skipping"
				DONE_TASKS=$((DONE_TASKS + 1))
				continue
			fi

			# Skip blocked tasks
			if [[ "$task_status" == "blocked" ]]; then
				warn "$task_code: blocked — skipping"
				FAILED_TASKS=$((FAILED_TASKS + 1))
				wave_failed=$((wave_failed + 1))
				continue
			fi

			wave_had_tasks=1
			step "Executing $task_code: $(echo "$task_title" | cut -c1-60)"

			# Run the task
			local task_exit=0
			run_task "$task_db_id" "$task_code" || task_exit=$?

			if [[ "$DRY_RUN" -eq 1 ]]; then
				DONE_TASKS=$((DONE_TASKS + 1))
				continue
			fi

			# Verify task status after execution
			local new_status
			new_status=$(db_query "$DB_FILE" "SELECT status FROM tasks WHERE id=$task_db_id;")

			if [[ "$new_status" == "done" || "$new_status" == "submitted" ]]; then
				# Per-task validation deferred to wave-level Thor (Opus)
				# Mechanical gates (test/typecheck/lint) already enforced by executor
				success "Task $task_code complete (status=$new_status)"
				DONE_TASKS=$((DONE_TASKS + 1))
			else
				warn "Task $task_code ended with status=$new_status (exit=$task_exit)"
				FAILED_TASKS=$((FAILED_TASKS + 1))
				wave_failed=$((wave_failed + 1))
				warn "Stopping wave — fix task $task_code before continuing"
				break
			fi

		done < <(get_wave_tasks "$wave_db_id")

		# Per-wave Thor validation (only if tasks ran and none failed)
		if [[ "$wave_had_tasks" -eq 1 && "$wave_failed" -eq 0 ]]; then
			echo ""
			local thor_wave_exit=0
			validate_wave "$wave_db_id" "$wave_code" || thor_wave_exit=$?

			if [[ "$thor_wave_exit" -ne 0 ]]; then
				warn "Wave $wave_code failed Thor validation — stopping execution"
				error "Fix wave issues before continuing. Resume with: execute-plan.sh $PLAN_ID --from <first-failed-task>"
				break
			fi

			# Wave-per-worktree: merge via PR after successful Thor validation
			if [[ "$DRY_RUN" -eq 0 && -x "${SCRIPT_DIR}/wave-worktree.sh" ]]; then
				local wave_wt_check
				wave_wt_check=$(db_query "$DB_FILE" "SELECT COALESCE(worktree_path,'') FROM waves WHERE id=$wave_db_id;")
				if [[ -n "$wave_wt_check" ]]; then
					step "Wave $wave_code: merging via PR..."
					if ! "${SCRIPT_DIR}/wave-worktree.sh" merge "$PLAN_ID" "$wave_db_id" 2>&1; then
						warn "Wave $wave_code merge failed — attempting auto-fix..."
						# Auto-fix: install missing tools, retry
						_preflight_check 2>/dev/null || true
						if ! "${SCRIPT_DIR}/wave-worktree.sh" merge "$PLAN_ID" "$wave_db_id" 2>&1; then
							warn "Wave $wave_code merge still failing — continuing to next wave"
						fi
					fi
				fi
			fi

			# DB sync: push dashboard.db snapshot to coordinator after wave done
			if [[ "$DRY_RUN" -eq 0 && -x "${SCRIPT_DIR}/mesh-sync-all.sh" ]]; then
				step "Syncing DB to mesh peers after wave $wave_code done..."
				"${SCRIPT_DIR}/mesh-sync-all.sh" --phase config 2>&1 || {
					warn "DB sync failed (non-blocking) — coordinator may have stale data"
				}
			fi
		elif [[ "$wave_failed" -gt 0 ]]; then
			warn "Wave $wave_code had $wave_failed failed task(s) — STOPPING execution"
			error "Fix failed/blocked tasks before continuing."
			error "Resume with: execute-plan.sh $PLAN_ID --from <first-failed-task> --engine $ENGINE"
			break
		fi

	done < <(get_waves)

	# Summary
	echo ""
	log "=== EXECUTION SUMMARY ==="
	log "  Total tasks:   $TOTAL_TASKS"
	log "  Done:          $DONE_TASKS"
	log "  Skipped:       $SKIPPED_TASKS"
	log "  Failed:        $FAILED_TASKS"
	log "  Log:           $LOG_FILE"
	echo ""

	if [[ "$FAILED_TASKS" -gt 0 ]]; then
		warn "$FAILED_TASKS task(s) failed or blocked"
		warn "Check log: $LOG_FILE"
		warn "Resume with: execute-plan.sh $PLAN_ID --from <task_id> --engine $ENGINE"
		return 1
	else
		success "Plan $PLAN_ID execution complete"
		return 0
	fi
}
