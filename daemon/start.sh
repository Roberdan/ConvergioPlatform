#!/usr/bin/env bash
# Start the ConvergioPlatform daemon (or TUI dashboard)
# Usage:
#   ./daemon/start.sh              — start daemon (default: serve)
#   ./daemon/start.sh tui          — launch TUI dashboard
#   ./daemon/start.sh tui --api-url http://host:8420
set -euo pipefail

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
