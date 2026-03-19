#!/usr/bin/env bash
# copilot-bridge.sh — Register/unregister Copilot agents in IPC daemon
# Thin wrapper over agent-bridge.sh with Copilot defaults
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source agent-bridge.sh library functions
# shellcheck source=agent-bridge.sh
source "$SCRIPT_DIR/agent-bridge.sh"

# Defaults for Copilot
DEFAULT_NAME="copilot"
DEFAULT_TYPE="copilot"

main() {
  local action=""
  local name="$DEFAULT_NAME"
  local task=""

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --register)   action="register" ;;
      --unregister) action="unregister" ;;
      --heartbeat)  action="heartbeat" ;;
      --name)       shift; name="$1" ;;
      --task)       shift; task="$1" ;;
      *) echo "Unknown: $1" >&2; exit 1 ;;
    esac
    shift
  done

  case "$action" in
    register)   agent_register "$name" "$DEFAULT_TYPE" ;;
    unregister) agent_unregister "$name" ;;
    heartbeat)  agent_heartbeat "$name" "${task:-idle}" ;;
    *)          echo "Usage: copilot-bridge.sh --register|--unregister|--heartbeat [--name N] [--task T]" >&2; exit 1 ;;
  esac
}

main "$@"
