#!/usr/bin/env bash
# mesh-delegate-task.sh — Delegate a task to a mesh peer via tmux + claude
# Usage: mesh-delegate-task.sh --peer <ssh_alias> --prompt "..." [--plan-id N] [--session-name X]
#
# SYNC MODEL: GitHub (origin) is the single source of truth.
# Peer pulls from origin, works, pushes to origin. Coordinator pulls from origin.
# NEVER push directly between peers — prevents force-push disasters.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLATFORM_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Parse args
PEER="" PROMPT="" PLAN_ID="" SESSION_NAME="" REPO_PATH=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --peer) PEER="$2"; shift 2 ;;
    --prompt) PROMPT="$2"; shift 2 ;;
    --plan-id) PLAN_ID="$2"; shift 2 ;;
    --session-name) SESSION_NAME="$2"; shift 2 ;;
    --repo-path) REPO_PATH="$2"; shift 2 ;;
    *) echo "Unknown arg: $1" >&2; exit 1 ;;
  esac
done

if [[ -z "$PEER" || -z "$PROMPT" ]]; then
  echo "Usage: mesh-delegate-task.sh --peer <ssh_alias> --prompt \"...\" [--plan-id N]" >&2
  exit 1
fi

SESSION_NAME="${SESSION_NAME:-convergio-$(date +%s)}"

# Detect remote repo path
if [[ -z "$REPO_PATH" ]]; then
  REPO_PATH=$(ssh "$PEER" "find /Users -maxdepth 4 -name 'ConvergioPlatform' -type d 2>/dev/null | grep GitHub | head -1" 2>/dev/null)
  if [[ -z "$REPO_PATH" ]]; then
    echo "ERROR: Cannot find ConvergioPlatform on $PEER" >&2
    exit 1
  fi
fi

echo "=== Mesh Delegate ==="
echo "  Peer:    $PEER"
echo "  Repo:    $REPO_PATH"
echo "  Session: $SESSION_NAME"
echo ""

# 1. Push coordinator's latest to origin first
echo "[1/4] Pushing latest to origin..."
cd "$PLATFORM_DIR" && git push origin main 2>/dev/null || echo "  (already up to date)"

# 2. Sync peer from origin
echo "[2/4] Syncing peer from origin..."
ssh "$PEER" "cd '$REPO_PATH' && git pull origin main --ff-only 2>&1 | tail -2" 2>/dev/null
echo "  OK: peer synced from origin"

# 3. Write prompt to file on peer
echo "[3/4] Writing prompt to peer..."
PROMPT_FILE="/tmp/convergio-delegate-${SESSION_NAME}.md"
ssh "$PEER" "cat > '$PROMPT_FILE'" <<EOF
$PROMPT

IMPORTANT POST-TASK INSTRUCTIONS:
When you are done and have committed all changes:
1. Run: git push origin main
2. Then signal completion to the daemon on localhost:8420
NEVER use sqlite3 directly — use cvg CLI or daemon API.
EOF
echo "  OK: prompt written to $PROMPT_FILE"

# 4. Copy delegation-complete.sh to peer so post-exit hook can call it
echo "[4/4] Launching Claude in tmux session..."
COMPLETION_SCRIPT="$(cd "$SCRIPT_DIR" && pwd)/delegation-complete.sh"
if [[ -f "$COMPLETION_SCRIPT" ]]; then
  REMOTE_COMPLETION="/tmp/delegation-complete-${SESSION_NAME}.sh"
  scp -q "$COMPLETION_SCRIPT" "${PEER}:${REMOTE_COMPLETION}" 2>/dev/null \
    && ssh "$PEER" "chmod +x '${REMOTE_COMPLETION}'" 2>/dev/null \
    && echo "  OK: delegation-complete.sh copied to peer" \
    || echo "  WARN: could not copy delegation-complete.sh; cleanup will be manual"
else
  echo "  WARN: delegation-complete.sh not found; cleanup will be manual"
  REMOTE_COMPLETION=""
fi

# Build post-exit command: when claude process exits, run cleanup on peer
PLAN_ID_ARG="${PLAN_ID:-0}"
if [[ -n "$REMOTE_COMPLETION" ]]; then
  POST_EXIT_CMD="'${REMOTE_COMPLETION}' --session-name '${SESSION_NAME}' --prompt-file '${PROMPT_FILE}' --plan-id '${PLAN_ID_ARG}'"
else
  POST_EXIT_CMD="true"
fi

ssh "$PEER" "tmux kill-session -t '$SESSION_NAME' 2>/dev/null; \
  tmux new-session -d -s '$SESSION_NAME' -c '$REPO_PATH'; \
  tmux send-keys -t '$SESSION_NAME' \
    'claude -p \"\$(cat $PROMPT_FILE)\" --dangerously-skip-permissions; ${POST_EXIT_CMD}' \
    Enter" 2>/dev/null
echo "  OK: Claude launched (completion hook registered)"

# Register delegation with daemon
curl -sf -X POST "http://localhost:8420/api/plan-db/agent/start" \
  -H "Content-Type: application/json" \
  -d "{\"agent_id\":\"delegate-${SESSION_NAME}\",\"description\":\"delegated to ${PEER}\"}" 2>/dev/null || true

echo ""
echo "=== Delegation Active ==="
echo "  Monitor:    ssh -t $PEER 'tmux attach -t $SESSION_NAME'"
echo "  Status:     ssh $PEER 'tmux capture-pane -t $SESSION_NAME -p | tail -20'"
echo "  Kill:       ssh $PEER 'tmux kill-session -t $SESSION_NAME'"
echo "  On complete: ${REMOTE_COMPLETION:-delegation-complete.sh (not available)}"
