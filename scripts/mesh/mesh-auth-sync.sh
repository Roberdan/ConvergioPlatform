#!/usr/bin/env bash
set -euo pipefail
# mesh-auth-sync.sh — Sync Claude & Copilot auth from master to all mesh nodes
# Usage: mesh-auth-sync.sh [--peer NAME] [--check-only]
# Reads credentials from master node's keychain, writes to remote nodes.
# Claude: gnome-keyring (Linux) / macOS Keychain
# Copilot: gh auth token (via gh CLI)

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/lib/peers.sh"
peers_load

CHECK_ONLY=false
TARGET_PEER=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --check-only) CHECK_ONLY=true; shift ;;
    --peer) TARGET_PEER="${2:-}"; shift 2 ;;
    *) shift ;;
  esac
done

SELF="$(peers_self)"
G='\033[0;32m' R='\033[0;31m' Y='\033[1;33m' N='\033[0m'
RPATH='export PATH="/opt/homebrew/bin:/usr/local/bin:$HOME/.local/bin:$PATH";'

# --- Claude credentials ---
# Read from master (current node) keychain
_read_claude_creds() {
  local os
  os="$(uname -s)"
  if [[ "$os" == "Darwin" ]]; then
    security find-generic-password -s "Claude Code-credentials" -w 2>/dev/null || echo ""
  else
    secret-tool lookup service "Claude Code-credentials" account "Claude Code-credentials" 2>/dev/null || echo ""
  fi
}

_write_claude_creds_remote() {
  local peer="$1" creds="$2"
  local dest user target os
  dest="$(peers_best_route "$peer")"
  user="$(peers_get "$peer" "user" 2>/dev/null || echo "")"
  target="${user:+${user}@}${dest}"
  os="$(peers_get "$peer" "os" 2>/dev/null || echo "linux")"

  if [[ "$os" == "macos" ]]; then
    # macOS: write to keychain (may need unlocked keychain)
    ssh -n "$target" "${RPATH} security delete-generic-password -s 'Claude Code-credentials' 2>/dev/null; security add-generic-password -s 'Claude Code-credentials' -a 'Claude Code-credentials' -w '${creds}' 2>&1" 2>/dev/null
  else
    # Linux: write to gnome-keyring via secret-tool
    ssh -n "$target" "${RPATH} echo '${creds}' | secret-tool store --label='Claude Code-credentials' service 'Claude Code-credentials' account 'Claude Code-credentials' 2>&1" 2>/dev/null
  fi
}

_check_claude_remote() {
  local peer="$1"
  local dest user target
  dest="$(peers_best_route "$peer")"
  user="$(peers_get "$peer" "user" 2>/dev/null || echo "")"
  target="${user:+${user}@}${dest}"
  ssh -n -o ConnectTimeout=5 "$target" "${RPATH} claude auth status 2>/dev/null | grep -o '\"authMethod\": *\"[^\"]*\"' | head -1" 2>/dev/null || echo ""
}

# --- gh auth (for Copilot) ---
_check_gh_remote() {
  local peer="$1"
  local dest user target
  dest="$(peers_best_route "$peer")"
  user="$(peers_get "$peer" "user" 2>/dev/null || echo "")"
  target="${user:+${user}@}${dest}"
  ssh -n -o ConnectTimeout=5 "$target" "${RPATH} gh auth status 2>&1 | grep -q 'Logged in' && echo 'ok' || echo 'no'" 2>/dev/null || echo "no"
}

# --- Main ---
echo "Reading master credentials..."
CLAUDE_CREDS="$(_read_claude_creds)"
[[ -z "$CLAUDE_CREDS" ]] && { echo -e "${R}No Claude credentials found on master${N}"; }

_process_peer() {
  local peer="$1"
  [[ "$peer" == "$SELF" ]] && return

  if ! peers_check "$peer" 2>/dev/null; then
    echo -e "  ${Y}${peer}${N}: offline, skipped"
    return
  fi

  # Check Claude
  local claude_method
  claude_method="$(_check_claude_remote "$peer")"
  if echo "$claude_method" | grep -q "claude.ai"; then
    echo -e "  ${G}${peer}${N}: Claude auth OK (claude.ai)"
  elif echo "$claude_method" | grep -q "oauth_token"; then
    echo -e "  ${Y}${peer}${N}: Claude auth via oauth_token (may be API key)"
    if ! $CHECK_ONLY && [[ -n "$CLAUDE_CREDS" ]]; then
      _write_claude_creds_remote "$peer" "$CLAUDE_CREDS" && \
        echo -e "    -> credentials synced" || \
        echo -e "    ${R}-> sync failed${N}"
    fi
  else
    echo -e "  ${R}${peer}${N}: Claude NOT authenticated"
    if ! $CHECK_ONLY && [[ -n "$CLAUDE_CREDS" ]]; then
      _write_claude_creds_remote "$peer" "$CLAUDE_CREDS" && \
        echo -e "    -> credentials synced" || \
        echo -e "    ${R}-> sync failed (keychain locked?)${N}"
    fi
  fi

  # Check gh/Copilot
  local gh_status
  gh_status="$(_check_gh_remote "$peer")"
  if [[ "$gh_status" == "ok" ]]; then
    echo -e "  ${G}${peer}${N}: gh auth OK"
  else
    echo -e "  ${R}${peer}${N}: gh NOT authenticated"
    if ! $CHECK_ONLY; then
      echo -e "    ${Y}-> run on node: gh auth login${N}"
    fi
  fi
}

if [[ -n "$TARGET_PEER" ]]; then
  _process_peer "$TARGET_PEER"
else
  for peer in $(peers_list); do
    _process_peer "$peer"
  done
fi
