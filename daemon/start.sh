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
