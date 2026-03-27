#!/usr/bin/env bash
# sync-db.sh — Safe DB rsync between Convergio nodes.
# Usage: sync-db.sh <source-ssh> <target-ssh>
#   source-ssh: user@host or alias (e.g. "macProM1")
#   target-ssh: user@host or alias — receives the DB
# Steps: stop daemon → WAL checkpoint → rsync (resolve symlinks) → integrity check → restart → health wait
set -euo pipefail

readonly SCRIPT_NAME="sync-db.sh"
readonly DB_REL_PATH="GitHub/ConvergioPlatform/data/dashboard.db"
readonly DB_DEFAULT="\${HOME}/${DB_REL_PATH}"
readonly HEALTH_RETRIES=10
readonly HEALTH_SLEEP=2
readonly WAL_WAIT=3
readonly DAEMON_PROCESS="convergio-platform-daemon"
readonly HEALTH_URL="http://localhost:8420/api/health"

log() { echo "[${SCRIPT_NAME}] $*"; }
err() { echo "[${SCRIPT_NAME}] ERROR: $*" >&2; }

usage() {
  echo "Usage: ${SCRIPT_NAME} <source-ssh> <target-ssh>" >&2
  echo "  source-ssh: SSH alias or user@host of the DB source node" >&2
  echo "  target-ssh: SSH alias or user@host of the DB target node" >&2
  echo "  DB path resolved from DASHBOARD_DB env var or ~/GitHub/ConvergioPlatform/data/dashboard.db" >&2
  exit 1
}

# Resolve the real path of the DB on a remote node (follow symlinks)
resolve_remote_db() {
  local node="$1"
  local default_path="$2"
  # ssh: resolve DASHBOARD_DB env var first; fall back to default; follow symlinks via readlink -f
  ssh "${node}" "bash -lc '
    raw=\"\${DASHBOARD_DB:-${default_path}}\"
    real=\"\$(readlink -f \"\$raw\" 2>/dev/null || echo \"\$raw\")\"
    echo \"\$real\"
  '"
}

# Step 1: Stop daemon on target
stop_target_daemon() {
  local target="$1"
  local db_path="$2"
  log "[$target] Stopping ${DAEMON_PROCESS}..."
  ssh "${target}" "pkill -f '${DAEMON_PROCESS}' 2>/dev/null && echo 'daemon stopped' || echo 'daemon was not running'"
  log "[$target] Waiting ${WAL_WAIT}s for WAL checkpoint to complete..."
  sleep "${WAL_WAIT}"
  # Force WAL checkpoint on source before copying (source may be this Mac or remote)
  log "[source] WAL checkpoint on source DB: ${db_path}"
  sqlite3 "${db_path}" "PRAGMA wal_checkpoint(FULL);" 2>/dev/null || true
}

# Step 3: rsync DB + WAL + SHM from source to target (resolve symlinks with -L)
rsync_db() {
  local source_node="$1"
  local target_node="$2"
  local source_db="$3"
  local target_db="$4"
  local source_dir
  source_dir="$(dirname "${source_db}")"
  local target_dir
  target_dir="$(dirname "${target_db}")"
  local db_base
  db_base="$(basename "${source_db}")"

  log "Rsyncing DB from ${source_node}:${source_db} → ${target_node}:${target_db}"

  # Ensure target directory exists
  ssh "${target_node}" "mkdir -p '${target_dir}'"

  # rsync with -L to dereference symlinks, -a for archive, -z for compression
  # Transfer all three SQLite WAL-mode files atomically
  rsync -aLz --progress \
    -e "ssh" \
    "${source_node}:${source_dir}/${db_base}" \
    "${source_node}:${source_dir}/${db_base}-wal" \
    "${source_node}:${source_dir}/${db_base}-shm" \
    "${target_node}:${target_dir}/" 2>/dev/null || {
      # WAL and SHM may not exist if DB is in WAL+checkpoint state — that is fine
      rsync -aLz --progress \
        -e "ssh" \
        "${source_node}:${source_dir}/${db_base}" \
        "${target_node}:${target_dir}/"
    }
  log "Rsync complete."
}

# Step 4: Integrity check on target
verify_integrity() {
  local target="$1"
  local db_path="$2"
  log "[$target] Running PRAGMA integrity_check on ${db_path}..."
  local result
  result="$(ssh "${target}" "sqlite3 '${db_path}' 'PRAGMA integrity_check;'")"
  if [[ "${result}" == "ok" ]]; then
    log "[$target] Integrity check PASSED."
  else
    err "[$target] Integrity check FAILED: ${result}"
    return 1
  fi
}

# Step 5: Restart daemon on target (read Telegram token from keychain)
restart_target_daemon() {
  local target="$1"
  log "[$target] Restarting ${DAEMON_PROCESS} via launchd..."
  ssh "${target}" "bash -lc '
    plist=\"\${HOME}/Library/LaunchAgents/com.convergio.kernel.plist\"
    if [[ -f \"\$plist\" ]]; then
      launchctl unload \"\$plist\" 2>/dev/null || true
      # Read Telegram token from keychain
      token=\"\$(security find-generic-password -s convergio-telegram-token -a convergio -w 2>/dev/null || echo \"\")\"
      chat_id=\"\$(security find-generic-password -s convergio-telegram-chat-id -a convergio -w 2>/dev/null || echo \"\")\"
      if [[ -n \"\$token\" ]]; then
        export CONVERGIO_TELEGRAM_TOKEN=\"\$token\"
        export CONVERGIO_TELEGRAM_CHAT_ID=\"\$chat_id\"
      fi
      launchctl load \"\$plist\"
      echo \"launchd agent reloaded\"
    else
      echo \"WARNING: launchd plist not found at \$plist — attempting direct start\" >&2
      nohup \"\${HOME}/GitHub/ConvergioPlatform/daemon/target/release/${DAEMON_PROCESS}\" serve --kernel >/tmp/convergio-daemon.log 2>&1 &
      echo \"daemon started directly (pid \$!)\"
    fi
  '"
}

# Step 6: Wait for /api/health on target
wait_for_health() {
  local target="$1"
  log "[$target] Waiting for ${HEALTH_URL} (${HEALTH_RETRIES}x${HEALTH_SLEEP}s)..."
  local attempt=1
  while [[ $attempt -le ${HEALTH_RETRIES} ]]; do
    local http_code
    http_code="$(ssh "${target}" "curl -s -o /dev/null -w '%{http_code}' '${HEALTH_URL}' 2>/dev/null || echo 000")"
    if [[ "${http_code}" == "200" ]]; then
      log "[$target] Health check PASSED (HTTP 200) on attempt ${attempt}."
      return 0
    fi
    log "[$target] Attempt ${attempt}/${HEALTH_RETRIES}: status=${http_code}, retrying in ${HEALTH_SLEEP}s..."
    sleep "${HEALTH_SLEEP}"
    (( attempt++ ))
  done
  err "[$target] Daemon did not become healthy after ${HEALTH_RETRIES} attempts."
  return 1
}

main() {
  if [[ $# -ne 2 ]]; then
    usage
  fi

  local source_node="$1"
  local target_node="$2"

  log "=== Convergio DB Sync: ${source_node} → ${target_node} ==="

  # Resolve actual DB paths on both nodes
  local source_db
  source_db="$(resolve_remote_db "${source_node}" "\${HOME}/${DB_REL_PATH}")"
  log "Source DB (resolved): ${source_db}"

  local target_db
  target_db="$(resolve_remote_db "${target_node}" "\${HOME}/${DB_REL_PATH}")"
  log "Target DB (resolved): ${target_db}"

  # Execute sync pipeline
  stop_target_daemon "${target_node}" "${source_db}"
  rsync_db "${source_node}" "${target_node}" "${source_db}" "${target_db}"
  verify_integrity "${target_node}" "${target_db}"
  restart_target_daemon "${target_node}"
  wait_for_health "${target_node}"

  log "=== DB Sync complete: ${source_node} → ${target_node} ==="
}

main "$@"
