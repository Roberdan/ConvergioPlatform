#!/bin/bash
# session-recovery.sh - Check all active plans for uncommitted work
# Run at session start to detect and recover lost changes
# Version: 1.0.0
set -euo pipefail

DAEMON_URL="${DAEMON_URL:-http://localhost:8420}"
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m'

command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 1; }

if ! curl -sf "${DAEMON_URL}/api/health" &>/dev/null; then
	echo "Daemon not reachable at ${DAEMON_URL}"
	exit 0
fi

issues=0

# Check all active plans with worktrees via daemon API
ACTIVE_PLANS=$(curl -sf "${DAEMON_URL}/api/plan-db/plans" 2>/dev/null || echo '[]')
DOING_PLANS=$(echo "$ACTIVE_PLANS" | jq -r '.[] | select(.status == "doing") | "\(.id)|\(.name)|\(.worktree_path // "")|\(.status)"')

while IFS='|' read -r plan_id name worktree status; do
	[[ -z "$worktree" || ! -d "$worktree" ]] && continue

	dirty=$(git -C "$worktree" status --porcelain 2>/dev/null || echo "")
	stashes=$(git -C "$worktree" stash list 2>/dev/null | grep -c "" || echo "0")
	branch=$(git -C "$worktree" branch --show-current 2>/dev/null || echo "unknown")

	if [[ -n "$dirty" ]]; then
		echo -e "${RED}[DIRTY]${NC} Plan #${plan_id} (${name}) on ${branch}"
		echo "  Worktree: $worktree"
		echo "  Uncommitted files:"
		git -C "$worktree" status --porcelain 2>/dev/null | head -10 | sed 's/^/    /'
		issues=$((issues + 1))
	fi

	if [[ "$stashes" -gt 0 ]]; then
		echo -e "${YELLOW}[STASH]${NC} Plan #${plan_id} (${name}): ${stashes} stash(es)"
		git -C "$worktree" stash list 2>/dev/null | head -3 | sed 's/^/    /'
		issues=$((issues + 1))
	fi

	# Check for done tasks without commits (task marked done after last commit)
	last_commit_ts=$(git -C "$worktree" log -1 --format='%ct' 2>/dev/null || echo "0")
	PLAN_DETAIL=$(curl -sf "${DAEMON_URL}/api/plan-db/json/${plan_id}" 2>/dev/null || echo '{}')
	orphan_tasks=$(echo "$PLAN_DETAIL" | jq -r --arg cutoff "$last_commit_ts" '
		[.tasks[]? | select(.status == "done" and .validated_at != null and .completed_at != null)
		| select((.completed_at | sub("\\.[0-9]+Z$"; "Z") | fromdateiso8601 // 0) > ($cutoff | tonumber))]
		| .[].task_id' 2>/dev/null || echo "")
	if [[ -n "$orphan_tasks" ]]; then
		echo -e "${YELLOW}[ORPHAN]${NC} Plan #${plan_id}: tasks done after last commit:"
		echo "$orphan_tasks" | sed 's/^/    /'
		issues=$((issues + 1))
	fi
done <<<"$DOING_PLANS"

# Check non-plan repos with known paths for dirty state
for repo_path in "$HOME/GitHub/MirrorBuddy"; do
	[[ ! -d "$repo_path/.git" ]] && continue
	dirty=$(git -C "$repo_path" status --porcelain 2>/dev/null | head -1)
	stashes=$(git -C "$repo_path" stash list 2>/dev/null | grep -c "" || echo "0")
	branch=$(git -C "$repo_path" branch --show-current 2>/dev/null || echo "unknown")

	if [[ -n "$dirty" || "$stashes" -gt 0 ]]; then
		name=$(basename "$repo_path")
		echo -e "${YELLOW}[REPO]${NC} ${name} on ${branch}"
		[[ -n "$dirty" ]] && echo "  Uncommitted:" && git -C "$repo_path" status --porcelain 2>/dev/null | head -5 | sed 's/^/    /'
		[[ "$stashes" -gt 0 ]] && echo "  Stashes: $stashes"
		issues=$((issues + 1))
	fi
done

if [[ "$issues" -eq 0 ]]; then
	echo -e "${GREEN}[OK]${NC} All active plans and repos clean"
else
	echo ""
	echo -e "${RED}Found $issues issue(s). Review and commit/stash before proceeding.${NC}"
fi

exit 0
