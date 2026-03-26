#!/usr/bin/env bash
# post-commit-sync.sh — Sync worktree back to coordinator via rsync after each commit.
# Install as git post-commit hook: ln -s ../../scripts/mesh/post-commit-sync.sh .git/hooks/post-commit
#
# SYNC MODEL: rsync (filesystem), NOT git push. Git is for commits only.
# On rsync failure: saves .patch in .git/pending-patches/ for retry on next heartbeat.
# Heartbeat retry: source this script with --retry-pending to flush the queue.
#
# Env vars (set in worktree or passed by hook):
#   CONVERGIO_COORDINATOR_HOST        — SSH hostname or alias of coordinator
#   CONVERGIO_COORDINATOR_PATH        — Remote worktree path on coordinator
#   CONVERGIO_COORDINATOR_LOCAL_PATH  — (optional) skip SSH, use local path
#   CONVERGIO_DELEGATION_ID           — delegation ID for daemon sync-status API
#   CONVERGIO_DAEMON_URL              — daemon base URL (default: http://localhost:8420)
#
# Usage (direct): post-commit-sync.sh [--help] [--retry-pending] [--list-pending]
#                 post-commit-sync.sh --repo-path <path> [--dry-run-patch] [--local-sync]
set -euo pipefail

REPO_PATH="${CONVERGIO_REPO_PATH:-$(git rev-parse --show-toplevel 2>/dev/null || echo "")}"
COORDINATOR_HOST="${CONVERGIO_COORDINATOR_HOST:-}"
COORDINATOR_PATH="${CONVERGIO_COORDINATOR_PATH:-}"
COORDINATOR_LOCAL="${CONVERGIO_COORDINATOR_LOCAL_PATH:-}"
DELEGATION_ID="${CONVERGIO_DELEGATION_ID:-}"
DAEMON_URL="${CONVERGIO_DAEMON_URL:-http://localhost:8420}"
DRY_RUN_PATCH=false
LOCAL_SYNC=false
MODE="sync"

# Logging helpers
_info() { echo "[post-commit-sync] $*"; }
_warn() { echo "[post-commit-sync] WARN: $*" >&2; }
_err()  { echo "[post-commit-sync] ERROR: $*" >&2; }

cleanup() { :; }  # placeholder for trap — extended per subshell if needed
trap cleanup EXIT

# Parse args
while [[ $# -gt 0 ]]; do
  case "$1" in
    --help|-h)
      echo "Usage: post-commit-sync.sh [OPTIONS]"
      echo ""
      echo "Options:"
      echo "  --repo-path <path>    Override repo root (default: git rev-parse)"
      echo "  --retry-pending       Flush all pending-patches to coordinator"
      echo "  --list-pending        List pending patch files and exit"
      echo "  --dry-run-patch       Simulate rsync failure: save patch, skip real rsync"
      echo "  --local-sync          Rsync to CONVERGIO_COORDINATOR_LOCAL_PATH (no SSH)"
      echo "  --help                Show this help"
      exit 0
      ;;
    --repo-path) REPO_PATH="$2"; shift 2 ;;
    --retry-pending) MODE="retry"; shift ;;
    --list-pending)  MODE="list";  shift ;;
    --dry-run-patch) DRY_RUN_PATCH=true; shift ;;
    --local-sync)    LOCAL_SYNC=true; shift ;;
    *) _err "Unknown argument: $1"; exit 1 ;;
  esac
done

# Resolve repo path
if [[ -z "$REPO_PATH" ]]; then
  _err "Cannot detect git repo root. Run from inside a git worktree or pass --repo-path."
  exit 1
fi
PATCH_DIR="$REPO_PATH/.git/pending-patches"

# Notify daemon of sync status (non-fatal — daemon may be down during delegated work)
_notify_daemon() {
  local status="$1" message="${2:-}"
  [[ -z "$DELEGATION_ID" ]] && return 0
  curl -sf -X POST "${DAEMON_URL}/api/delegation/${DELEGATION_ID}/sync-status" \
    -H "Content-Type: application/json" \
    -d "{\"status\":\"${status}\",\"message\":$(printf '%s' "\"${message}\"")}" \
    2>/dev/null || true
}

# Save a patch file for later retry
_save_patch() {
  local commit_sha reason
  commit_sha="$(git -C "$REPO_PATH" rev-parse --short HEAD 2>/dev/null || echo "unknown")"
  reason="${1:-rsync-failed}"
  mkdir -p "$PATCH_DIR"
  local patch_file
  patch_file="$PATCH_DIR/${commit_sha}-$(date +%s).patch"
  if git -C "$REPO_PATH" format-patch -1 --stdout HEAD >"$patch_file" 2>/dev/null; then
    _warn "Saved patch for retry: $patch_file (reason: $reason)"
    _notify_daemon "patch_saved" "commit=${commit_sha} reason=${reason}"
  else
    _warn "Could not create patch for ${commit_sha} — worktree may have no commits"
    rm -f "$patch_file"
  fi
}

# Core rsync: push worktree contents to coordinator
_rsync_to_coordinator() {
  local src="${REPO_PATH}/"  # trailing slash = sync contents
  local excludes=(
    --exclude='node_modules/'
    --exclude='.next/'
    --exclude='target/release/'
    --exclude='target/debug/'
    --exclude='__pycache__/'
    --exclude='.pytest_cache/'
    --exclude='test-results/'
    --exclude='.DS_Store'
    --exclude='*.pyc'
  )

  if [[ "$LOCAL_SYNC" == "true" || -n "$COORDINATOR_LOCAL" ]]; then
    # Local path rsync (used in tests and same-machine setups)
    local dst="${COORDINATOR_LOCAL:-$COORDINATOR_PATH}"
    [[ -z "$dst" ]] && { _err "CONVERGIO_COORDINATOR_LOCAL_PATH not set"; return 1; }
    mkdir -p "$dst"
    rsync -az "${excludes[@]}" "$src" "${dst}/" 2>/dev/null
    return $?
  fi

  # SSH rsync to remote coordinator
  [[ -z "$COORDINATOR_HOST" ]] && { _err "CONVERGIO_COORDINATOR_HOST not set"; return 1; }
  [[ -z "$COORDINATOR_PATH" ]] && { _err "CONVERGIO_COORDINATOR_PATH not set"; return 1; }
  rsync -az \
    -e "ssh -o ConnectTimeout=10 -o BatchMode=yes -o StrictHostKeyChecking=accept-new" \
    "${excludes[@]}" \
    "$src" \
    "${COORDINATOR_HOST}:${COORDINATOR_PATH}/" \
    2>/dev/null
  return $?
}

# Retry all pending patches by rsyncing the full worktree
_retry_pending() {
  if [[ ! -d "$PATCH_DIR" ]]; then
    _info "No pending patches directory at $PATCH_DIR"
    return 0
  fi
  local count
  count="$(find "$PATCH_DIR" -name "*.patch" 2>/dev/null | wc -l | tr -d ' ')"
  if [[ "$count" -eq 0 ]]; then
    _info "No pending patches to retry."
    return 0
  fi
  _info "Retrying sync with $count pending patch(es)..."
  if _rsync_to_coordinator; then
    rm -f "$PATCH_DIR"/*.patch
    _info "Retry succeeded — $count patch(es) cleared."
    _notify_daemon "synced" "retry cleared ${count} patches"
  else
    _warn "Retry rsync failed — patches remain in $PATCH_DIR"
    _notify_daemon "retry_failed" "patches remain: ${count}"
    return 1
  fi
}

# List pending patches (for heartbeat inspection)
_list_pending() {
  _info "Pending patches directory: $PATCH_DIR"
  if [[ ! -d "$PATCH_DIR" ]]; then
    _info "  (no pending-patches directory)"
    return 0
  fi
  local count
  count="$(find "$PATCH_DIR" -name "*.patch" 2>/dev/null | wc -l | tr -d ' ')"
  _info "  Count: $count"
  find "$PATCH_DIR" -name "*.patch" 2>/dev/null | while read -r f; do
    _info "  - $(basename "$f")"
  done
}

# Main sync: rsync → on failure, save patch
_do_sync() {
  local commit_sha
  commit_sha="$(git -C "$REPO_PATH" rev-parse --short HEAD 2>/dev/null || echo "unknown")"
  _info "Post-commit sync for commit $commit_sha"

  if [[ "$DRY_RUN_PATCH" == "true" ]]; then
    # Simulate rsync failure — used by T5 integration test
    _warn "dry-run-patch: simulating rsync failure, saving patch"
    _save_patch "dry-run-patch"
    return 0
  fi

  if _rsync_to_coordinator; then
    _info "Sync OK → coordinator updated"
    _notify_daemon "synced" "commit=${commit_sha}"
    # Clear any pending patches from prior failures
    if [[ -d "$PATCH_DIR" ]]; then
      find "$PATCH_DIR" -name "*.patch" -delete 2>/dev/null || true
    fi
  else
    _warn "Rsync failed — saving patch for retry"
    _save_patch "rsync-failed"
    return 0  # Non-fatal: hook must not block the commit
  fi
}

# Dispatch
case "$MODE" in
  sync)   _do_sync ;;
  retry)  _retry_pending ;;
  list)   _list_pending ;;
esac
