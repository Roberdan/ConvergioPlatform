#!/bin/bash
# execute-plan-engine.sh - Wave execution loop, validation, and resume logic
# Version: 2.0.0 - All DB access via daemon HTTP API (zero sqlite3)

# Source per-task runner helpers (API-based helpers, prompt builder, run_task)
# shellcheck source=execute-plan-runner.sh
source "$(dirname "${BASH_SOURCE[0]}")/execute-plan-runner.sh"

# ============================================================================
# Thor per-task validation (via daemon API)
# ============================================================================
validate_task() {
	local task_db_id="$1"
	local task_code="$2"

	if [[ "$DRY_RUN" -eq 1 ]]; then
		step "DRY-RUN: would validate task $task_code via Thor"
		return 0
	fi

	step "Thor per-task validation: $task_code"
	local result
	result=$(curl -sf -X POST "${DAEMON_URL}/api/validation/enqueue" \
		-H 'Content-Type: application/json' \
		-d "{\"task_id\":${task_db_id},\"plan_id\":${PLAN_ID},\"validator\":\"execute-plan\"}" 2>/dev/null)
	if echo "$result" | jq -e '.ok // false' &>/dev/null; then
		success "Thor: task $task_code PASS"
		return 0
	else
		warn "Thor: task $task_code REJECTED"
		return 1
	fi
}

# ============================================================================
# Thor per-wave validation (via daemon API)
# ============================================================================
validate_wave() {
	local wave_db_id="$1"
	local wave_code="$2"

	if [[ "$DRY_RUN" -eq 1 ]]; then
		step "DRY-RUN: would validate wave $wave_code via Thor"
		return 0
	fi

	step "Thor per-wave validation: $wave_code"
	local result
	result=$(curl -sf -X POST "${DAEMON_URL}/api/validation/enqueue" \
		-H 'Content-Type: application/json' \
		-d "{\"wave_id\":${wave_db_id},\"validator\":\"execute-plan\"}" 2>/dev/null)
	if echo "$result" | jq -e '.ok // false' &>/dev/null; then
		success "Thor: wave $wave_code PASS"
		return 0
	else
		warn "Thor: wave $wave_code REJECTED"
		return 1
	fi
}

# ============================================================================
# Resume logic
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
			SKIP_UNTIL_TASK=""
			return 1
		fi
		return 0
	fi
	return 1
}

# ============================================================================
# Main execution loop (all data via daemon API)
# ============================================================================
execute_plan_waves() {
	local TOTAL_TASKS=0
	local DONE_TASKS=0
	local SKIPPED_TASKS=0
	local FAILED_TASKS=0

	# Plan-level done gate
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

		# Skip already-completed waves
		if [[ "$wave_status" == "done" && -z "$FROM_TASK" ]]; then
			success "Wave $wave_code already done — skipping"
			continue
		fi

		local wave_had_tasks=0
		local wave_failed=0

		while IFS='|' read -r task_db_id task_code task_status task_title; do
			TOTAL_TASKS=$((TOTAL_TASKS + 1))

			if should_skip_task "$task_code"; then
				step "Skipping $task_code (before resume point)"
				SKIPPED_TASKS=$((SKIPPED_TASKS + 1))
				continue
			fi

			if [[ "$task_status" == "done" && -z "$FROM_TASK" ]]; then
				step "$task_code: already done — skipping"
				DONE_TASKS=$((DONE_TASKS + 1))
				continue
			fi

			if [[ "$task_status" == "blocked" ]]; then
				warn "$task_code: blocked — skipping"
				FAILED_TASKS=$((FAILED_TASKS + 1))
				wave_failed=$((wave_failed + 1))
				continue
			fi

			wave_had_tasks=1
			step "Executing $task_code: $(echo "$task_title" | cut -c1-60)"

			local task_exit=0
			run_task "$task_db_id" "$task_code" || task_exit=$?

			if [[ "$DRY_RUN" -eq 1 ]]; then
				DONE_TASKS=$((DONE_TASKS + 1))
				continue
			fi

			# Verify task status via API (refresh cache after execution)
			local new_status
			new_status=$(get_task_status "$task_db_id")

			if [[ "$new_status" == "done" || "$new_status" == "submitted" ]]; then
				success "Task $task_code complete (status=$new_status)"
				DONE_TASKS=$((DONE_TASKS + 1))
			else
				warn "Task $task_code ended with status=$new_status (exit=$task_exit)"
				FAILED_TASKS=$((FAILED_TASKS + 1))
				wave_failed=$((wave_failed + 1))
				warn "Stopping wave — fix $task_code before continuing"
				break
			fi

		done < <(get_wave_tasks "$wave_db_id")

		# Per-wave Thor validation
		if [[ "$wave_had_tasks" -eq 1 && "$wave_failed" -eq 0 ]]; then
			echo ""
			local thor_wave_exit=0
			validate_wave "$wave_db_id" "$wave_code" || thor_wave_exit=$?

			if [[ "$thor_wave_exit" -ne 0 ]]; then
				warn "Wave $wave_code failed Thor — stopping"
				error "Resume: execute-plan.sh $PLAN_ID --from <task>"
				break
			fi
		elif [[ "$wave_failed" -gt 0 ]]; then
			warn "Wave $wave_code: $wave_failed failed — STOPPING"
			error "Resume: execute-plan.sh $PLAN_ID --from <task> --engine $ENGINE"
			break
		fi

	done < <(get_waves)

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
		warn "Resume: execute-plan.sh $PLAN_ID --from <task_id> --engine $ENGINE"
		return 1
	else
		success "Plan $PLAN_ID execution complete"
		return 0
	fi
}
