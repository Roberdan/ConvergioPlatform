#!/usr/bin/env bash
# worker-postprocess.sh — Post-execution processing for copilot-worker.
# Extracts tokens, updates task status, logs delegation, auto-validates wave.
# Sourced by copilot-worker.sh after CLI execution completes.
# Required vars: EXIT_CODE, FINAL_EXIT_CODE, TOTAL_DURATION, COPILOT_OUTPUT,
#   TASK_ID, PLAN_ID, PROJECT_ID, TASK_TITLE, TASK_TYPE, MODEL, WT,
#   PROMPT, WAVE_DB_ID, WAVE_ID, _found_plan_id, AUTO_VALIDATE,
#   TIMEOUT, CLI, CLI_ARGS, ATTEMPT, SCRIPT_DIR

EXIT_CODE="$FINAL_EXIT_CODE"
START_TS="$(($(date +%s) - TOTAL_DURATION))"

# Parse worker output and extract token usage
WORKER_RESULT_JSON="$(echo "$COPILOT_OUTPUT" | parse_worker_result 2>/dev/null || echo '{}')"
TOKENS_USED="$(echo "$WORKER_RESULT_JSON" | jq -r '.tokens_used // 0' 2>/dev/null || echo 0)"

if [[ "$TOKENS_USED" == "0" || "$TOKENS_USED" == "" ]]; then
	if [[ "$COPILOT_OUTPUT" =~ [Tt]okens[[:space:]]*used[[:space:]]*:[[:space:]]*([0-9]+) ]]; then
		TOKENS_USED="${BASH_REMATCH[1]}"
	elif [[ "$COPILOT_OUTPUT" =~ [Ii]nput[[:space:]]*tokens[[:space:]]*:[[:space:]]*([0-9]+).*[Oo]utput[[:space:]]*tokens[[:space:]]*:[[:space:]]*([0-9]+) ]]; then
		TOKENS_USED=$((${BASH_REMATCH[1]} + ${BASH_REMATCH[2]}))
	elif [[ "$COPILOT_OUTPUT" =~ \"usage\"[[:space:]]*:[[:space:]]*\{[^}]*\"total_tokens\"[[:space:]]*:[[:space:]]*([0-9]+) ]]; then
		TOKENS_USED="${BASH_REMATCH[1]}"
	fi
fi

if [[ "$TOKENS_USED" == "0" || "$TOKENS_USED" == "" ]]; then
	PROMPT_SIZE=${#PROMPT}
	OUTPUT_SIZE=${#COPILOT_OUTPUT}
	TOTAL_SIZE=$((PROMPT_SIZE + OUTPUT_SIZE))
	TOKENS_USED=$((TOTAL_SIZE / 4))
	[[ $TOKENS_USED -lt 1 ]] && TOKENS_USED=1
fi

# Process results and update task status based on exit code
FINAL_STATUS="$(cvg plan show "$_found_plan_id" 2>/dev/null \
	| jq -r --argjson tid "$TASK_ID" '.tasks[] | select(.id == $tid) | .status // ""' 2>/dev/null || echo '')"
NOTE=""
THOR_RESULT="UNKNOWN"
STASH_REF=""

if [[ "$EXIT_CODE" -eq 124 ]]; then
	if verify_work_done "$WT" >/dev/null 2>&1; then
		(cd "$WT" && git stash push --include-untracked \
			--message "copilot-worker timeout task ${TASK_ID}") >/dev/null 2>&1 || true
		STASH_REF="$(git -C "$WT" rev-parse --verify --short stash@{0} 2>/dev/null || true)"
	fi
	NOTE="Timeout after $ATTEMPT attempts (${TOTAL_DURATION}s total)"
	[[ -n "$STASH_REF" ]] && NOTE="${NOTE}; stash=${STASH_REF}"
	safe_update_task "$TASK_ID" blocked "$NOTE" --tokens "$TOKENS_USED" || true
	echo "{\"status\":\"timeout\",\"task_id\":${TASK_ID},\"attempts\":${ATTEMPT},\"stash_ref\":\"${STASH_REF}\"}" >&2
	THOR_RESULT="REJECT"
elif [[ "$EXIT_CODE" -eq 130 ]]; then
	NOTE="Interrupted by user"
	safe_update_task "$TASK_ID" blocked "$NOTE" --tokens "$TOKENS_USED" || true
	echo "{\"status\":\"interrupted\",\"task_id\":${TASK_ID}}" >&2
	THOR_RESULT="REJECT"
elif [[ "$EXIT_CODE" -ne 0 ]]; then
	NOTE="Copilot error (exit $EXIT_CODE)"
	safe_update_task "$TASK_ID" blocked "$NOTE" --tokens "$TOKENS_USED" || true
	echo "{\"status\":\"error\",\"task_id\":${TASK_ID},\"exit_code\":${EXIT_CODE}}" >&2
	THOR_RESULT="REJECT"
elif [[ "$FINAL_STATUS" != "done" && "$FINAL_STATUS" != "submitted" ]]; then
	_title_lower="$(echo "$TASK_TITLE" | tr '[:upper:]' '[:lower:]')"
	IS_VERIFY_TASK=false
	if [[ "$TASK_TYPE" == "chore" && "$_title_lower" == create\ pr* ]]; then IS_VERIFY_TASK=true; fi
	if [[ "$TASK_TYPE" == "test" ]] && [[ "$_title_lower" == verify* || "$_title_lower" == consolidate\ and\ verify* || "$_title_lower" == run\ full\ validation* ]]; then IS_VERIFY_TASK=true; fi
	if [[ "$TASK_TYPE" == "doc" || "$TASK_TYPE" == "docs" ]]; then IS_VERIFY_TASK=true; fi
	if WORK_DONE="$(verify_work_done "$WT" 2>/dev/null)"; then
		ARTIFACTS_JSON="$(git -C "$WT" status --porcelain | awk '{print $2}' | jq -Rsc 'split("\n") | map(select(length>0)) | unique')"
		OUTPUT_DATA="$(jq -cn --arg summary 'Auto-completed from detected worktree changes' --argjson artifacts "$ARTIFACTS_JSON" '{summary:$summary,artifacts:$artifacts}')"
		NOTE="Auto-completed: worker changed files but task status was not updated"
		safe_update_task "$TASK_ID" submitted "$NOTE" --tokens "$TOKENS_USED" --output-data "$OUTPUT_DATA" || true
		FINAL_STATUS="submitted"
		THOR_RESULT="PENDING"
		echo '{"status":"submitted","task_id":'$TASK_ID',"copilot_exit":'$EXIT_CODE'}'
	elif [[ "$IS_VERIFY_TASK" == true && "$EXIT_CODE" -eq 0 ]]; then
		NOTE="Auto-completed: verification/closure task with clean exit (no file changes expected)"
		OUTPUT_DATA='{"summary":"Verification task completed without file changes","artifacts":[]}'
		safe_update_task "$TASK_ID" submitted "$NOTE" --tokens "$TOKENS_USED" --output-data "$OUTPUT_DATA" || true
		FINAL_STATUS="submitted"
		THOR_RESULT="PENDING"
		echo '{"status":"submitted","task_id":'$TASK_ID',"copilot_exit":'$EXIT_CODE'}'
	else
		NOTE="Copilot exited without completing"
		safe_update_task "$TASK_ID" blocked "$NOTE" --tokens "$TOKENS_USED" || true
		echo '{"status":"incomplete","task_id":'$TASK_ID',"copilot_exit":'$EXIT_CODE'}' >&2
		THOR_RESULT="REJECT"
	fi
else
	echo '{"status":"'$FINAL_STATUS'","task_id":'$TASK_ID'}'
	if [[ "$FINAL_STATUS" == "done" ]]; then
		THOR_RESULT="PASS"
	else
		THOR_RESULT="PENDING"
	fi
fi

DURATION_MS="$((TOTAL_DURATION * 1000))"
log_delegation "$TASK_ID" "$PLAN_ID" "$PROJECT_ID" "copilot" "$MODEL" \
	"$PROMPT_TOKENS" "$TOKENS_USED" "$DURATION_MS" "$EXIT_CODE" "$THOR_RESULT" "0" "unknown" || true

complete_agent_tracking

# F-12: Auto-trigger @validate when wave fully submitted
if [[ "$AUTO_VALIDATE" == "true" && "$FINAL_STATUS" == "submitted" && "$WAVE_DB_ID" != "0" ]]; then
	eval_json="$("$SCRIPT_DIR/plan-db.sh" evaluate-wave "$WAVE_DB_ID" 2>/dev/null || echo '{"result":"BLOCKED"}')"
	eval_result="$(echo "$eval_json" | jq -r '.result // "BLOCKED"' 2>/dev/null || echo "BLOCKED")"
	_wave_tasks="$(cvg plan show "$_found_plan_id" 2>/dev/null | jq --argjson wid "$WAVE_DB_ID" '[.tasks[] | select(.wave_id_fk == $wid)]' 2>/dev/null || echo '[]')"
	unresolved_count="$(echo "$_wave_tasks" | jq '[.[] | select(.status | IN("submitted","done","cancelled","skipped") | not)] | length' 2>/dev/null || echo "1")"
	submitted_count="$(echo "$_wave_tasks" | jq '[.[] | select(.status == "submitted")] | length' 2>/dev/null || echo "0")"
	if [[ "$eval_result" == "READY" && "$unresolved_count" -eq 0 && "$submitted_count" -gt 0 ]]; then
		validate_prompt="@validate Wave ${WAVE_ID:-$WAVE_DB_ID} in plan ${PLAN_ID}. All wave tasks are submitted. Run wave-level validation now."
		echo "Auto-validate: wave ${WAVE_ID:-$WAVE_DB_ID} is fully submitted. Triggering @validate..."
		timeout "$TIMEOUT" $CLI $CLI_ARGS --add-dir "$WT" \
			-p "$validate_prompt" >/dev/null 2>&1 || {
			echo "WARN: Auto-validate trigger failed for wave ${WAVE_ID:-$WAVE_DB_ID}" >&2
		}
	fi
elif [[ "$AUTO_VALIDATE" != "true" ]]; then
	echo "Auto-validate disabled via --no-auto-validate."
fi

exit $EXIT_CODE
