#!/usr/bin/env bash
# uninstall-launchd.sh — Unload and remove the convergio kernel launchd agent.
set -euo pipefail

PLIST_DEST="${HOME}/Library/LaunchAgents/com.convergio.kernel.plist"
LABEL="com.convergio.kernel"

trap cleanup EXIT

cleanup() {
  local exit_code=$?
  if [[ $exit_code -ne 0 ]]; then
    echo "ERROR: uninstall encountered an error (exit $exit_code)." >&2
  fi
}

log() { echo "[uninstall-launchd] $*"; }

# Unload agent if currently loaded
unload_agent() {
  if launchctl list | grep -q "${LABEL}" 2>/dev/null; then
    log "Unloading agent: ${LABEL}"
    launchctl unload "${PLIST_DEST}"
    sleep 1
    if launchctl list | grep -q "${LABEL}" 2>/dev/null; then
      echo "WARNING: agent still appears in launchctl list after unload." >&2
    else
      log "Agent unloaded successfully."
    fi
  else
    log "Agent not loaded — skipping unload."
  fi
}

# Remove plist file
remove_plist() {
  if [[ -f "${PLIST_DEST}" ]]; then
    rm -f "${PLIST_DEST}"
    log "Plist removed: ${PLIST_DEST}"
  else
    log "Plist not found at ${PLIST_DEST} — already removed."
  fi
}

# Confirm removal
confirm_removal() {
  if [[ -f "${PLIST_DEST}" ]]; then
    echo "ERROR: plist still present after removal: ${PLIST_DEST}" >&2
    return 1
  fi
  if launchctl list | grep -q "${LABEL}" 2>/dev/null; then
    echo "ERROR: agent still registered in launchctl after uninstall." >&2
    return 1
  fi
  log "Removal confirmed: plist gone, agent not registered."
}

main() {
  log "=== Convergio Kernel launchd uninstall ==="
  unload_agent
  remove_plist
  confirm_removal
  log "=== Uninstall complete ==="
}

main "$@"
