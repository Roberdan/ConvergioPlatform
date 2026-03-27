#!/usr/bin/env bash
# install-launchd.sh — Install convergio kernel as a launchd agent on macOS (M1 Pro).
# Builds the daemon if needed, installs the plist, loads it, and waits for health.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PLIST_SRC="${REPO_ROOT}/scripts/kernel/com.convergio.kernel.plist"
PLIST_DEST="${HOME}/Library/LaunchAgents/com.convergio.kernel.plist"
BINARY="${REPO_ROOT}/daemon/target/release/convergio-platform-daemon"
KERNEL_STATUS_URL="http://localhost:8420/api/kernel/status"
LOG_FILE="${HOME}/Library/Logs/convergio-kernel.log"
HEALTH_RETRIES=10
HEALTH_SLEEP=2

trap cleanup EXIT

cleanup() {
  local exit_code=$?
  if [[ $exit_code -ne 0 ]]; then
    echo "ERROR: install failed (exit $exit_code). Check ${LOG_FILE} for details." >&2
  fi
}

log() { echo "[install-launchd] $*"; }

# Build daemon if binary is missing or sources are newer
build_daemon() {
  log "Building daemon with kernel feature..."
  (cd "${REPO_ROOT}/daemon" && cargo build --features kernel --release)
  log "Build complete: ${BINARY}"
}

# Inject env vars into a temp plist copy, then install
install_plist() {
  local tmp_plist
  tmp_plist="$(mktemp /tmp/com.convergio.kernel.XXXXXX.plist)"
  trap 'rm -f "${tmp_plist}"' RETURN

  local token="${CONVERGIO_TELEGRAM_TOKEN:-}"
  local chat_id="${CONVERGIO_TELEGRAM_CHAT_ID:-}"

  sed \
    -e "s|__CONVERGIO_TELEGRAM_TOKEN__|${token}|g" \
    -e "s|__CONVERGIO_TELEGRAM_CHAT_ID__|${chat_id}|g" \
    "${PLIST_SRC}" > "${tmp_plist}"

  mkdir -p "${HOME}/Library/LaunchAgents"
  cp "${tmp_plist}" "${PLIST_DEST}"
  log "Plist installed: ${PLIST_DEST}"
}

# Unload existing agent if already loaded (ignore errors)
unload_existing() {
  if launchctl list | grep -q "com.convergio.kernel" 2>/dev/null; then
    log "Unloading existing agent..."
    launchctl unload "${PLIST_DEST}" 2>/dev/null || true
    sleep 1
  fi
}

# Wait for /api/kernel/status to return healthy
wait_for_healthy() {
  log "Waiting for kernel to become healthy (up to $((HEALTH_RETRIES * HEALTH_SLEEP))s)..."
  local attempt=1
  while [[ $attempt -le $HEALTH_RETRIES ]]; do
    local http_code
    http_code="$(curl -s -o /dev/null -w "%{http_code}" "${KERNEL_STATUS_URL}" 2>/dev/null || echo "000")"
    if [[ "${http_code}" == "200" ]]; then
      log "Kernel healthy (HTTP 200) after ${attempt} attempt(s)."
      return 0
    fi
    log "  Attempt ${attempt}/${HEALTH_RETRIES}: status=${http_code}, retrying in ${HEALTH_SLEEP}s..."
    sleep "${HEALTH_SLEEP}"
    (( attempt++ ))
  done
  echo "ERROR: kernel did not become healthy after ${HEALTH_RETRIES} attempts." >&2
  echo "       Tail log: tail -50 ${LOG_FILE}" >&2
  return 1
}

# Send Telegram notification if token configured
notify_telegram() {
  local token="${CONVERGIO_TELEGRAM_TOKEN:-}"
  local chat_id="${CONVERGIO_TELEGRAM_CHAT_ID:-}"
  if [[ -z "${token}" || -z "${chat_id}" ]]; then
    log "Telegram not configured — skipping notification."
    return 0
  fi
  local msg="Kernel avviato su $(hostname) ($(date '+%Y-%m-%d %H:%M:%S'))"
  curl -s -X POST \
    "https://api.telegram.org/bot${token}/sendMessage" \
    -d "chat_id=${chat_id}" \
    -d "text=${msg}" \
    -d "parse_mode=Markdown" > /dev/null || log "WARNING: Telegram notification failed (non-fatal)."
  log "Telegram notification sent."
}

main() {
  log "=== Convergio Kernel launchd install ==="
  log "Repo: ${REPO_ROOT}"

  if [[ ! -f "${BINARY}" ]]; then
    build_daemon
  else
    log "Binary already built: ${BINARY}"
  fi

  unload_existing
  install_plist
  launchctl load "${PLIST_DEST}"
  log "Agent loaded: com.convergio.kernel"

  wait_for_healthy
  notify_telegram
  log "=== Install complete ==="
}

main "$@"
