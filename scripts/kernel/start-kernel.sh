#!/usr/bin/env bash
# start-kernel.sh — Wrapper for launchd: sources env, starts daemon.
# launchd cannot source env files directly, so this script does it.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
DAEMON_BIN="${REPO_DIR}/daemon/target/release/convergio-platform-daemon"
ENV_FILE="${HOME}/.convergio/env"
VENV="${HOME}/convergio-env/bin/activate"

# Source and export secrets (source alone doesn't export to child process)
if [[ -f "$ENV_FILE" ]]; then
  set -a  # auto-export all variables
  source "$ENV_FILE"
  set +a
fi

# Normalize legacy Telegram variable names so older node env files still boot cleanly.
[[ -z "${CONVERGIO_TELEGRAM_TOKEN:-}" && -n "${TELEGRAM_BOT_TOKEN:-}" ]] \
  && export CONVERGIO_TELEGRAM_TOKEN="${TELEGRAM_BOT_TOKEN}"
[[ -z "${CONVERGIO_TELEGRAM_CHAT_ID:-}" && -n "${TELEGRAM_CHAT_ID:-}" ]] \
  && export CONVERGIO_TELEGRAM_CHAT_ID="${TELEGRAM_CHAT_ID}"

# Source Python venv (for mlx_lm)
[[ -f "$VENV" ]] && source "$VENV"

# Ensure Claude CLI and other user binaries are in PATH
export PATH="${HOME}/.local/bin:${HOME}/convergio-env/bin:/opt/homebrew/bin:${PATH}"

# Prevent sleep — kernel node must stay awake 24/7
caffeinate -s &

# Create unified Convergio tmux session if it doesn't exist.
# All delegations land here as windows. User monitors with: tmux attach -t Convergio
tmux has-session -t Convergio 2>/dev/null || \
  tmux new-session -d -s Convergio -n kernel -c "${REPO_DIR}"

# Start daemon
exec "$DAEMON_BIN" serve --dev-mode
