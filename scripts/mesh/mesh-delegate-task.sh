#!/usr/bin/env bash
# mesh-delegate-task.sh — Delegate a task to a mesh peer via tmux + claude
# Usage: mesh-delegate-task.sh --peer <ssh_alias> --prompt "..." [--plan-id N] [--session-name X]
#        [--coordinator-url ssh://user@host/path]
#
# SYNC MODEL: GitHub (origin) is the single source of truth.
# Peer pulls from origin, works, pushes to origin. Coordinator pulls from origin.
# NEVER push directly between peers — prevents force-push disasters.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLATFORM_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Parse args
PEER="" PROMPT="" PLAN_ID="" SESSION_NAME="" REPO_PATH="" COORDINATOR_URL=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --peer)             PEER="$2";             shift 2 ;;
    --prompt)           PROMPT="$2";           shift 2 ;;
    --plan-id)          PLAN_ID="$2";          shift 2 ;;  # forwarded to remote daemon
    --session-name)     SESSION_NAME="$2";     shift 2 ;;
    --repo-path)        REPO_PATH="$2";        shift 2 ;;
    --coordinator-url)  COORDINATOR_URL="$2";  shift 2 ;;
    *) echo "Unknown arg: $1" >&2; exit 1 ;;
  esac
done

if [[ -z "$PEER" || -z "$PROMPT" ]]; then
  echo "Usage: mesh-delegate-task.sh --peer <ssh_alias> --prompt \"...\" [--plan-id N]" >&2
  exit 1
fi

SESSION_NAME="${SESSION_NAME:-convergio-$(date +%s)}"

# Detect remote repo path — REPO_PATH expands on coordinator (client) side intentionally
if [[ -z "$REPO_PATH" ]]; then
  # shellcheck disable=SC2029
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
echo "[1/5] Pushing latest to origin..."
cd "$PLATFORM_DIR" && git push origin main 2>/dev/null || echo "  (already up to date)"

# 2. Setup coordinator git remote on peer (idempotent — skips if already configured)
echo "[2/5] Configuring coordinator git remote on peer..."
if [[ -n "$COORDINATOR_URL" ]]; then
  "$SCRIPT_DIR/setup-coordinator-remote.sh" \
    --peer "$PEER" \
    --peer-repo-path "$REPO_PATH" \
    --coordinator-url "$COORDINATOR_URL"
else
  echo "  INFO: --coordinator-url not provided, skipping coordinator remote setup"
fi

# 3. Sync peer from origin
# REPO_PATH and tail -2 expand on coordinator side intentionally (SC2029)
echo "[3/5] Syncing peer from origin..."
# shellcheck disable=SC2029
ssh "$PEER" "cd '$REPO_PATH' && git pull origin main --ff-only 2>&1 | tail -2" 2>/dev/null
echo "  OK: peer synced from origin"

# 4. Write prompt to file on peer
# EOF is intentionally unquoted so $PROMPT and instructions expand on coordinator side (SC2087)
echo "[4/5] Writing prompt to peer..."
PROMPT_FILE="/tmp/convergio-delegate-${SESSION_NAME}.md"
# PROMPT_FILE expands on coordinator side intentionally (SC2029)
# shellcheck disable=SC2029,SC2087
ssh "$PEER" "cat > '$PROMPT_FILE'" <<EOF
$PROMPT

IMPORTANT POST-TASK INSTRUCTIONS:
When you are done and have committed all changes:
1. Run: git push origin main
2. Then signal completion to the daemon on localhost:8420
NEVER use sqlite3 directly — use cvg CLI or daemon API.
EOF
echo "  OK: prompt written to $PROMPT_FILE"

# 5. Create tmux session and launch claude
# SESSION_NAME, REPO_PATH, PROMPT_FILE all expand on coordinator side intentionally (SC2029)
echo "[5/5] Launching Claude in tmux session..."
# shellcheck disable=SC2029
ssh "$PEER" "tmux kill-session -t '$SESSION_NAME' 2>/dev/null; \
  tmux new-session -d -s '$SESSION_NAME' -c '$REPO_PATH'; \
  tmux send-keys -t '$SESSION_NAME' 'claude -p \"\$(cat $PROMPT_FILE)\" --dangerously-skip-permissions' Enter" 2>/dev/null
echo "  OK: Claude launched"

# Register delegation with daemon
curl -sf -X POST "http://localhost:8420/api/plan-db/agent/start" \
  -H "Content-Type: application/json" \
  -d "{\"agent_id\":\"delegate-${SESSION_NAME}\",\"description\":\"delegated to ${PEER}\"${PLAN_ID:+,\"plan_id\":\"${PLAN_ID}\"}}" 2>/dev/null || true

echo ""
echo "=== Delegation Active ==="
echo "  Monitor:  ssh -t $PEER 'tmux attach -t $SESSION_NAME'"
echo "  Status:   ssh $PEER 'tmux capture-pane -t $SESSION_NAME -p | tail -20'"
echo "  Kill:     ssh $PEER 'tmux kill-session -t $SESSION_NAME'"
