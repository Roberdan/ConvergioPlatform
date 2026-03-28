#!/bin/bash
# copilot-plan-runner.sh — Recursive execution machine
# Spawns fresh CLI sessions until plan is 100% complete.
# Uses daemon API execution-context for prompt generation.
# Usage: copilot-plan-runner.sh <plan_id>
set -euo pipefail
trap 'echo "ERROR at line $LINENO" >&2' ERR

PLAN_ID="${1:?Usage: copilot-plan-runner.sh <plan_id>}"
DAEMON_API="${CVG_URL:-http://localhost:8420}"
MAX_RETRIES=50
RETRY=0

plan_done() {
	local ctx
	ctx="$(curl -sf "${DAEMON_API}/api/plan-db/execution-context/${PLAN_ID}" 2>/dev/null || echo '{}')"
	local status
	status="$(echo "$ctx" | python3 -c "import json,sys; print(json.load(sys.stdin).get('status','unknown'))" 2>/dev/null || echo 'unknown')"
	[ "$status" = "completed" ] || [ "$status" = "cancelled" ]
}

get_context() {
	curl -sf "${DAEMON_API}/api/plan-db/execution-context/${PLAN_ID}" 2>/dev/null || echo '{}'
}

echo "=== Plan #${PLAN_ID} Runner (auto-restart) ==="

while ! plan_done; do
	RETRY=$((RETRY + 1))
	if [ "$RETRY" -gt "$MAX_RETRIES" ]; then
		echo "[FAIL] Max retries ($MAX_RETRIES) reached."
		exit 1
	fi

	CTX="$(get_context)"

	# Extract key info via python3 (handles control chars in JSON)
	eval "$(echo "$CTX" | python3 -c "
import json, sys
d = json.load(sys.stdin)
wt = d.get('worktree_path', '')
prompt = d.get('prompt', '')
wave = d.get('current_wave', {})
nt = d.get('next_task', {})
status = d.get('status', 'unknown')
print(f'WORKTREE=\"{wt}\"')
print(f'WAVE_ID=\"{wave.get(\"id\",\"?\")}\"')
print(f'NEEDS_THOR={\"true\" if wave.get(\"needs_thor\") else \"false\"}')
print(f'NEXT_TASK=\"{nt.get(\"task_id\",\"none\")}\"')
print(f'PLAN_STATUS=\"{status}\"')
# Write prompt to temp file (can contain special chars)
import tempfile, os
fd, path = tempfile.mkstemp(prefix='convergio-prompt-', suffix='.txt')
os.write(fd, prompt.encode())
os.close(fd)
print(f'PROMPT_FILE=\"{path}\"')
" 2>/dev/null)"

	echo ""
	echo "[Run ${RETRY}/${MAX_RETRIES}] Wave: ${WAVE_ID} | Next: ${NEXT_TASK} | Thor: ${NEEDS_THOR}"

	# Reset ONLY in_progress tasks (NOT submitted — those are awaiting Thor)
	curl -sf "${DAEMON_API}/api/plan-db/json/${PLAN_ID}" 2>/dev/null \
		| python3 -c "
import json, sys, urllib.request
d = json.load(sys.stdin)
for t in d.get('tasks', []):
    if t.get('status') == 'in_progress':
        print(f'  Resetting stuck task {t[\"id\"]} (was in_progress)...')
        try:
            req = urllib.request.Request(
                '${DAEMON_API}/api/plan-db/task/update',
                data=json.dumps({'task_id': t['id'], 'status': 'pending', 'summary': 'Reset by runner'}).encode(),
                headers={'Content-Type': 'application/json'},
                method='POST')
            urllib.request.urlopen(req)
        except: pass
" 2>/dev/null || true

	# cd to worktree
	if [ -n "$WORKTREE" ] && [ -d "$WORKTREE" ]; then
		echo "[INFO] Worktree: $WORKTREE"
		cd "$WORKTREE"
		# Git sync before starting
		git pull --rebase origin main 2>/dev/null || true
	fi

	# Read prompt from temp file
	PROMPT="$(cat "$PROMPT_FILE" 2>/dev/null || echo "/execute $PLAN_ID")"
	rm -f "$PROMPT_FILE" 2>/dev/null || true

	# Use claude or copilot
	if command -v claude &>/dev/null; then
		CLI="claude"
		CLI_ARGS="--dangerously-skip-permissions -p"
	elif command -v copilot &>/dev/null; then
		CLI="copilot"
		CLI_ARGS="--yolo -p"
	else
		echo "[FAIL] No CLI found" >&2; exit 1
	fi

	echo "[INFO] Launching $CLI..."
	$CLI $CLI_ARGS "$PROMPT" 2>&1
	EXIT_CODE=$?

	echo ""
	echo "[Run ${RETRY}] $CLI exited (code ${EXIT_CODE}). Checking..."
	sleep 5
done

echo ""
echo "=== Plan #${PLAN_ID} COMPLETE ==="
