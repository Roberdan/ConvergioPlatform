#!/usr/bin/env bash
# Start the ConvergioPlatform dashboard via Rust daemon
set -euo pipefail

DIR="$(cd "$(dirname "$0")" && pwd)"
PLATFORM_DIR="$(cd "$DIR/.." && pwd)"
DAEMON="$PLATFORM_DIR/daemon/target/release/convergio-platform-daemon"

export DASHBOARD_DB="${DASHBOARD_DB:-$PLATFORM_DIR/data/dashboard.db}"

if [ ! -f "$DASHBOARD_DB" ]; then
  echo "ERROR: dashboard.db not found at $DASHBOARD_DB"
  exit 1
fi

if [ ! -f "$DAEMON" ]; then
  echo "Building daemon (first time)..."
  cd "$PLATFORM_DIR/daemon" && cargo build --release
fi

PORT="${PORT:-8420}"
echo "Starting Convergio Control Room on http://localhost:$PORT"
exec "$DAEMON" serve --static-dir "$DIR" --bind "0.0.0.0:$PORT"
