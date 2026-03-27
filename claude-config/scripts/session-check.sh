#!/usr/bin/env bash
# Version: 1.0.0
# Fast session status script — outputs JSON in <5s
# No web calls except gh pr list (3s timeout)
set -euo pipefail

DAEMON_URL="${DAEMON_URL:-http://localhost:8420}"
TIMESTAMP=$(date -u '+%Y-%m-%dT%H:%M:%SZ')

command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 1; }

# Check daemon is reachable
_api_available() { curl -sf "${DAEMON_URL}/api/health" &>/dev/null; }

# --- Git status ---
BRANCH=$(git -C "$HOME/.claude" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
PORCELAIN=$(git -C "$HOME/.claude" status --porcelain 2>/dev/null || echo "")
UNCOMMITTED=$(echo "$PORCELAIN" | grep -c . || true)
UNPUSHED=$(git -C "$HOME/.claude" rev-list "@{u}..HEAD" --count 2>/dev/null || echo 0)
CLEAN=$([[ "$UNCOMMITTED" -eq 0 && "$UNPUSHED" -eq 0 ]] && echo true || echo false)

GIT_STATUS=$(jq -n \
	--arg branch "$BRANCH" \
	--argjson clean "$CLEAN" \
	--argjson uncommitted "$UNCOMMITTED" \
	--argjson unpushed "$UNPUSHED" \
	'{branch: $branch, clean: $clean, uncommitted: $uncommitted, unpushed: $unpushed}')

# --- Plans from daemon API ---
PLANS_JSON="[]"
STUCK_WAVE_MESSAGES="[]"
STALE_TASK_MESSAGES="[]"

if _api_available; then
	# Active/recent plans via daemon API
	ALL_PLANS=$(curl -sf "${DAEMON_URL}/api/plan-db/plans" 2>/dev/null || echo '[]')

	PLANS_JSON=$(echo "$ALL_PLANS" | jq '[
		.[] | select(.status == "doing" or .status == "todo")
		| {id: .id, name: .name, status: .status,
		   progress: "\(.tasks_done // 0)/\(.tasks_total // 0)",
		   waves_stuck: 0}
	] | sort_by(-.id) | .[0:10]')

	# Waves stuck in merging state — iterate active plans
	ACTIVE_PLAN_IDS=$(echo "$PLANS_JSON" | jq -r '.[].id')
	for pid in $ACTIVE_PLAN_IDS; do
		PLAN_DETAIL=$(curl -sf "${DAEMON_URL}/api/plan-db/json/${pid}" 2>/dev/null || echo '{}')
		STUCK=$(echo "$PLAN_DETAIL" | jq -r '
			[.waves[]? | select(.status == "merging")] |
			group_by(.wave_id) | .[] |
			"\(.[0].wave_id)\t\(length)"' 2>/dev/null || echo "")
		PLAN_NAME=$(echo "$PLAN_DETAIL" | jq -r '.name // "unknown"')
		if [[ -n "$STUCK" ]]; then
			while IFS=$'\t' read -r wave_id cnt; do
				[[ -z "$wave_id" ]] && continue
				PLANS_JSON=$(echo "$PLANS_JSON" | jq \
					--argjson pid "$pid" \
					--argjson cnt "$cnt" \
					'map(if .id == $pid then .waves_stuck += $cnt else . end)')
				STUCK_WAVE_MESSAGES=$(echo "$STUCK_WAVE_MESSAGES" | jq \
					--arg msg "Wave $wave_id stuck in merging state (plan $pid: $PLAN_NAME)" \
					'. + [$msg]')
			done <<<"$STUCK"
		fi

		# Stale in_progress tasks (older than 2h)
		TWO_HOURS_AGO=$(date -u -v-2H '+%Y-%m-%dT%H:%M:%SZ' 2>/dev/null || date -u -d '2 hours ago' '+%Y-%m-%dT%H:%M:%SZ' 2>/dev/null || echo "")
		if [[ -n "$TWO_HOURS_AGO" ]]; then
			STALE=$(echo "$PLAN_DETAIL" | jq -r --arg cutoff "$TWO_HOURS_AGO" '
				[.tasks[]? | select(.status == "in_progress" and .started_at != null and .started_at < $cutoff)]
				| .[:5][] | "\(.task_id)\t\(.title // "untitled")\t\(.plan_id // "")"' 2>/dev/null || echo "")
			if [[ -n "$STALE" ]]; then
				while IFS=$'\t' read -r task_id title stale_plan_id; do
					[[ -z "$task_id" ]] && continue
					STALE_TASK_MESSAGES=$(echo "$STALE_TASK_MESSAGES" | jq \
						--arg msg "Task $task_id ('$title') in_progress >2h (plan ${stale_plan_id:-$pid})" \
						'. + [$msg]')
				done <<<"$STALE"
			fi
		fi
	done
fi

# --- Open PRs (gh with 3s timeout) ---
OPEN_PRS="[]"
if command -v gh &>/dev/null; then
	PR_RAW=$(timeout 3 gh pr list --state open --limit 5 \
		--json number,title,state,statusCheckRollup 2>/dev/null || echo "[]")
	if [[ "$PR_RAW" != "[]" && -n "$PR_RAW" ]]; then
		OPEN_PRS=$(echo "$PR_RAW" | jq '[.[] | {
      number: .number,
      title: .title,
      state: .state,
      ci: (
        if (.statusCheckRollup | length) == 0 then "unknown"
        elif (.statusCheckRollup | all(.conclusion == "SUCCESS")) then "passing"
        elif (.statusCheckRollup | any(.conclusion == "FAILURE")) then "failing"
        else "pending"
        end
      )
    }]' 2>/dev/null || echo "[]")
	fi
fi

# --- Forgotten array ---
FORGOTTEN="[]"
if [[ "$UNCOMMITTED" -gt 0 ]]; then
	FORGOTTEN=$(echo "$FORGOTTEN" | jq --argjson n "$UNCOMMITTED" \
		'. + ["\($n) uncommitted file(s) in current directory"]')
fi
if [[ "$UNPUSHED" -gt 0 ]]; then
	FORGOTTEN=$(echo "$FORGOTTEN" | jq --argjson n "$UNPUSHED" \
		'. + ["\($n) unpushed commit(s)"]')
fi
FORGOTTEN=$(echo "$FORGOTTEN" | jq \
	--argjson stuck "$STUCK_WAVE_MESSAGES" \
	--argjson stale "$STALE_TASK_MESSAGES" \
	'. + $stuck + $stale')

# --- Next steps ---
NEXT_STEPS="[]"

# Per plan: remaining tasks (from already-fetched plans data)
if _api_available; then
	REMAINING=$(echo "$ALL_PLANS" | jq -r '
		[.[] | select((.status == "doing" or .status == "todo") and ((.tasks_total // 0) - (.tasks_done // 0)) > 0)]
		| sort_by(-.id) | .[0:5][]
		| "\(.id)\t\(.name)\t\((.tasks_total // 0) - (.tasks_done // 0))"' 2>/dev/null || echo "")
	if [[ -n "$REMAINING" ]]; then
		while IFS=$'\t' read -r pid pname rem; do
			[[ -z "$pid" ]] && continue
			NEXT_STEPS=$(echo "$NEXT_STEPS" | jq \
				--arg msg "Complete remaining $rem task(s) in plan $pid ($pname)" \
				'. + [$msg]')
		done <<<"$REMAINING"
	fi
fi

# Stuck waves
if [[ $(echo "$STUCK_WAVE_MESSAGES" | jq 'length') -gt 0 ]]; then
	NEXT_STEPS=$(echo "$NEXT_STEPS" | jq \
		'. + ["Fix stuck merging waves (see forgotten array)"]')
fi

# Unpushed commits
if [[ "$UNPUSHED" -gt 0 ]]; then
	NEXT_STEPS=$(echo "$NEXT_STEPS" | jq '. + ["Push unpushed commits"]')
fi

# PRs ready to merge
MERGEABLE_PRS=$(echo "$OPEN_PRS" | jq '[.[] | select(.ci == "passing")] | length')
if [[ "$MERGEABLE_PRS" -gt 0 ]]; then
	NEXT_STEPS=$(echo "$NEXT_STEPS" | jq \
		--argjson n "$MERGEABLE_PRS" \
		'. + ["Merge \($n) PR(s) with passing CI"]')
fi

# --- Final JSON ---
jq -n \
	--arg ts "$TIMESTAMP" \
	--argjson git_status "$GIT_STATUS" \
	--argjson plans "$PLANS_JSON" \
	--argjson open_prs "$OPEN_PRS" \
	--argjson forgotten "$FORGOTTEN" \
	--argjson next_steps "$NEXT_STEPS" \
	'{
    timestamp: $ts,
    git_status: $git_status,
    plans: $plans,
    open_prs: $open_prs,
    forgotten: $forgotten,
    next_steps: $next_steps
  }'
