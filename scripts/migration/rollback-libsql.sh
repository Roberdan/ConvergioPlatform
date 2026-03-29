#!/usr/bin/env bash
set -euo pipefail

# Rollback the libsql timestamp-based sync migration.
#
# Restores dashboard.db from the backup created before migration.
# Usage: ./scripts/migration/rollback-libsql.sh [backup_path]
#
# If no backup_path is given, looks for data/dashboard.db.bak in
# the project root (derived from DASHBOARD_DB or $HOME/.claude/data/).

trap 'echo "[rollback] ERROR on line $LINENO" >&2' ERR

resolve_db_path() {
    if [[ -n "${DASHBOARD_DB:-}" ]]; then
        echo "$DASHBOARD_DB"
    else
        echo "${HOME}/.claude/data/dashboard.db"
    fi
}

main() {
    local db_path
    db_path="$(resolve_db_path)"

    local backup_path="${1:-${db_path}.bak}"

    if [[ ! -f "$backup_path" ]]; then
        echo "[rollback] ERROR: backup not found at $backup_path" >&2
        echo "[rollback] Provide path as argument or ensure .bak exists" >&2
        exit 1
    fi

    if [[ ! -f "$db_path" ]]; then
        echo "[rollback] WARNING: no DB at $db_path — restoring from backup" >&2
    fi

    echo "[rollback] Restoring $db_path from $backup_path"

    # Stop daemon if running to avoid WAL contention
    local pid
    pid="$(pgrep -f 'convergio.*daemon' || true)"
    if [[ -n "$pid" ]]; then
        echo "[rollback] Stopping daemon (pid $pid) before restore"
        kill "$pid" 2>/dev/null || true
        sleep 1
    fi

    # Remove WAL/SHM files — stale WAL from old DB would corrupt restore
    rm -f "${db_path}-wal" "${db_path}-shm"

    cp -f "$backup_path" "$db_path"
    echo "[rollback] Restored successfully"

    # Verify the restored DB opens
    if command -v sqlite3 &>/dev/null; then
        local count
        count="$(sqlite3 "$db_path" "SELECT COUNT(*) FROM sqlite_master;" 2>/dev/null || echo "FAIL")"
        if [[ "$count" == "FAIL" ]]; then
            echo "[rollback] WARNING: restored DB failed integrity check" >&2
            exit 1
        fi
        echo "[rollback] Verified: $count tables/indexes in restored DB"
    else
        echo "[rollback] sqlite3 not available — skipping verification"
    fi

    echo "[rollback] Done. Restart daemon with: ./daemon/start.sh"
}

main "$@"
