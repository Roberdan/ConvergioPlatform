#!/usr/bin/env bash
# auto-rebuild.sh — Auto-rebuild daemon after git pull.
# Runs via launchd every 5 minutes. Pulls latest main, rebuilds daemon
# if daemon/ files changed, restarts if binary changed, notifies via Telegram.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly DAEMON_DIR="$REPO_ROOT/daemon"
readonly DAEMON_BIN="$DAEMON_DIR/target/release/convergio-platform-daemon"
readonly LOCK_FILE="/tmp/convergio-auto-rebuild.lock"
readonly LOG_FILE="/tmp/convergio-auto-rebuild.log"
readonly DAEMON_PORT="8420"
readonly HEALTH_URL="http://localhost:${DAEMON_PORT}/api/health"
readonly NOTIFY_URL="http://localhost:${DAEMON_PORT}/api/notify"
readonly HOSTNAME="$(hostname -s)"

# launchd/nohup don't inherit shell env
if [[ -f "$HOME/.convergio/env" ]]; then
  set -a
  # shellcheck source=/dev/null
  . "$HOME/.convergio/env"
  set +a
fi

# Daemon needs many FDs for SQLite + HTTP + background tasks
ulimit -n 10240 2>/dev/null || true

cleanup() {
  rm -f "$LOCK_FILE"
}
trap cleanup EXIT

log() { echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" >> "$LOG_FILE"; }

notify_result() {
  local severity="$1" title="$2" message="$3"
  curl -sf -X POST "$NOTIFY_URL" \
    -H "Content-Type: application/json" \
    -d "{\"severity\":\"$severity\",\"title\":\"$title\",\"message\":\"$message\"}" \
    >> "$LOG_FILE" 2>&1 || true
}

# Prevent overlapping runs (launchd can re-fire)
if [[ -f "$LOCK_FILE" ]]; then
  LOCK_PID=$(cat "$LOCK_FILE" 2>/dev/null || echo "")
  if [[ -n "$LOCK_PID" ]] && kill -0 "$LOCK_PID" 2>/dev/null; then
    log "Another auto-rebuild is running (PID $LOCK_PID), skipping."
    # Remove trap so we don't delete active lock
    trap - EXIT
    exit 0
  fi
  log "Stale lock file found, removing."
  rm -f "$LOCK_FILE"
fi
echo $$ > "$LOCK_FILE"

cd "$REPO_ROOT"

# Record HEAD before pull
BEFORE_SHA="$(git rev-parse HEAD)"

# Step 1: git pull --rebase origin main
log "Pulling latest from origin/main..."
if ! git pull --rebase origin main >> "$LOG_FILE" 2>&1; then
  log "FAIL: git pull failed"
  notify_result "error" "Auto-rebuild failed ($HOSTNAME)" "git pull --rebase failed. Manual intervention required."
  exit 1
fi

AFTER_SHA="$(git rev-parse HEAD)"

# No changes — nothing to do
if [[ "$BEFORE_SHA" == "$AFTER_SHA" ]]; then
  log "No new commits. Nothing to rebuild."
  exit 0
fi

# Step 2: Check if daemon/ files changed
CHANGED_FILES="$(git diff --name-only "$BEFORE_SHA" "$AFTER_SHA")"
if ! echo "$CHANGED_FILES" | grep -q '^daemon/'; then
  log "Changes detected but none in daemon/. Skipping rebuild."
  exit 0
fi

log "daemon/ files changed. Rebuilding..."

# Record binary checksum before build
BEFORE_BIN_SUM=""
if [[ -f "$DAEMON_BIN" ]]; then
  BEFORE_BIN_SUM="$(shasum -a 256 "$DAEMON_BIN" | awk '{print $1}')"
fi

# Step 3: Build daemon with kernel feature
cd "$DAEMON_DIR"
if ! cargo build --features kernel --release >> "$LOG_FILE" 2>&1; then
  log "FAIL: cargo build failed"
  notify_result "error" "Auto-rebuild failed ($HOSTNAME)" "cargo build --features kernel --release failed."
  exit 1
fi
cd "$REPO_ROOT"

log "Build succeeded."

# Step 4: Check if binary actually changed
AFTER_BIN_SUM=""
if [[ -f "$DAEMON_BIN" ]]; then
  AFTER_BIN_SUM="$(shasum -a 256 "$DAEMON_BIN" | awk '{print $1}')"
fi

if [[ "$BEFORE_BIN_SUM" == "$AFTER_BIN_SUM" ]]; then
  log "Binary unchanged after rebuild. No restart needed."
  notify_result "info" "Auto-rebuild ($HOSTNAME)" "Rebuilt daemon (no binary change). Commits: ${BEFORE_SHA:0:7}..${AFTER_SHA:0:7}"
  exit 0
fi

# Step 5: Restart daemon via start.sh
log "Binary changed. Restarting daemon..."
# Graceful stop: send SIGTERM to existing daemon
pkill -f "convergio-platform-daemon" 2>/dev/null || true
sleep 2

# Start daemon in background via start.sh
nohup "$DAEMON_DIR/start.sh" >> "$LOG_FILE" 2>&1 &
DAEMON_PID=$!
log "Started daemon (PID $DAEMON_PID)"

# Wait for health check (max 30s)
RETRIES=15
for ((i = 1; i <= RETRIES; i++)); do
  sleep 2
  if curl -sf "$HEALTH_URL" > /dev/null 2>&1; then
    log "Daemon healthy after restart."
    COMMIT_RANGE="${BEFORE_SHA:0:7}..${AFTER_SHA:0:7}"
    notify_result "success" "Auto-rebuild OK ($HOSTNAME)" "Daemon rebuilt and restarted. Commits: $COMMIT_RANGE"
    exit 0
  fi
done

log "FAIL: Daemon did not become healthy after restart."
notify_result "error" "Auto-rebuild failed ($HOSTNAME)" "Daemon rebuilt but health check failed after restart."
exit 1
