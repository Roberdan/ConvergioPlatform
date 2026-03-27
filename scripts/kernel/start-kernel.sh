#!/usr/bin/env bash
# start-kernel.sh — Wrapper for launchd: sources env, starts daemon.
# launchd cannot source env files directly, so this script does it.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
DAEMON_BIN="${REPO_DIR}/daemon/target/release/convergio-platform-daemon"
ENV_FILE="${HOME}/.convergio/env"
VENV="${HOME}/convergio-env/bin/activate"

# Source secrets
[[ -f "$ENV_FILE" ]] && source "$ENV_FILE"

# Source Python venv (for mlx_lm)
[[ -f "$VENV" ]] && source "$VENV"

# Start daemon
exec "$DAEMON_BIN" serve --dev-mode
