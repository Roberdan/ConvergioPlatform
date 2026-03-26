#!/bin/bash
# sqlite3-safe.sh — Drop-in sqlite3 wrapper with busy_timeout + WAL mode
# Why: 58 scripts use sqlite3 directly without busy_timeout, causing DB locks
#      when daemon and scripts run concurrently. This wrapper adds safety.
# Usage: source this file, then use `_db` instead of `sqlite3`
#   source scripts/platform/sqlite3-safe.sh
#   _db "$DB_PATH" "SELECT * FROM plans;"
# Or symlink this as sqlite3 in PATH to intercept all calls.
set -euo pipefail

_db() {
  local db="${1:?db path required}"
  shift
  sqlite3 "$db" "PRAGMA busy_timeout=5000; PRAGMA journal_mode=WAL;" ".timeout 5000" "$@"
}

# If called directly (not sourced), act as sqlite3 wrapper
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  if [[ $# -lt 1 ]]; then
    echo "Usage: sqlite3-safe.sh <db_path> [sql...]" >&2
    exit 1
  fi
  DB="$1"; shift
  sqlite3 "$DB" "PRAGMA busy_timeout=5000; PRAGMA journal_mode=WAL;" "$@"
fi
