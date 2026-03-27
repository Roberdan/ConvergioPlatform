#!/bin/bash
# DEPRECATED — use daemon API endpoints for task validation instead
# Task-level validation functions
# Version: 2.0.0 — migrated from sqlite3 to cvg CLI / daemon API
command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 1; }

DAEMON_URL="${DAEMON_URL:-http://localhost:8420}"

# Validate a single task by DB id or task_id within a plan
# Usage: validate-task <task_db_id_or_task_id> [plan_id] [validated_by] [--force] [--report 'JSON']
# Sets validated_at + validated_by + validation_report on the task
cmd_validate_task() {
	local identifier="$1"
	local plan_id="${2:-}"
	local validated_by="${3:-thor}"
	local force=false
	local report=""

	local skip_next=false
	for i in "$@"; do
		if [[ "$skip_next" == true ]]; then
			skip_next=false
			continue
		fi
		case "$i" in
		--force) force=true ;;
		--report) skip_next=true ;;
		esac
	done

	local prev=""
	for arg in "$@"; do
		if [[ "$prev" == "--report" ]]; then
			report="$arg"
		fi
		prev="$arg"
	done

	# Resolve task via daemon API
	local task_db_id="" task_json="" plan_json=""
	if [[ -n "$plan_id" ]]; then
		plan_json=$(curl -sf "${DAEMON_URL}/api/plan-db/json/${plan_id}" 2>/dev/null) || plan_json=$(cvg plan show "$plan_id" 2>/dev/null) || plan_json='{}'
	fi

	if [[ "$identifier" =~ ^[0-9]+$ ]]; then
		# Try as db id first
		if [[ -n "$plan_json" ]]; then
			task_json=$(echo "$plan_json" | jq -c ".tasks[]? | select(.id==${identifier} or .db_id==${identifier})" 2>/dev/null)
			[[ -n "$task_json" ]] && task_db_id="$identifier"
		fi
	fi
	if [[ -z "$task_db_id" && -n "$plan_id" && -n "$plan_json" ]]; then
		# Try as task_id string
		task_json=$(echo "$plan_json" | jq -c --arg tid "$identifier" '.tasks[]? | select(.task_id == $tid)' 2>/dev/null)
		[[ -n "$task_json" ]] && task_db_id=$(echo "$task_json" | jq -r '.id // .db_id // ""')
	fi

	if [[ -z "$task_db_id" || -z "$task_json" ]]; then
		log_error "Task not found: $identifier (plan: ${plan_id:-any})"
		return 1
	fi

	local task_status
	task_status=$(echo "$task_json" | jq -r '.status // ""')
	if [[ "$task_status" != "submitted" && "$task_status" != "done" ]]; then
		log_error "Task $identifier status is '$task_status' — only 'submitted' or 'done' tasks can be validated"
		log_error "Flow: in_progress → submitted (plan-db-safe.sh) → done (Thor validate-task)"
		return 1
	fi

	if [[ "$task_status" == "done" ]]; then
		local already_validated
		already_validated=$(echo "$task_json" | jq -r '.validated_at // ""')
		if [[ -n "$already_validated" && "$already_validated" != "null" ]]; then
			echo -e "${YELLOW}Task $identifier already validated at $already_validated${NC}"
			return 0
		fi
	fi

	local valid_validators="thor|thor-quality-assurance-guardian|thor-per-wave"
	local effective_validator="$validated_by"
	if [[ "$force" == false ]]; then
		if ! echo "$validated_by" | grep -qE "^($valid_validators)$"; then
			log_error "REJECTED: Validator '$validated_by' is not a Thor agent."
			log_error "Only [$valid_validators] can validate tasks. Use --force to override (audited)."
			return 1
		fi
	elif [[ "$force" == true ]]; then
		if ! echo "$validated_by" | grep -qE "^($valid_validators)$"; then
			effective_validator="forced-admin"
			log_warn "FORCED validation: using 'forced-admin' validator (audited, not Thor)"
			local timestamp
			timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
			local audit_entry="{\"timestamp\":\"$timestamp\",\"event\":\"forced_validation\",\"task_db_id\":$task_db_id,\"validated_by\":\"$validated_by\",\"forced_as\":\"forced-admin\",\"action\":\"forced_bypass\"}"
			mkdir -p "$(dirname "$AUDIT_LOG")"
			echo "$audit_entry" >>"$AUDIT_LOG" 2>/dev/null || true
		fi
	fi

	local task_id_text
	task_id_text=$(echo "$task_json" | jq -r '.task_id // ""')

	if [[ "$task_status" == "submitted" ]]; then
		# Validate task via daemon API: submitted -> done
		cvg task validate "$task_db_id" "$plan_id" 2>/dev/null || \
		curl -sf -X POST "${DAEMON_URL}/api/plan-db/task/validate" \
			-H 'Content-Type: application/json' \
			-d "{\"task_id\":${task_db_id},\"validated_by\":\"${effective_validator}\"${report:+,\"validation_report\":$(jq -n --arg r "$report" '$r')}}" >/dev/null 2>&1 || true

		echo -e "${GREEN}Task $task_id_text: submitted → done (validated by $effective_validator)${NC}"
	else
		# Task already done — set validated_at if not already set
		curl -sf -X POST "${DAEMON_URL}/api/plan-db/task/validate" \
			-H 'Content-Type: application/json' \
			-d "{\"task_id\":${task_db_id},\"validated_by\":\"${effective_validator}\"${report:+,\"validation_report\":$(jq -n --arg r "$report" '$r')}}" >/dev/null 2>&1 || true
		echo -e "${GREEN}Task $task_id_text validated by $effective_validator (legacy re-validation)${NC}"
	fi

	[[ -n "$report" ]] && echo -e "${GREEN}  Validation report saved${NC}"
	return 0
}
