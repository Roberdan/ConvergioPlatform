#!/usr/bin/env bash
# mesh-rsync.sh — rsync-based sync primitive for ConvergioPlatform mesh
# Preserves: hidden files, .env, .claude/, permissions, symlinks
# Usage: mesh-rsync.sh <local-path> <peer-dns> <remote-path> [--delete] [--dry-run]
set -euo pipefail

LOCAL_PATH="${1:?Usage: mesh-rsync.sh <local-path> <peer-dns> <remote-path>}"
PEER_DNS="${2:?peer-dns required}"
REMOTE_PATH="${3:?remote-path required}"
DELETE_FLAG=""
DRY_RUN=""

shift 3
while [[ $# -gt 0 ]]; do
  case "$1" in
    --delete) DELETE_FLAG="--delete"; shift ;;
    --dry-run) DRY_RUN="--dry-run"; shift ;;
    *) echo "Unknown: $1" >&2; exit 1 ;;
  esac
done

# Standard excludes — build artifacts only, NEVER hidden files
EXCLUDES=(
  --exclude='node_modules/'
  --exclude='.next/'
  --exclude='target/release/'
  --exclude='target/debug/'
  --exclude='__pycache__/'
  --exclude='.pytest_cache/'
  --exclude='test-results/'
  --exclude='playwright-report/'
  --exclude='.DS_Store'
  --exclude='*.pyc'
)

# Trailing slash on source = sync contents, not the dir itself
[[ "$LOCAL_PATH" != */ ]] && LOCAL_PATH="${LOCAL_PATH}/"
[[ "$REMOTE_PATH" != */ ]] && REMOTE_PATH="${REMOTE_PATH}/"

echo "[mesh-rsync] ${LOCAL_PATH} → ${PEER_DNS}:${REMOTE_PATH}"

rsync -avz \
  --progress \
  --human-readable \
  -e "ssh -o ConnectTimeout=10 -o BatchMode=yes -o StrictHostKeyChecking=accept-new" \
  "${EXCLUDES[@]}" \
  $DELETE_FLAG \
  $DRY_RUN \
  "$LOCAL_PATH" \
  "${PEER_DNS}:${REMOTE_PATH}"

echo "[mesh-rsync] Done: ${PEER_DNS}:${REMOTE_PATH}"
