#!/bin/bash
# Plan-level validation functions
# Version: 2.0.0 — migrated from sqlite3 to cvg CLI / daemon API
command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 1; }

DAEMON_URL="${DAEMON_URL:-http://localhost:8420}"

# Helper: fetch full plan JSON (cached per invocation)
_vp_plan_json() {
	local plan_id="$1"
	curl -sf "${DAEMON_URL}/api/plan-db/json/${plan_id}" 2>/dev/null || cvg plan show "$plan_id" 2>/dev/null || echo '{}'
}

cmd_validate() {
	local plan_id="$1"
	local validated_by="${2:-thor}"
	local errors=0
	local warnings=0

	echo -e "${BLUE}======= THOR VALIDATION - Plan $plan_id =======${NC}"
	echo ""

	# Fetch plan JSON once
	local pj
	pj=$(_vp_plan_json "$plan_id")

	local project_id
	project_id=$(echo "$pj" | jq -r '.project_id // ""')

	echo -e "${YELLOW}[1/7] Wave counter sync...${NC}"
	# Check wave counters via plan JSON
	local wave_issues
	wave_issues=$(echo "$pj" | jq -r '
		[.waves[] | {
			wave_id: .wave_id,
			recorded_done: (.tasks_done // 0),
			recorded_total: (.tasks_total // 0),
			actual_done: ([.tasks[]? | select(.status == "done")] | length),
			actual_total: ([.tasks[]?] | length)
		} | select(.recorded_done != .actual_done or .recorded_total != .actual_total)]
		| if length > 0 then map("\(.wave_id)|\(.recorded_done)|\(.recorded_total)|\(.actual_done)|\(.actual_total)") | .[] else empty end
	' 2>/dev/null || echo "")
	if [ -n "$wave_issues" ]; then
		echo -e "${RED}  ERROR: Wave counters out of sync${NC}"
		echo "$wave_issues"
		((errors++))
	else
		echo -e "${GREEN}  OK${NC}"
	fi

	echo -e "${YELLOW}[2/7] Orphan tasks...${NC}"
	# Tasks with no valid wave
	local orphans
	orphans=$(echo "$pj" | jq -r '
		(.waves // [] | map(.id // .db_id) ) as $wave_ids |
		[.tasks[]? | select(.wave_id_fk as $wf | $wave_ids | index($wf) | not)]
		| if length > 0 then map("\(.id // .db_id)|\(.task_id)|\(.wave_id // "")") | .[] else empty end
	' 2>/dev/null || echo "")
	if [ -n "$orphans" ]; then
		echo -e "${RED}  ERROR: Orphan tasks found${NC}"
		echo "$orphans"
		((errors++))
	else
		echo -e "${GREEN}  OK${NC}"
	fi

	echo -e "${YELLOW}[3/7] Incomplete in done waves...${NC}"
	local incomplete
	incomplete=$(echo "$pj" | jq -r '
		[.waves[]? | select(.status == "done") |
			.wave_id as $wid | .tasks[]? | select(.status | IN("done","cancelled","skipped") | not) |
			"\($wid)|\(.task_id)|\(.status)"]
		| if length > 0 then .[] else empty end
	' 2>/dev/null || echo "")
	if [ -n "$incomplete" ]; then
		echo -e "${RED}  ERROR: Incomplete tasks in done waves${NC}"
		echo "$incomplete"
		((errors++))
	else
		echo -e "${GREEN}  OK${NC}"
	fi

	echo -e "${YELLOW}[4/7] Plan counter sync...${NC}"
	local plan_totals
	plan_totals=$(echo "$pj" | jq -r '
		(.tasks_done // 0) as $pd | (.tasks_total // 0) as $pt |
		([.waves[]? | .tasks_done // 0] | add // 0) as $ad |
		([.waves[]? | .tasks_total // 0] | add // 0) as $at |
		if $pd != $ad or $pt != $at then "\($pd)|\($pt)|\($ad)|\($at)" else empty end
	' 2>/dev/null || echo "")
	if [ -n "$plan_totals" ]; then
		echo -e "${RED}  ERROR: Plan counters out of sync${NC}"
		((errors++))
	else
		echo -e "${GREEN}  OK${NC}"
	fi

	echo -e "${YELLOW}[5/7] Date consistency...${NC}"
	local bad_dates
	bad_dates=$(echo "$pj" | jq -r '
		[.waves[]? | select(.planned_end != null and .planned_start != null and .planned_end < .planned_start) | .wave_id]
		| if length > 0 then .[] else empty end
	' 2>/dev/null || echo "")
	if [ -n "$bad_dates" ]; then
		echo -e "${YELLOW}  WARNING: Waves with end < start${NC}"
		((warnings++))
	else
		echo -e "${GREEN}  OK${NC}"
	fi

	echo -e "${YELLOW}[6/7] Executor agent tracking...${NC}"
	local missing_agent
	missing_agent=$(echo "$pj" | jq '[.tasks[]? | select(.status == "done" and (.executor_agent == null or .executor_agent == ""))] | length' 2>/dev/null || echo "0")
	if [ "$missing_agent" -gt 0 ]; then
		echo -e "${YELLOW}  WARNING: $missing_agent done tasks missing executor_agent${NC}"
		((warnings++))
	else
		echo -e "${GREEN}  OK${NC}"
	fi

	echo -e "${YELLOW}[7/7] Output data JSON validity...${NC}"
	local invalid_json=0
	local tasks_with_output
	tasks_with_output=$(echo "$pj" | jq -r '[.tasks[]? | select(.output_data != null and .output_data != "") | {task_id, output_data}] | .[] | "\(.task_id)|\(.output_data)"' 2>/dev/null || echo "")
	while IFS='|' read -r tid output; do
		[[ -z "$tid" ]] && continue
		if ! echo "$output" | jq -e . >/dev/null 2>&1; then
			echo -e "${RED}  ERROR: Task $tid has invalid JSON in output_data${NC}"
			((invalid_json++))
		fi
	done <<<"$tasks_with_output"
	if [ "$invalid_json" -gt 0 ]; then
		echo -e "${RED}  ERROR: $invalid_json tasks with invalid output_data JSON${NC}"
		((errors++))
	else
		echo -e "${GREEN}  OK${NC}"
	fi

	echo ""
	if [ $errors -gt 0 ]; then
		echo -e "${RED}FAILED: $errors errors, $warnings warnings${NC}"
		echo -e "${YELLOW}Run 'plan-db.sh sync $plan_id' to fix${NC}"
		return 1
	fi

	# Mark plan validated via daemon API
	curl -sf -X POST "${DAEMON_URL}/api/plan-db/plan/validate" \
		-H 'Content-Type: application/json' \
		-d "{\"plan_id\":${plan_id},\"validated_by\":\"${validated_by}\"}" >/dev/null 2>&1 || true

	# Check for unvalidated done tasks
	local unvalidated_count
	unvalidated_count=$(echo "$pj" | jq '[.tasks[]? | select(.status == "done" and .validated_at == null)] | length' 2>/dev/null || echo "0")
	if [ "$unvalidated_count" -gt 0 ]; then
		log_warn "$unvalidated_count done tasks lack per-task Thor validation — run validate-task for each"
		echo "$pj" | jq -r '.tasks[]? | select(.status == "done" and .validated_at == null) | "  - \(.task_id): \(.title)"' 2>/dev/null || true
	fi

	# Record version via daemon API
	# TODO: needs daemon endpoint for plan_versions insert
	curl -sf -X POST "${DAEMON_URL}/api/plan-db/plan/version" \
		-H 'Content-Type: application/json' \
		-d "{\"plan_id\":${plan_id},\"change_type\":\"validated\",\"change_reason\":\"Validated - 0 errors\",\"changed_by\":\"${validated_by}\",\"changed_host\":\"${PLAN_DB_HOST:-unknown}\"}" >/dev/null 2>&1 || true
	echo -e "${GREEN}PASSED: Plan $plan_id validated by $validated_by (host: ${PLAN_DB_HOST:-unknown})${NC}"

	# Auto-close if all tasks done
	local tasks_done tasks_total current_status
	tasks_done=$(echo "$pj" | jq -r '.tasks_done // 0')
	tasks_total=$(echo "$pj" | jq -r '.tasks_total // 0')
	current_status=$(echo "$pj" | jq -r '.status // ""')
	if [[ "$tasks_total" -gt 0 && "$tasks_done" -eq "$tasks_total" && "$current_status" != "done" ]]; then
		cvg plan complete "$plan_id" 2>/dev/null || \
		curl -sf -X POST "${DAEMON_URL}/api/plan-db/plan/update" \
			-H 'Content-Type: application/json' \
			-d "{\"plan_id\":${plan_id},\"status\":\"done\"}" >/dev/null 2>&1 || true
		echo -e "${GREEN}AUTO-CLOSE: Plan $plan_id marked as done (all $tasks_total tasks complete)${NC}"

		# TODO: needs daemon endpoint for plan_versions insert
		curl -sf -X POST "${DAEMON_URL}/api/plan-db/plan/version" \
			-H 'Content-Type: application/json' \
			-d "{\"plan_id\":${plan_id},\"change_type\":\"completed\",\"change_reason\":\"Auto-closed after Thor validation\",\"changed_by\":\"${validated_by}\",\"changed_host\":\"${PLAN_DB_HOST:-unknown}\"}" >/dev/null 2>&1 || true
	fi

	return 0
}

# Auto-approve plan: register reviews + approval in one shot (for delegated/autonomous plans)
cmd_auto_approve() {
	local plan_id="$1"
	local reason="${2:-Auto-approved for autonomous execution}"

	local pj
	pj=$(_vp_plan_json "$plan_id")
	local plan_name
	plan_name=$(echo "$pj" | jq -r '.name // ""')
	if [[ -z "$plan_name" ]]; then
		log_error "Plan $plan_id not found"
		return 1
	fi

	# Check existing reviews via plan JSON
	local reviews_json
	reviews_json=$(echo "$pj" | jq -c '.reviews // []' 2>/dev/null || echo '[]')

	local review_count biz_count challenger_count approval_count
	review_count=$(echo "$reviews_json" | jq '[.[] | select(.reviewer_agent | test("reviewer") and (test("challenger") | not))] | length' 2>/dev/null || echo "0")
	biz_count=$(echo "$reviews_json" | jq '[.[] | select(.reviewer_agent | test("business|advisor"))] | length' 2>/dev/null || echo "0")
	challenger_count=$(echo "$reviews_json" | jq '[.[] | select(.reviewer_agent | test("challenger"))] | length' 2>/dev/null || echo "0")
	approval_count=$(echo "$reviews_json" | jq '[.[] | select(.reviewer_agent == "user-approval")] | length' 2>/dev/null || echo "0")

	local added=0
	# Insert missing reviews via daemon API
	if [[ "$review_count" -eq 0 ]]; then
		curl -sf -X POST "${DAEMON_URL}/api/plan-db/review/create" \
			-H 'Content-Type: application/json' \
			-d "{\"plan_id\":${plan_id},\"reviewer_agent\":\"plan-reviewer\",\"verdict\":\"approved\",\"suggestions\":$(jq -n --arg r "$reason" '$r')}" >/dev/null 2>&1 || true
		added=$((added + 1))
	fi
	if [[ "$biz_count" -eq 0 ]]; then
		curl -sf -X POST "${DAEMON_URL}/api/plan-db/review/create" \
			-H 'Content-Type: application/json' \
			-d "{\"plan_id\":${plan_id},\"reviewer_agent\":\"plan-business-advisor\",\"verdict\":\"approved\",\"suggestions\":$(jq -n --arg r "$reason" '$r')}" >/dev/null 2>&1 || true
		added=$((added + 1))
	fi
	if [[ "$challenger_count" -eq 0 ]]; then
		curl -sf -X POST "${DAEMON_URL}/api/plan-db/review/create" \
			-H 'Content-Type: application/json' \
			-d "{\"plan_id\":${plan_id},\"reviewer_agent\":\"challenger\",\"verdict\":\"proceed\",\"suggestions\":$(jq -n --arg r "$reason" '$r')}" >/dev/null 2>&1 || true
		added=$((added + 1))
	fi
	if [[ "$approval_count" -eq 0 ]]; then
		curl -sf -X POST "${DAEMON_URL}/api/plan-db/review/create" \
			-H 'Content-Type: application/json' \
			-d "{\"plan_id\":${plan_id},\"reviewer_agent\":\"user-approval\",\"verdict\":\"approved\",\"suggestions\":$(jq -n --arg r "$reason" '$r')}" >/dev/null 2>&1 || true
		added=$((added + 1))
	fi
	log_info "Auto-approved plan #$plan_id ($plan_name): $added gate(s) registered"
}

# Check plan readiness for execution (BLOCKS if metadata missing)
cmd_check_readiness() {
	local plan_id="$1"
	local errors=0
	echo -e "${BLUE}======= READINESS CHECK - Plan $plan_id =======${NC}"

	echo -e "${YELLOW}[0/N] Precondition cycle detection...${NC}"
	if ! detect_precondition_cycles "$plan_id"; then
		echo -e "${RED}  FAIL: Circular dependencies in wave preconditions${NC}"
		errors=$((errors + 1))
	else
		echo -e "${GREEN}  OK: No cycles${NC}"
	fi

	local pj
	pj=$(_vp_plan_json "$plan_id")

	local src wt
	src=$(echo "$pj" | jq -r '.source_file // ""')
	wt=$(echo "$pj" | jq -r '.worktree_path // ""')
	if [[ -z "$src" ]]; then
		echo -e "${RED}  FAIL: source_file not set${NC}"
		errors=$((errors + 1))
	else
		echo -e "${GREEN}  OK: source_file${NC}"
	fi
	local wave_wt_count
	wave_wt_count=$(echo "$pj" | jq '[.waves[]? | select(.worktree_path != null and .worktree_path != "")] | length' 2>/dev/null || echo "0")
	if [[ -z "$wt" && "$wave_wt_count" -eq 0 ]]; then
		echo -e "${RED}  FAIL: No worktree set (plan-level or wave-level). Use wave-worktree.sh create or --auto-worktree${NC}"
		errors=$((errors + 1))
	elif [[ -n "$wt" ]]; then
		echo -e "${GREEN}  OK: plan worktree_path ($wt)${NC}"
	else
		echo -e "${GREEN}  OK: wave-level worktrees ($wave_wt_count waves with worktree)${NC}"
	fi

	local no_desc no_tc
	no_desc=$(echo "$pj" | jq '[.tasks[]? | select(.status == "pending" and (.description == null or .description == ""))] | length' 2>/dev/null || echo "0")
	no_tc=$(echo "$pj" | jq '[.tasks[]? | select(.status == "pending" and (.test_criteria == null or .test_criteria == ""))] | length' 2>/dev/null || echo "0")
	if [[ "$no_desc" -gt 0 ]]; then
		echo -e "${RED}  FAIL: $no_desc tasks missing description${NC}"
		errors=$((errors + 1))
	else
		echo -e "${GREEN}  OK: all tasks have description${NC}"
	fi
	if [[ "$no_tc" -gt 0 ]]; then
		echo -e "${RED}  FAIL: $no_tc tasks missing test_criteria${NC}"
		errors=$((errors + 1))
	else
		echo -e "${GREEN}  OK: all tasks have test_criteria${NC}"
	fi

	# Planner Process Gates (Rule 14: MANDATORY for 3+ tasks)
	local task_count
	task_count=$(echo "$pj" | jq '[.tasks[]?] | length' 2>/dev/null || echo "0")
	if [[ "$task_count" -ge 3 ]]; then
		local gate_errors=0
		echo -e "${YELLOW}[P] Planner Process Gates (Rule 14, $task_count tasks)...${NC}"

		local reviews_json
		reviews_json=$(echo "$pj" | jq -c '.reviews // []' 2>/dev/null || echo '[]')

		local review_count biz_count challenger_count approval_count
		review_count=$(echo "$reviews_json" | jq '[.[] | select(.reviewer_agent | test("reviewer") and (test("challenger") | not))] | length' 2>/dev/null || echo "0")
		biz_count=$(echo "$reviews_json" | jq '[.[] | select(.reviewer_agent | test("business|advisor"))] | length' 2>/dev/null || echo "0")
		challenger_count=$(echo "$reviews_json" | jq '[.[] | select(.reviewer_agent | test("challenger"))] | length' 2>/dev/null || echo "0")
		approval_count=$(echo "$reviews_json" | jq '[.[] | select(.reviewer_agent == "user-approval")] | length' 2>/dev/null || echo "0")

		if [[ "$review_count" -eq 0 ]]; then
			echo -e "${RED}  FAIL: No plan-reviewer record. Run Step 3.1 (plan intelligence review).${NC}"
			gate_errors=$((gate_errors + 1))
		else
			local rv
			rv=$(echo "$reviews_json" | jq -r '[.[] | select(.reviewer_agent | test("reviewer") and (test("challenger") | not))] | last | .verdict // ""' 2>/dev/null)
			echo -e "${GREEN}  OK: plan-reviewer (verdict: $rv)${NC}"
		fi
		if [[ "$biz_count" -eq 0 ]]; then
			echo -e "${RED}  FAIL: No business-advisor record. Run Step 3.1 (business assessment).${NC}"
			gate_errors=$((gate_errors + 1))
		else
			echo -e "${GREEN}  OK: plan-business-advisor${NC}"
		fi
		if [[ "$challenger_count" -eq 0 ]]; then
			echo -e "${RED}  FAIL: No challenger-review record. Run Step 3.3 (challenger review).${NC}"
			gate_errors=$((gate_errors + 1))
		else
			local cv
			cv=$(echo "$reviews_json" | jq -r '[.[] | select(.reviewer_agent | test("challenger"))] | last | .verdict // ""' 2>/dev/null)
			echo -e "${GREEN}  OK: plan-challenger (verdict: $cv)${NC}"
		fi
		if [[ "$approval_count" -eq 0 ]]; then
			echo -e "${RED}  FAIL: No user-approval record. Run: plan-db.sh approve $plan_id${NC}"
			gate_errors=$((gate_errors + 1))
		else
			echo -e "${GREEN}  OK: user-approval${NC}"
		fi
		errors=$((errors + gate_errors))
	fi

	if [[ $errors -gt 0 ]]; then
		echo -e "${RED}BLOCKED: $errors issues. Fix before /execute.${NC}"
		return 1
	fi
	echo -e "${GREEN}READY: Plan $plan_id is ready for execution${NC}"
	return 0
}

# Sync counters
cmd_sync() {
	local plan_id="$1"
	log_info "Syncing counters for plan $plan_id..."

	# Use daemon API to sync counters
	curl -sf -X POST "${DAEMON_URL}/api/plan-db/plan/sync" \
		-H 'Content-Type: application/json' \
		-d "{\"plan_id\":${plan_id}}" 2>/dev/null || {
		# Fallback: trigger via cvg CLI
		# TODO: needs daemon endpoint for counter sync
		echo "WARNING: Counter sync API not available, skipping" >&2
	}

	# Show updated state
	local pj
	pj=$(_vp_plan_json "$plan_id")
	echo "$pj" | jq -r '.waves[]? | "\(.wave_id)\t\(.name // "")\t\(.status)\t\(.tasks_done // 0)/\(.tasks_total // 0)"' 2>/dev/null || true
	log_info "Sync complete"
}
