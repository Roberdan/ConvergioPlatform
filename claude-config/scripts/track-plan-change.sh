#!/bin/bash
set -euo pipefail
# Track Plan Change - Records plan modifications for learning/optimization
# Usage: ./track-plan-change.sh <project_id> <plan_name> <change_type> [--reason "text"] [--tasks-before N] [--tasks-after N]
# Change types: created, user_edit, scope_add, scope_remove, blocker, replan, task_split, completed

# Version: 1.1.0
set -euo pipefail

CLAUDE_HOME="${HOME}/.claude"
DAEMON_API="http://localhost:8420"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 1; }

# Required args
PROJECT_ID="${1:-}"
PLAN_NAME="${2:-}"
CHANGE_TYPE="${3:-}"

if [[ -z "$PROJECT_ID" || -z "$PLAN_NAME" || -z "$CHANGE_TYPE" ]]; then
	echo "Usage: $0 <project_id> <plan_name> <change_type> [options]" >&2
	echo "Types: created, user_edit, scope_add, scope_remove, blocker, replan, task_split, completed" >&2
	exit 1
fi

# Validate change type
VALID_TYPES="created user_edit scope_add scope_remove blocker replan task_split completed"
if [[ ! " $VALID_TYPES " =~ " $CHANGE_TYPE " ]]; then
	echo "Invalid change_type: $CHANGE_TYPE" >&2
	echo "Valid types: $VALID_TYPES" >&2
	exit 1
fi

shift 3

# Optional args
REASON=""
TASKS_BEFORE=""
TASKS_AFTER=""
DIFF_SUMMARY=""

while [[ $# -gt 0 ]]; do
	case $1 in
	--reason)
		REASON="$2"
		shift 2
		;;
	--tasks-before)
		TASKS_BEFORE="$2"
		shift 2
		;;
	--tasks-after)
		TASKS_AFTER="$2"
		shift 2
		;;
	--diff)
		DIFF_SUMMARY="$2"
		shift 2
		;;
	*) shift ;;
	esac
done

TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

# Get git commit hash if in repo
GIT_HASH=""
PLAN_FOLDER="${CLAUDE_HOME}/plans/${PROJECT_ID}"
if [[ -d "${CLAUDE_HOME}/.git" ]]; then
	cd "$CLAUDE_HOME"
	GIT_HASH=$(git rev-parse HEAD 2>/dev/null || echo "")
fi

# Track plan change via daemon API
_result="$(curl -sf -X POST "${DAEMON_API}/api/events" \
	-H 'Content-Type: application/json' \
	-d "$(jq -nc \
		--arg pid "$PROJECT_ID" --arg pname "$PLAN_NAME" --arg ct "$CHANGE_TYPE" \
		--arg reason "$REASON" --arg diff "$DIFF_SUMMARY" --arg git "$GIT_HASH" \
		--arg ts "$TIMESTAMP" --arg tb "${TASKS_BEFORE:-}" --arg ta "${TASKS_AFTER:-}" \
		'{type:"plan_version", project_id:$pid, plan_name:$pname, change_type:$ct,
		 change_reason:$reason, diff_summary:$diff, git_commit_hash:$git,
		 tasks_before:($tb | if . == "" then null else tonumber end),
		 tasks_after:($ta | if . == "" then null else tonumber end),
		 created_at:$ts}')" 2>/dev/null || echo '{}')"

NEW_VERSION="$(echo "$_result" | jq -r '.version // 1' 2>/dev/null || echo 1)"

# Also update plans table status if needed via cvg CLI
if [[ "$CHANGE_TYPE" == "created" ]]; then
	# Plan creation is handled by cvg plan create
	true
elif [[ "$CHANGE_TYPE" == "completed" ]]; then
	# Mark plan as completed via API
	curl -sf -X POST "${DAEMON_API}/api/plan-status" \
		-H 'Content-Type: application/json' \
		-d "$(jq -nc --arg pid "$PROJECT_ID" --arg pname "$PLAN_NAME" '{project_id:$pid, plan_name:$pname, status:"completed"}')" 2>/dev/null || true
fi

# Auto-commit to git if configured
if [[ -d "${CLAUDE_HOME}/.git" && -n "$GIT_HASH" ]]; then
	cd "$CLAUDE_HOME"
	if [[ -n $(git status --porcelain "plans/${PROJECT_ID}/" 2>/dev/null) ]]; then
		git add "plans/${PROJECT_ID}/" 2>/dev/null || true
		git commit -m "plan(${PROJECT_ID}): ${CHANGE_TYPE} - ${PLAN_NAME} v${NEW_VERSION}" \
			-m "${REASON:-No reason provided}" \
			-m "Generated with [Claude Code](https://claude.com/claude-code)" 2>/dev/null || true
	fi
fi

# Output result
jq -n \
	--arg project "$PROJECT_ID" \
	--arg plan "$PLAN_NAME" \
	--argjson version "$NEW_VERSION" \
	--arg type "$CHANGE_TYPE" \
	--arg reason "$REASON" \
	--arg git "$GIT_HASH" \
	'{
        "status": "success",
        "project_id": $project,
        "plan_name": $plan,
        "version": $version,
        "change_type": $type,
        "reason": (if $reason != "" then $reason else null end),
        "git_commit": (if $git != "" then $git else null end)
    }'
