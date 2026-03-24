#!/usr/bin/env bash
# mesh-delegate-task.sh — Delegate a task to a mesh peer via tmux + claude
# Usage: mesh-delegate-task.sh --peer <ssh_alias> --prompt "..." [--plan-id N] [--session-name X]
# Creates tmux session, writes prompt to file, syncs repo, launches claude, installs sync-back hook.
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
COORDINATOR_HOST="$(hostname -s)"
COORDINATOR_IP="$(ipconfig getifaddr en0 2>/dev/null || hostname -I 2>/dev/null | awk '{print $1}')"

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

# 1. Ensure remote has SSH remote pointing back to coordinator for sync-back
echo "[1/5] Setting up git SSH remote for sync-back..."
ssh "$PEER" "cd '$REPO_PATH' && \
  git remote remove coordinator 2>/dev/null; \
  git remote add coordinator '$COORDINATOR_HOST:$PLATFORM_DIR' 2>/dev/null || true" 2>/dev/null
echo "  OK: coordinator remote → $COORDINATOR_HOST:$PLATFORM_DIR"

# 2. Sync: push our main to remote
echo "[2/5] Syncing repo to peer..."
ssh "$PEER" "cd '$REPO_PATH' && git fetch coordinator main 2>/dev/null && git reset --hard coordinator/main 2>/dev/null" 2>/dev/null || \
  echo "  WARN: fetch failed, trying pull from origin"
ssh "$PEER" "cd '$REPO_PATH' && git pull origin main --ff-only 2>/dev/null || true" 2>/dev/null
echo "  OK: repo synced"

# 3. Write prompt to file on peer
echo "[3/5] Writing prompt to peer..."
PROMPT_FILE="/tmp/convergio-delegate-${SESSION_NAME}.md"
ssh "$PEER" "cat > '$PROMPT_FILE'" <<EOF
$PROMPT

IMPORTANT POST-TASK INSTRUCTIONS:
When you are done and have committed all changes:
1. Run: git push coordinator main
2. If push fails, run: git format-patch -1 HEAD --stdout > /tmp/last-delegate-patch.patch
3. Signal completion: curl -sf -X POST http://${COORDINATOR_IP}:8420/api/plan-db/agent/complete -H 'Content-Type: application/json' -d '{"agent_id":"delegate-${SESSION_NAME}"}' 2>/dev/null || true
EOF
echo "  OK: prompt written to $PROMPT_FILE"

# 4. Install post-commit hook for auto sync-back
echo "[4/5] Installing sync-back post-commit hook..."
ssh "$PEER" "mkdir -p '$REPO_PATH/.git/hooks' && cat > '$REPO_PATH/.git/hooks/post-commit-delegate' && chmod +x '$REPO_PATH/.git/hooks/post-commit-delegate'" <<'HOOKEOF'
#!/usr/bin/env bash
# Auto sync-back to coordinator after commit (installed by mesh-delegate-task.sh)
git push coordinator main 2>/dev/null || echo "[delegate] sync-back failed, patch saved" >&2
HOOKEOF

# Chain into existing post-commit if it exists
ssh "$PEER" "cd '$REPO_PATH' && \
  if [ ! -f .git/hooks/post-commit ]; then \
    echo '#!/usr/bin/env bash' > .git/hooks/post-commit; \
    chmod +x .git/hooks/post-commit; \
  fi; \
  grep -q 'post-commit-delegate' .git/hooks/post-commit 2>/dev/null || \
    echo '.git/hooks/post-commit-delegate 2>/dev/null || true' >> .git/hooks/post-commit" 2>/dev/null
echo "  OK: post-commit hook installed"

# 5. Create tmux session and launch claude
echo "[5/5] Launching Claude in tmux session..."
ssh "$PEER" "tmux kill-session -t '$SESSION_NAME' 2>/dev/null; \
  tmux new-session -d -s '$SESSION_NAME' -c '$REPO_PATH'; \
  tmux send-keys -t '$SESSION_NAME' 'claude --dangerously-skip-permissions --input-file $PROMPT_FILE' Enter" 2>/dev/null
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
if [[ -n "$PLAN_ID" ]]; then
  echo "  Plan:     cvg delegation status $PLAN_ID"
fi
