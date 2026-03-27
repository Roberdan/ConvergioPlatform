#!/usr/bin/env bash
# install-db-sync.sh — Install the com.convergio.sync-db launchd agent on this Mac.
# Configures source and target SSH nodes, then loads the 30-min periodic DB sync job.
# Usage: install-db-sync.sh [--source <ssh-alias>] [--target <ssh-alias>] [--db-path <path>]
set -euo pipefail

readonly SCRIPT_NAME="install-db-sync.sh"
readonly REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly PLIST_SRC="${REPO_ROOT}/scripts/kernel/com.convergio.sync-db.plist"
readonly PLIST_DEST="${HOME}/Library/LaunchAgents/com.convergio.sync-db.plist"
readonly LOG_FILE="${HOME}/Library/Logs/convergio-sync-db.log"
readonly LABEL="com.convergio.sync-db"

# Defaults
SOURCE_SSH="${CONVERGIO_SOURCE_SSH:-localhost}"
TARGET_SSH="${CONVERGIO_TARGET_SSH:-macProM1}"
DB_PATH="${DASHBOARD_DB:-${HOME}/GitHub/ConvergioPlatform/data/dashboard.db}"

log() { echo "[${SCRIPT_NAME}] $*"; }
err() { echo "[${SCRIPT_NAME}] ERROR: $*" >&2; }

usage() {
  echo "Usage: ${SCRIPT_NAME} [--source <ssh-alias>] [--target <ssh-alias>] [--db-path <path>]" >&2
  echo "  --source    SSH alias/user@host of the source node (default: localhost)" >&2
  echo "  --target    SSH alias/user@host of the target node (default: macProM1)" >&2
  echo "  --db-path   Absolute path to dashboard.db on source (default: \$DASHBOARD_DB or ~/GitHub/ConvergioPlatform/data/dashboard.db)" >&2
  echo "" >&2
  echo "  Env overrides: CONVERGIO_SOURCE_SSH, CONVERGIO_TARGET_SSH, DASHBOARD_DB" >&2
  exit 1
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --source)
        SOURCE_SSH="$2"; shift 2 ;;
      --target)
        TARGET_SSH="$2"; shift 2 ;;
      --db-path)
        DB_PATH="$2"; shift 2 ;;
      --help|-h)
        usage ;;
      *)
        err "Unknown argument: $1"
        usage ;;
    esac
  done
}

# Validate SSH connectivity to both nodes
validate_ssh() {
  local node="$1"
  if [[ "${node}" == "localhost" ]]; then
    return 0
  fi
  log "Checking SSH connectivity to ${node}..."
  if ! ssh -o ConnectTimeout=5 -o BatchMode=yes "${node}" true 2>/dev/null; then
    err "Cannot SSH to ${node}. Ensure SSH keys are configured and the node is reachable."
    return 1
  fi
  log "  SSH to ${node}: OK"
}

# Install plist with substituted values
install_plist() {
  local tmp_plist
  tmp_plist="$(mktemp /tmp/com.convergio.sync-db.XXXXXX.plist)"
  trap 'rm -f "${tmp_plist}"' RETURN

  sed \
    -e "s|__HOME__|${HOME}|g" \
    -e "s|__REPO_ROOT__|${REPO_ROOT}|g" \
    -e "s|__SOURCE_SSH__|${SOURCE_SSH}|g" \
    -e "s|__TARGET_SSH__|${TARGET_SSH}|g" \
    -e "s|__DASHBOARD_DB__|${DB_PATH}|g" \
    "${PLIST_SRC}" > "${tmp_plist}"

  mkdir -p "${HOME}/Library/LaunchAgents"
  cp "${tmp_plist}" "${PLIST_DEST}"
  log "Plist installed: ${PLIST_DEST}"
}

# Unload existing agent if already loaded
unload_existing() {
  if launchctl list 2>/dev/null | grep -q "${LABEL}"; then
    log "Unloading existing ${LABEL} agent..."
    launchctl unload "${PLIST_DEST}" 2>/dev/null || true
    sleep 1
  fi
}

# Make sync script executable
ensure_executable() {
  local sync_script="${REPO_ROOT}/scripts/kernel/sync-db.sh"
  if [[ ! -x "${sync_script}" ]]; then
    chmod +x "${sync_script}"
    log "Made executable: ${sync_script}"
  fi
}

print_summary() {
  log "=== DB Sync agent installed ==="
  log "  Label    : ${LABEL}"
  log "  Source   : ${SOURCE_SSH}"
  log "  Target   : ${TARGET_SSH}"
  log "  DB path  : ${DB_PATH}"
  log "  Interval : every 30 minutes"
  log "  Log      : ${LOG_FILE}"
  log ""
  log "Useful commands:"
  log "  View log   : tail -f ${LOG_FILE}"
  log "  Run now    : launchctl start ${LABEL}"
  log "  Uninstall  : launchctl unload ${PLIST_DEST} && rm ${PLIST_DEST}"
}

main() {
  parse_args "$@"

  log "=== Convergio DB Sync launchd install ==="
  log "Repo: ${REPO_ROOT}"
  log "Source SSH : ${SOURCE_SSH}"
  log "Target SSH : ${TARGET_SSH}"
  log "DB path    : ${DB_PATH}"

  validate_ssh "${SOURCE_SSH}"
  validate_ssh "${TARGET_SSH}"

  ensure_executable
  unload_existing
  install_plist

  launchctl load "${PLIST_DEST}"
  log "Agent loaded: ${LABEL}"

  print_summary
}

main "$@"
