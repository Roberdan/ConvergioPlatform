#!/usr/bin/env bash
# Start the ConvergioPlatform daemon (or TUI dashboard)
# Usage:
#   ./daemon/start.sh              — start daemon (default: serve)
#   ./daemon/start.sh tui          — launch TUI dashboard
#   ./daemon/start.sh tui --api-url http://host:8420
set -euo pipefail

# Daemon needs many FDs for SQLite + HTTP + background tasks.
# macOS default is 256 which causes "Too many open files" errors.
ulimit -n 10240 2>/dev/null || true

# Load Convergio credentials if available.
# nohup/launchd don't inherit shell env, so we source explicitly.
if [ -f "$HOME/.convergio/env" ]; then
  set -a
  . "$HOME/.convergio/env"
  set +a
fi

# Load Telegram token from macOS Keychain if not already set.
# The token is stored under service "convergio-telegram-token".
if [ -z "${CONVERGIO_TELEGRAM_TOKEN:-}" ] && command -v security >/dev/null 2>&1; then
  token=$(security find-generic-password -s "convergio-telegram-token" -w 2>/dev/null) || true
  if [ -n "$token" ]; then
    export CONVERGIO_TELEGRAM_TOKEN="$token"
  fi
fi
if [ -z "${CONVERGIO_TELEGRAM_CHAT_ID:-}" ] && command -v security >/dev/null 2>&1; then
  chat_id=$(security find-generic-password -s "convergio-telegram-chat-id" -w 2>/dev/null) || true
  if [ -n "$chat_id" ]; then
    export CONVERGIO_TELEGRAM_CHAT_ID="$chat_id"
  fi
fi

# Export repo root so daemon APIs can resolve project-relative paths.
export CONVERGIO_REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# ── CWD Guard (2026-03-31 incident) ──────────────────────────────────────────
# Daemon MUST run from the main repo checkout, never from a worktree.
# A worktree CWD causes git_clean evidence gate failures and auth issues.
REPO_GITDIR="$(git -C "$CONVERGIO_REPO_ROOT" rev-parse --git-dir 2>/dev/null)" || true
REPO_COMMON="$(git -C "$CONVERGIO_REPO_ROOT" rev-parse --git-common-dir 2>/dev/null)" || true
if [ -n "$REPO_GITDIR" ] && [ -n "$REPO_COMMON" ] && [ "$REPO_GITDIR" != "$REPO_COMMON" ]; then
  echo "FATAL: start.sh invoked from a worktree ($CONVERGIO_REPO_ROOT)." >&2
  echo "Daemon must run from the main repo. cd to the main checkout first." >&2
  exit 1
fi

# ── Auth Token Guard ──────────────────────────────────────────────────────────
# Without CONVERGIO_AUTH_TOKEN, daemon runs in auth-required mode but nothing
# can authenticate → all API calls return 401. Auto-provision a dev token.
if [ -z "${CONVERGIO_AUTH_TOKEN:-}" ]; then
  export CONVERGIO_AUTH_TOKEN="dev-local-$(hostname -s)"
  echo "WARN: CONVERGIO_AUTH_TOKEN not set. Auto-provisioned: $CONVERGIO_AUTH_TOKEN" >&2
  # Persist so MCP server and CLI can use it
  grep -q "CONVERGIO_AUTH_TOKEN" "$HOME/.convergio/env" 2>/dev/null || \
    echo "CONVERGIO_AUTH_TOKEN=$CONVERGIO_AUTH_TOKEN" >> "$HOME/.convergio/env"
fi

DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$DIR"

# Check if binary exists
if [ -f target/release/convergio-platform-daemon ]; then
  ./target/release/convergio-platform-daemon "$@"
elif [ -f target/debug/convergio-platform-daemon ]; then
  echo "WARN: Using debug build"
  ./target/debug/convergio-platform-daemon "$@"
else
  echo "Building daemon..."
  cargo build --release
  ./target/release/convergio-platform-daemon "$@"
fi
