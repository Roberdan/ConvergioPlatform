#!/usr/bin/env bash
# Start the ConvergioPlatform dashboard
set -euo pipefail

DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$DIR"

# Load .env if exists
[ -f .env ] && export $(grep -v '^#' .env | xargs)

# Default DB path
export DASHBOARD_DB="${DASHBOARD_DB:-$HOME/.claude/data/dashboard.db}"

# Check DB exists
if [ ! -f "$DASHBOARD_DB" ]; then
  echo "ERROR: dashboard.db not found at $DASHBOARD_DB"
  echo "Set DASHBOARD_DB env var to the correct path"
  exit 1
fi

echo "Starting ConvergioPlatform Dashboard..."
echo "  DB: $DASHBOARD_DB"
echo "  Port: ${PORT:-8788}"

python3 api_server.py
