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

# Temp file on remote peer — unique per invocation to prevent race conditions
REMOTE_TMPFILE=""

_cleanup_remote_tmpfile() {
  if [[ -n "$REMOTE_TMPFILE" ]]; then
    ssh "$PEER" "rm -f $(printf '%q' "$REMOTE_TMPFILE")" 2>/dev/null || true
  fi
}
trap '_cleanup_remote_tmpfile' EXIT

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

# 3. Write prompt to file on peer — use mktemp for a unique path, avoiding races
echo "[3/4] Writing prompt to peer..."
REMOTE_TMPFILE="$(ssh "$PEER" 'mktemp /tmp/convergio-delegate-XXXXXX.md' 2>/dev/null)"
if [[ -z "$REMOTE_TMPFILE" ]]; then
  echo "ERROR: Could not create temp file on $PEER" >&2
  exit 1
fi
ssh "$PEER" "cat > $(printf '%q' "$REMOTE_TMPFILE")" <<EOF
$PROMPT

IMPORTANT POST-TASK INSTRUCTIONS:
When you are done and have committed all changes:
1. Run: git push origin main
2. Then signal completion to the daemon on localhost:8420
NEVER use sqlite3 directly — use cvg CLI or daemon API.
EOF
echo "  OK: prompt written to $REMOTE_TMPFILE"

# 4. Create tmux session and launch claude
echo "[4/4] Launching Claude in tmux session..."
local safe_remote_tmpfile safe_session safe_repo_path
safe_remote_tmpfile="$(printf '%q' "$REMOTE_TMPFILE")"
safe_session="$(printf '%q' "$SESSION_NAME")"
safe_repo_path="$(printf '%q' "$REPO_PATH")"
ssh "$PEER" "tmux kill-session -t ${safe_session} 2>/dev/null; \
  tmux new-session -d -s ${safe_session} -c ${safe_repo_path}; \
  tmux send-keys -t ${safe_session} 'claude -p \"\$(cat ${safe_remote_tmpfile})\" --dangerously-skip-permissions' Enter" 2>/dev/null
echo "  OK: Claude launched"

# Register delegation with daemon
curl -sf -X POST "http://localhost:8420/api/plan-db/agent/start" \
  -H "Content-Type: application/json" \
  -d "{\"agent_id\":\"delegate-${SESSION_NAME}\",\"description\":\"delegated to ${PEER}\"}" 2>/dev/null || true

echo ""
echo "=== Delegation Active ==="
echo "  Monitor:  ssh -t $PEER 'tmux attach -t $SESSION_NAME'"
echo "  Status:   ssh $PEER 'tmux capture-pane -t $SESSION_NAME -p | tail -20'"
echo "  Kill:     ssh $PEER 'tmux kill-session -t $SESSION_NAME'"
