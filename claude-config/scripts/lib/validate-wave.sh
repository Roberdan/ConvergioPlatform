#!/bin/bash
# DEPRECATED — use daemon API endpoints for wave validation instead
# Wave-level validation functions
# Version: 2.0.0 — migrated from sqlite3 to cvg CLI / daemon API
command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 1; }

DAEMON_URL="${DAEMON_URL:-http://localhost:8420}"

# Helper: fetch plan JSON that contains this wave
_vw_wave_plan_json() {
	local wave_db_id="$1"
	# Get wave info via plan list, find which plan contains this wave
	local plan_list plan_json=""
	plan_list=$(curl -sf "${DAEMON_URL}/api/plan-db/list" | jq -r '.[].id' 2>/dev/null) || return 1
	for pid in $plan_list; do
		local pj
		pj=$(curl -sf "${DAEMON_URL}/api/plan-db/json/${pid}" 2>/dev/null) || continue
		if echo "$pj" | jq -e ".waves[]? | select(.id==${wave_db_id} or .db_id==${wave_db_id})" >/dev/null 2>&1; then
			echo "$pj"
			return 0
		fi
	done
	return 1
}

# Validate all done tasks in a wave
# Usage: validate-wave <wave_db_id> [validated_by]
cmd_validate_wave() {
	local wave_db_id="$1"
	local validated_by="${2:-thor}"

	local pj
	pj=$(_vw_wave_plan_json "$wave_db_id") || {
		log_error "Wave not found: $wave_db_id"
		return 1
	}

	local wave_json
	wave_json=$(echo "$pj" | jq -c ".waves[]? | select(.id==${wave_db_id} or .db_id==${wave_db_id})" 2>/dev/null)
	if [[ -z "$wave_json" ]]; then
		log_error "Wave not found: $wave_db_id"
		return 1
	fi

	local wave_id plan_id tasks_done tasks_total
	wave_id=$(echo "$wave_json" | jq -r '.wave_id // ""')
	plan_id=$(echo "$pj" | jq -r '.id // 0')
	tasks_done=$(echo "$wave_json" | jq -r '.tasks_done // 0')
	tasks_total=$(echo "$wave_json" | jq -r '.tasks_total // 0')

	# Get tasks for this wave from plan JSON
	local wave_tasks
	wave_tasks=$(echo "$pj" | jq -c "[.tasks[]? | select(.wave_id_fk==${wave_db_id})]" 2>/dev/null || echo '[]')

	# Check for truly unresolved tasks (pending, in_progress, blocked)
	local unresolved
	unresolved=$(echo "$wave_tasks" | jq '[.[] | select(.status | IN("done","submitted","cancelled","skipped") | not)] | length' 2>/dev/null || echo "0")
	if [[ "$unresolved" -gt 0 ]]; then
		log_error "Wave $wave_id has $unresolved unresolved tasks (pending/in_progress/blocked)"
		echo "$wave_tasks" | jq -r '.[] | select(.status | IN("done","submitted","cancelled","skipped") | not) | "  - \(.task_id) (\(.status)): \(.title // "")"' 2>/dev/null || true
		return 1
	fi

	# Batch-promote submitted tasks to done (wave-level Thor validation)
	local submitted_count
	submitted_count=$(echo "$wave_tasks" | jq '[.[] | select(.status == "submitted")] | length' 2>/dev/null || echo "0")
	if [[ "$submitted_count" -gt 0 ]]; then
		log_info "Promoting $submitted_count submitted tasks to done (validated by $validated_by at wave level)"
		# Update each submitted task via daemon API
		local submitted_ids
		submitted_ids=$(echo "$wave_tasks" | jq -r '.[] | select(.status == "submitted") | .id // .db_id' 2>/dev/null)
		for tid in $submitted_ids; do
			cvg task validate "$tid" "$plan_id" 2>/dev/null || \
			curl -sf -X POST "${DAEMON_URL}/api/plan-db/task/validate" \
				-H 'Content-Type: application/json' \
				-d "{\"task_id\":${tid},\"validated_by\":\"${validated_by}\"}" >/dev/null 2>&1 || true
		done
	fi

	# Thor gate: verify all done tasks have been validated
	# Re-fetch plan JSON after updates
	pj=$(curl -sf "${DAEMON_URL}/api/plan-db/json/${plan_id}" 2>/dev/null) || pj=$(_vw_wave_plan_json "$wave_db_id") || pj='{}'
	wave_tasks=$(echo "$pj" | jq -c "[.tasks[]? | select(.wave_id_fk==${wave_db_id})]" 2>/dev/null || echo '[]')

	local unvalidated
	unvalidated=$(echo "$wave_tasks" | jq '[.[] | select(.status == "done" and .validated_at == null)] | length' 2>/dev/null || echo "0")
	if [[ "$unvalidated" -gt 0 ]]; then
		log_error "Wave $wave_id has $unvalidated done tasks without Thor validation — cannot close wave"
		echo "$wave_tasks" | jq -r '.[] | select(.status == "done" and .validated_at == null) | "  - \(.task_id): \(.title // "") (missing validation)"' 2>/dev/null || true
		return 1
	fi

	echo -e "${YELLOW}Wave $wave_id: all tasks resolved + Thor-validated — closing wave${NC}"
	# Close wave via daemon API
	curl -sf -X POST "${DAEMON_URL}/api/plan-db/wave/update" \
		-H 'Content-Type: application/json' \
		-d "{\"wave_db_id\":${wave_db_id},\"status\":\"done\"}" >/dev/null 2>&1 || true
	return 0
}

# Evaluate wave preconditions - returns READY, SKIP, or BLOCKED
# Usage: cmd_evaluate_wave <wave_db_id>
# Output: JSON to stdout
cmd_evaluate_wave() {
	local wave_db_id="$1"

	local pj
	pj=$(_vw_wave_plan_json "$wave_db_id") || {
		echo '{"result":"BLOCKED","wave_id":"?","details":[{"error":"wave not found"}]}'
		return 1
	}

	local wave_json
	wave_json=$(echo "$pj" | jq -c ".waves[]? | select(.id==${wave_db_id} or .db_id==${wave_db_id})" 2>/dev/null)
	if [[ -z "$wave_json" ]]; then
		echo '{"result":"BLOCKED","wave_id":"?","details":[{"error":"wave not found"}]}'
		return 1
	fi

	local plan_id wave_id precondition
	plan_id=$(echo "$pj" | jq -r '.id // 0')
	wave_id=$(echo "$wave_json" | jq -r '.wave_id // ""')
	precondition=$(echo "$wave_json" | jq -r '.precondition // "null"')

	if [[ -z "$precondition" || "$precondition" == "null" ]]; then
		echo "{\"result\":\"READY\",\"wave_id\":\"$wave_id\",\"details\":[]}"
		return 0
	fi

	if ! echo "$precondition" | jq -e '.' >/dev/null 2>&1; then
		echo "{\"result\":\"BLOCKED\",\"wave_id\":\"$wave_id\",\"details\":[{\"error\":\"invalid precondition JSON\"}]}"
		return 1
	fi

	local cond_count
	cond_count=$(echo "$precondition" | jq 'length')
	local details="[]"
	local final_result="READY"

	for ((i = 0; i < cond_count; i++)); do
		local cond cond_type met="false"
		cond=$(echo "$precondition" | jq -c ".[$i]")
		cond_type=$(echo "$cond" | jq -r '.type')

		case "$cond_type" in
		wave_status)
			local target_wave_id target_status actual_status
			target_wave_id=$(echo "$cond" | jq -r '.wave_id')
			target_status=$(echo "$cond" | jq -r '.status')
			# Look up wave status from plan JSON
			actual_status=$(echo "$pj" | jq -r --arg wid "$target_wave_id" '.waves[]? | select(.wave_id == $wid) | .status // ""' 2>/dev/null)
			if [[ "$actual_status" == "$target_status" ]]; then
				met="true"
			else
				if [[ "$final_result" != "SKIP" ]]; then final_result="BLOCKED"; fi
			fi
			;;
		output_match)
			local task_id output_path equals_val actual_data extracted
			task_id=$(echo "$cond" | jq -r '.task_id')
			output_path=$(echo "$cond" | jq -r '.output_path')
			equals_val=$(echo "$cond" | jq -r '.equals')
			# Look up task output from plan JSON
			actual_data=$(echo "$pj" | jq -r --arg tid "$task_id" '.tasks[]? | select(.task_id == $tid) | .output_data // ""' 2>/dev/null)
			if [[ -n "$actual_data" ]]; then
				extracted=$(echo "$actual_data" | jq -r "$output_path" 2>/dev/null || echo "")
				if [[ "$extracted" == "$equals_val" ]]; then
					met="true"
				else
					if [[ "$final_result" != "SKIP" ]]; then final_result="BLOCKED"; fi
				fi
			else
				if [[ "$final_result" != "SKIP" ]]; then final_result="BLOCKED"; fi
			fi
			;;
		skip_if)
			local task_id output_path equals_val actual_data extracted
			task_id=$(echo "$cond" | jq -r '.task_id')
			output_path=$(echo "$cond" | jq -r '.output_path')
			equals_val=$(echo "$cond" | jq -r '.equals')
			actual_data=$(echo "$pj" | jq -r --arg tid "$task_id" '.tasks[]? | select(.task_id == $tid) | .output_data // ""' 2>/dev/null)
			if [[ -n "$actual_data" ]]; then
				extracted=$(echo "$actual_data" | jq -r "$output_path" 2>/dev/null || echo "")
				if [[ "$extracted" == "$equals_val" ]]; then
					met="true"
					final_result="SKIP"
				fi
			fi
			;;
		*)
			if [[ "$final_result" != "SKIP" ]]; then final_result="BLOCKED"; fi
			;;
		esac

		details=$(echo "$details" | jq \
			--argjson cond "$cond" \
			--argjson met "$met" \
			'. + [{"condition": $cond, "met": $met}]')
	done

	echo "$details" | jq -c \
		--arg result "$final_result" \
		--arg wave_id "$wave_id" \
		'{"result": $result, "wave_id": $wave_id, "details": .}'
	return 0
}
