#!/usr/bin/env bash
# setup-gh-credentials.sh — Per-repo GitHub credential routing.
# Eliminates gh auth switch by configuring each repo's credential.helper
# to use gh auth token directly. Only targets repos with github.com remotes.
#
# Usage: setup-gh-credentials.sh [--scan-dir <dir>] [--dry-run] [--help]
#   --scan-dir <dir>  Directory containing git repos (default: ~/GitHub)
#   --dry-run         Print actions without modifying git config
#   --help            Show this help
set -euo pipefail

C='\033[0;36m' G='\033[0;32m' Y='\033[1;33m' N='\033[0m'
ok()   { echo -e "${G}[OK]${N} $*"; }
info() { echo -e "${C}[->]${N} $*"; }
warn() { echo -e "${Y}[!]${N} $*"; }

SCAN_DIR="${HOME}/GitHub"
DRY_RUN=false

usage() {
  echo "Usage: setup-gh-credentials.sh [--scan-dir <dir>] [--dry-run] [--help]"
  echo "  --scan-dir <dir>  Directory containing git repos (default: ~/GitHub)"
  echo "  --dry-run         Print actions without modifying git config"
  echo "  --help            Show this help"
  exit 0
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --scan-dir) SCAN_DIR="$2"; shift 2 ;;
    --dry-run)  DRY_RUN=true; shift ;;
    --help)     usage ;;
    *)          warn "Unknown option: $1"; shift ;;
  esac
done

# Verify gh CLI is available
if ! command -v gh &>/dev/null; then
  warn "gh CLI not found — cannot configure credential routing"
  exit 1
fi

# Verify gh is authenticated
if ! gh auth status &>/dev/null 2>&1; then
  warn "gh not authenticated — run 'gh auth login' first"
  exit 1
fi

CREDENTIAL_HELPER='!gh auth token'
CONFIGURED=0
SKIPPED=0

info "Scanning repos in ${SCAN_DIR}..."

for repo_dir in "$SCAN_DIR"/*/; do
  [[ -d "${repo_dir}.git" ]] || continue

  local_name="$(basename "$repo_dir")"

  # Get origin URL (HTTPS or SSH)
  origin_url="$(git -C "$repo_dir" remote get-url origin 2>/dev/null || echo "")"
  [[ -z "$origin_url" ]] && continue

  # Only process github.com repos
  if ! echo "$origin_url" | grep -qE '(github\.com[:/])'; then
    continue
  fi

  # Extract org/owner from URL for logging
  if echo "$origin_url" | grep -qE '^https://'; then
    owner="$(echo "$origin_url" | sed -E 's|https://github\.com/([^/]+)/.*|\1|')"
  elif echo "$origin_url" | grep -qE '^git@'; then
    owner="$(echo "$origin_url" | sed -E 's|git@github\.com:([^/]+)/.*|\1|')"
  else
    owner="unknown"
  fi

  if [[ "$DRY_RUN" == "true" ]]; then
    info "[dry-run] ${local_name} (${owner}) — would set credential.helper"
    CONFIGURED=$((CONFIGURED + 1))
    continue
  fi

  # Set per-repo credential helper to use gh auth token
  git -C "$repo_dir" config --local credential.helper "$CREDENTIAL_HELPER" 2>/dev/null
  ok "${local_name} (${owner}) — credential.helper configured"
  CONFIGURED=$((CONFIGURED + 1))
done

if [[ "$DRY_RUN" == "true" ]]; then
  info "Dry run complete: ${CONFIGURED} repos would be configured"
else
  ok "Done: ${CONFIGURED} repos configured, ${SKIPPED} skipped"
fi
