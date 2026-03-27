#!/bin/bash
set -euo pipefail
# Planner Init - Single-call project context bootstrap
# Version: 2.0.0 — migrated from sqlite3 to cvg CLI / daemon API
# Returns JSON with everything the planner needs in ONE call
# Usage: planner-init.sh [project_path]
command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 1; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DAEMON_URL="${DAEMON_URL:-http://localhost:8420}"

PROJECT_PATH="${1:-$(pwd)}"
PROJECT_PATH="$(cd "$PROJECT_PATH" && pwd)"

# Derive project ID from folder name (same logic as register-project.sh)
FOLDER_NAME=$(basename "$PROJECT_PATH")
PROJECT_ID=$(echo "$FOLDER_NAME" | tr '[:upper:]' '[:lower:]' | tr ' ' '-' | tr -cd '[:alnum:]-')

# Auto-register if not in DB — check via daemon API
PROJECT_EXISTS=$(curl -sf "${DAEMON_URL}/api/projects/${PROJECT_ID}" 2>/dev/null | jq -r '.id // ""' 2>/dev/null || echo "")
if [[ -z "$PROJECT_EXISTS" ]]; then
	"$SCRIPT_DIR/register-project.sh" "$PROJECT_PATH" >/dev/null 2>&1 || true
fi

# Git info
GIT_BRANCH=$(cd "$PROJECT_PATH" && git branch --show-current 2>/dev/null || echo "none")
GIT_REMOTE=$(cd "$PROJECT_PATH" && git remote get-url origin 2>/dev/null || echo "")

# Get project plans via daemon API
PROJECT_NAME=""
ACTIVE_PLANS="[]"
RECENT_PLANS="[]"

# Try cvg first, fall back to daemon API
PLAN_LIST_JSON=$(cvg project plans "$PROJECT_ID" 2>/dev/null) || \
PLAN_LIST_JSON=$(curl -sf "${DAEMON_URL}/api/plan-db/list" 2>/dev/null) || \
PLAN_LIST_JSON='[]'

# Extract project name
PROJECT_NAME=$(curl -sf "${DAEMON_URL}/api/projects/${PROJECT_ID}" 2>/dev/null | jq -r '.name // ""' 2>/dev/null || echo "")
[[ -z "$PROJECT_NAME" ]] && PROJECT_NAME="$FOLDER_NAME"

# Filter active plans (todo/doing) for this project
ACTIVE_PLANS=$(echo "$PLAN_LIST_JSON" | jq -c --arg pid "$PROJECT_ID" '
    [.[] | select(.project_id == $pid and (.status == "todo" or .status == "doing"))
    | {id, name, status, progress: ((.tasks_done // 0) | tostring) + "/" + ((.tasks_total // 0) | tostring), worktree_path: (.worktree_path // "")}]
    | sort_by(if .status == "doing" then 0 else 1 end, .id) | .[0:5]
' 2>/dev/null || echo '[]')

# Filter recent completed plans
RECENT_PLANS=$(echo "$PLAN_LIST_JSON" | jq -c --arg pid "$PROJECT_ID" '
    [.[] | select(.project_id == $pid and .status == "done")
    | {id, name, completed_at: (.completed_at // "")}]
    | sort_by(.completed_at) | reverse | .[0:3]
' 2>/dev/null || echo '[]')

# Worktrees
WORKTREES="[]"
if cd "$PROJECT_PATH" && git rev-parse --git-dir >/dev/null 2>&1; then
	WORKTREES=$(git worktree list --porcelain 2>/dev/null |
		grep "^worktree " | sed 's/^worktree //' |
		jq -R -s 'split("\n") | map(select(length > 0))' 2>/dev/null || echo "[]")
fi

# Project structure checks
HAS_ADR=$([[ -d "$PROJECT_PATH/docs/adr" ]] && echo "true" || echo "false")
HAS_CHANGELOG=$([[ -f "$PROJECT_PATH/CHANGELOG.md" ]] && echo "true" || echo "false")

# Prompt files (avoid pipefail double-output: ls fails + jq already emitted [])
PROMPT_FILES=$(ls "$PROJECT_PATH/.copilot-tracking/prompt-"*.md 2>/dev/null) || PROMPT_FILES=""
if [[ -n "$PROMPT_FILES" ]]; then
	PROMPT_FILES=$(echo "$PROMPT_FILES" | jq -R -s 'split("\n") | map(select(length > 0))')
else
	PROMPT_FILES="[]"
fi

# Detect test framework
FRAMEWORK="unknown"
if [[ -f "$PROJECT_PATH/package.json" ]]; then
	if grep -q '"vitest"' "$PROJECT_PATH/package.json" 2>/dev/null; then
		FRAMEWORK="vitest"
	elif grep -q '"jest"' "$PROJECT_PATH/package.json" 2>/dev/null; then
		FRAMEWORK="jest"
	elif grep -q '"playwright"' "$PROJECT_PATH/package.json" 2>/dev/null; then
		FRAMEWORK="playwright"
	else
		FRAMEWORK="node"
	fi
elif [[ -f "$PROJECT_PATH/pyproject.toml" ]]; then
	FRAMEWORK="pytest"
elif [[ -f "$PROJECT_PATH/Cargo.toml" ]]; then
	FRAMEWORK="cargo"
fi

# Plan folder (ensure exists)
PLAN_FOLDER="${HOME}/.claude/plans/${PROJECT_ID}"
mkdir -p "$PLAN_FOLDER"

# Output JSON
jq -n \
	--arg pid "$PROJECT_ID" \
	--arg pname "$PROJECT_NAME" \
	--arg path "$PROJECT_PATH" \
	--arg branch "$GIT_BRANCH" \
	--arg remote "$GIT_REMOTE" \
	--arg plan_folder "$PLAN_FOLDER" \
	--argjson active "$ACTIVE_PLANS" \
	--argjson recent "$RECENT_PLANS" \
	--argjson worktrees "$WORKTREES" \
	--argjson has_adr "$HAS_ADR" \
	--argjson has_changelog "$HAS_CHANGELOG" \
	--argjson prompts "$PROMPT_FILES" \
	--arg fw "$FRAMEWORK" \
	'{
        project_id: $pid,
        project_name: $pname,
        path: $path,
        branch: $branch,
        remote: $remote,
        plan_folder: $plan_folder,
        active_plans: $active,
        recent_plans: $recent,
        worktrees: $worktrees,
        has_adr: $has_adr,
        has_changelog: $has_changelog,
        prompt_files: $prompts,
        framework: $fw
    }'
