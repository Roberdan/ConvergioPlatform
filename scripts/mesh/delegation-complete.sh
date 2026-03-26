#!/usr/bin/env bash
# delegation-complete.sh — Completion callback for delegated claude tasks on peer
# Called when claude exits on peer (post-exit hook or explicit call from mesh-delegate-task.sh)
#
# Responsibilities:
#   1. Signal coordinator daemon that delegation is done (POST /api/delegation/:id/progress)
#   2. Run final rsync to push work back to coordinator via origin
#   3. Remove prompt file on this peer
#   4. Kill the tmux session used for the delegation
#
# Usage: delegation-complete.sh --session-name <name> --prompt-file <path> --plan-id <N>
#          [--peer-addr <http://host:8420>] [--result done|failed]
# Env:   DRY_RUN=1  — stub network/tmux calls (used in tests)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# ── defaults ─────────────────────────────────────────────────────────────────
SESSION_NAME=""
PROMPT_FILE=""
PLAN_ID=""
PEER_ADDR="${COORDINATOR_ADDR:-http://localhost:8420}"
RESULT="done"
DRY_RUN="${DRY_RUN:-0}"

# ── parse args ────────────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case "$1" in
    --session-name) SESSION_NAME="$2"; shift 2 ;;
    --prompt-file)  PROMPT_FILE="$2";  shift 2 ;;
    --plan-id)      PLAN_ID="$2";      shift 2 ;;
    --peer-addr)    PEER_ADDR="$2";    shift 2 ;;
    --result)       RESULT="$2";       shift 2 ;;
    *)
      echo "ERROR: Unknown argument: $1" >&2
      echo "Usage: delegation-complete.sh --session-name <name> --prompt-file <path> --plan-id <N> [--peer-addr <url>] [--result done|failed]" >&2
      exit 1
      ;;
  esac
done

if [[ -z "$SESSION_NAME" || -z "$PROMPT_FILE" || -z "$PLAN_ID" ]]; then
  echo "ERROR: Required: --session-name, --prompt-file, --plan-id" >&2
  echo "Usage: delegation-complete.sh --session-name <name> --prompt-file <path> --plan-id <N>" >&2
  exit 1
fi

# ── helpers ───────────────────────────────────────────────────────────────────

log() { echo "[delegation-complete] $*"; }

# POST progress update to coordinator daemon
post_delegation_done() {
  local url="${PEER_ADDR}/api/delegation/progress"
  local payload
  payload="$(printf '{"plan_id":%s,"session":"%s","status":"%s","result":"%s"}' \
    "$PLAN_ID" "$SESSION_NAME" "done" "$RESULT")"

  if [[ "$DRY_RUN" == "1" ]]; then
    log "DRY_RUN: curl -sf -X POST $url -d $payload"
    return 0
  fi

  # Try daemon API first; fall back to cvg CLI if daemon unreachable
  if curl -sf -X POST "$url" \
      -H "Content-Type: application/json" \
      -d "$payload" \
      --max-time 10 2>/dev/null; then
    log "delegation marked done via API (plan $PLAN_ID)"
  else
    log "WARN: daemon unreachable — signalling via cvg CLI"
    # cvg delegation complete is not yet a command; use plan-db agent complete
    curl -sf -X POST "${PEER_ADDR}/api/plan-db/agent/complete" \
      -H "Content-Type: application/json" \
      -d "{\"agent_id\":\"delegate-${SESSION_NAME}\"}" \
      --max-time 10 2>/dev/null || log "WARN: cvg fallback also failed; coordinator will detect via polling"
  fi
}

# Push peer commits to origin so coordinator can pull them
run_final_sync() {
  if [[ "$DRY_RUN" == "1" ]]; then
    log "DRY_RUN: git push origin main"
    return 0
  fi

  local platform_dir
  platform_dir="$(cd "$SCRIPT_DIR/../.." && pwd)"

  log "pushing peer work to origin..."
  if cd "$platform_dir" && git push origin main 2>&1; then
    log "final sync OK"
  else
    log "WARN: git push failed — coordinator will detect via polling"
  fi
}

# Remove the prompt markdown file written by mesh-delegate-task.sh
remove_prompt_file() {
  if [[ -f "$PROMPT_FILE" ]]; then
    rm -f "$PROMPT_FILE"
    log "prompt file removed: $PROMPT_FILE"
  else
    log "prompt file already absent: $PROMPT_FILE"
  fi
}

# Kill the tmux session used for this delegation
kill_tmux_session() {
  if [[ "$DRY_RUN" == "1" ]]; then
    log "DRY_RUN: tmux kill-session -t $SESSION_NAME"
    # Still call stub so tests can capture
    tmux kill-session -t "$SESSION_NAME" 2>/dev/null || true
    return 0
  fi

  if tmux has-session -t "$SESSION_NAME" 2>/dev/null; then
    tmux kill-session -t "$SESSION_NAME"
    log "tmux session killed: $SESSION_NAME"
  else
    log "tmux session already gone: $SESSION_NAME"
  fi
}

# ── main ──────────────────────────────────────────────────────────────────────

log "starting (session=$SESSION_NAME plan=$PLAN_ID result=$RESULT)"

post_delegation_done
run_final_sync
remove_prompt_file
kill_tmux_session

log "done"
