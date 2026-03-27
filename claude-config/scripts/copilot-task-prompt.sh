#!/bin/bash
set -euo pipefail
# Generate self-contained prompt for Copilot CLI worker
# Usage: copilot-task-prompt.sh <db_task_id> [agent_role]
# Output: prompt string to stdout (pipe to copilot -p)

# Version: 2.1.0
set -euo pipefail

TASK_ID="${1:-}"
if [[ -z "$TASK_ID" ]]; then
	echo "Usage: copilot-task-prompt.sh <db_task_id> [agent_role]" >&2
	exit 1
fi
AGENT_ROLE="${2:-executor}"

command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 1; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONTEXT_LOADER="${SCRIPT_DIR}/lib/agent-context-loader.sh"

# Fetch task + wave + plan info via daemon API
_plan_list="$(curl -sf http://localhost:8420/api/plan-db/list 2>/dev/null || echo '[]')"
# Find the plan containing this task by DB id
_plan_id="$(echo "$_plan_list" | jq -r --argjson tid "$TASK_ID" '[.[] | select(.tasks[]? | (.id // .db_task_id) == $tid)] | .[0].id // empty' 2>/dev/null || echo '')"
if [[ -z "$_plan_id" ]]; then
	echo "Task $TASK_ID not found in any plan" >&2
	exit 1
fi
_plan_json="$(cvg plan show "$_plan_id" 2>/dev/null || echo '{}')"
TASK_JSON="$(echo "$_plan_json" | jq -c --argjson tid "$TASK_ID" '
	.tasks[] | select((.id // .db_task_id) == $tid) |
	{task_id: .task_id, title: .title, description: (.description // ""),
	 test_criteria: (.test_criteria // ""), wave_id: .wave_id, wave_name: .wave_name,
	 plan_id: '"$_plan_id"', plan_name: "'"$(echo "$_plan_json" | jq -r '.name // ""')"'",
	 worktree_path: (.worktree_path // ""), db_task_id: (.id // .db_task_id)}
' 2>/dev/null || echo '')"

if [[ -z "$TASK_JSON" ]]; then
	echo "Task $TASK_ID not found" >&2
	exit 1
fi

# Parse fields
TITLE=$(echo "$TASK_JSON" | jq -r '.title')
DESC=$(echo "$TASK_JSON" | jq -r '.description')
TC=$(echo "$TASK_JSON" | jq -r '.test_criteria')
WAVE=$(echo "$TASK_JSON" | jq -r '.wave_id')
TID=$(echo "$TASK_JSON" | jq -r '.task_id')
WT_RAW=$(echo "$TASK_JSON" | jq -r '.worktree_path')
PLAN_ID=$(echo "$TASK_JSON" | jq -r '.plan_id')

# Expand worktree path
WT="${WT_RAW/#\~/$HOME}"

# Fetch completed tasks with output_data from same plan (inter-task context)
PRIOR_OUTPUTS="$(echo "$_plan_json" | jq -r '
	[.tasks[] | select(.status == "done" and .output_data != null and .output_data != "") |
	 "\(.task_id): \(.output_data)"] | join("\n")
' 2>/dev/null || echo '')"

# Check for PR feedback from previous wave (overlapping wave protocol)
PR_FEEDBACK=""
# Extract current task's wave position, then find previous async wave
_cur_wave_pos="$(echo "$_plan_json" | jq -r --arg wid "$WAVE" '.waves[]? | select(.wave_id == $wid) | .position // 0' 2>/dev/null || echo 0)"
PREV_WAVE_DB_ID="$(echo "$_plan_json" | jq -r --argjson pos "$_cur_wave_pos" '
	[.waves[]? | select(.position < $pos and .merge_mode == "async")] | sort_by(.position) | last | .id // empty
' 2>/dev/null || echo '')"
if [[ -n "$PREV_WAVE_DB_ID" ]]; then
	FEEDBACK_FILE="${HOME}/.claude/data/pr-feedback-wave-${PREV_WAVE_DB_ID}.txt"
	if [[ -f "$FEEDBACK_FILE" ]]; then
		PR_FEEDBACK=$(head -50 "$FEEDBACK_FILE" 2>/dev/null || true)
	fi
fi

# Detect test framework
FW="unknown"
if [[ -f "$WT/package.json" ]]; then
	if grep -q '"vitest"' "$WT/package.json" 2>/dev/null; then
		FW="vitest"
	elif grep -q '"jest"' "$WT/package.json" 2>/dev/null; then
		FW="jest"
	else FW="node"; fi
elif [[ -f "$WT/pyproject.toml" ]]; then
	FW="pytest"
elif [[ -f "$WT/Cargo.toml" ]]; then
	FW="cargo"
fi

PRECHECK_JSON="{}"
if [[ -x "${HOME}/.claude/scripts/execution-preflight.sh" ]]; then
	PRECHECK_JSON="$("${HOME}/.claude/scripts/execution-preflight.sh" --plan-id "$PLAN_ID" "$WT" 2>/dev/null || echo '{}')"
fi

ROLE_CONTEXT=""
if [[ -x "$CONTEXT_LOADER" ]]; then
	ROLE_CONTEXT="$("$CONTEXT_LOADER" "$AGENT_ROLE" 2>/dev/null || true)"
fi

# Generate prompt
cat <<PROMPT
# Task Execution: $TID ($TITLE)

## !! MANDATORY COMPLETION — READ THIS FIRST !!

**You MUST run this command when your work is done. This is NON-NEGOTIABLE.**
**If you skip this, the plan dashboard will show 0% progress.**

\`\`\`bash
plan-db-safe.sh update-task $TASK_ID done "Summary of what was done" --tokens 0 --output-data '{"summary":"what was done","artifacts":["file1.ts"]}'
\`\`\`

Use \`plan-db-safe.sh\` (NOT \`plan-db.sh\`). \`plan-db.sh\` will REJECT done status.

## Setup
\`\`\`bash
export PATH="\$HOME/.claude/scripts:\$PATH"
cd "$WT" && pwd
worktree-guard.sh "$WT"
worktree-safety.sh audit "$WT" 2>/dev/null || true
execution-preflight.sh --plan-id $PLAN_ID "$WT"
plan-db-safe.sh update-task $TASK_ID in_progress "Started by Copilot"
\`\`\`

## Execution Readiness Snapshot
$PRECHECK_JSON

## Rules
1. Work ONLY in: $WT — NEVER checkout main/master
2. If \`worktree-guard.sh\` fails, STOP immediately
3. TDD: tests FIRST, then implement
4. If the snapshot warns about dirty worktree, missing troubleshooting, missing CI knowledge, or missing GH auth for PR/CI work, STOP and resolve before coding
5. For auth/CI/deploy/permissions/version issues, read TROUBLESHOOTING.md, relevant ADRs, and CI knowledge before deciding the fix

## Agent Instruction Context (Role: $AGENT_ROLE)
$(if [[ -n "$ROLE_CONTEXT" ]]; then echo "$ROLE_CONTEXT"; else echo "_Role context unavailable; proceed with repository-local instructions only._"; fi)

## Task
**Wave**: $WAVE | **Task**: $TID | **Framework**: $FW | **Plan**: $PLAN_ID

**Do**: $TITLE

$DESC

## Prior Task Outputs
$(if [[ -n "$PRIOR_OUTPUTS" ]]; then echo "$PRIOR_OUTPUTS"; else echo "None."; fi)

## Previous Wave PR Feedback
$(if [[ -n "$PR_FEEDBACK" ]]; then
echo "⚠️ The previous wave's PR had review feedback. Do NOT repeat these issues:"
echo ""
echo "$PR_FEEDBACK"
else echo "None."; fi)

## Test Criteria
$TC

## TDD Workflow
1. Write FAILING tests based on test criteria above
2. Run tests, confirm they FAIL (RED)
3. Implement minimum code to make tests PASS (GREEN)
4. Refactor if needed

## Coding Standards
- Max 250 lines per file. Split if exceeds.
- No TODO, FIXME, @ts-ignore in new code
- English for all code and comments
- If you touch auth, permissions, CI, PR, deployment, or versioning, add/keep a smoke-testable verification path

## !! FINAL STEP — DO NOT SKIP !!

Run this BEFORE you finish. If you already ran it above, verify with:
\`\`\`bash
curl -sf "http://localhost:8421/api/plan-db/context/$PLAN_ID" \
  | jq -r --argjson tid "$TASK_ID" '[.tasks[] | select((.id // .db_task_id) == \$tid) | .status][0]'
# Must show: submitted (Thor will validate to done)
\`\`\`

If NOT submitted, run:
\`\`\`bash
plan-db-safe.sh update-task $TASK_ID done "Summary" --tokens 0
\`\`\`
PROMPT
