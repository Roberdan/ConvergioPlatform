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
