#!/usr/bin/env bash
# Migration: add context_path, paused_at, paused_context to execution_runs
# and expand CHECK constraint to include 'paused' status.
# Idempotent: skips if context_path column already present.
set -euo pipefail

# --- Configuration -----------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
DB="${DASHBOARD_DB:-${REPO_ROOT}/data/dashboard.db}"
RUNS_DIR="${REPO_ROOT}/data/runs"

# --- Cleanup -----------------------------------------------------------------

cleanup() {
  local exit_code=$?
  if [[ ${exit_code} -ne 0 ]]; then
    echo "[convergio-db-migrate] ERROR: migration failed (exit ${exit_code})" >&2
    echo "[convergio-db-migrate] The database was NOT modified (transaction rolled back)." >&2
  fi
}
trap cleanup EXIT

# --- Guard: DB must exist ----------------------------------------------------

if [[ ! -f "${DB}" ]]; then
  echo "[convergio-db-migrate] ERROR: database not found: ${DB}" >&2
  exit 1
fi

# --- Idempotency check -------------------------------------------------------

existing_col=$(sqlite3 "${DB}" \
  "SELECT COUNT(*) FROM pragma_table_info('execution_runs') WHERE name='context_path';" 2>&1)

if [[ "${existing_col}" == "1" ]]; then
  echo "[convergio-db-migrate] INFO: context_path already exists — skipping migration."
  # Still ensure data/runs/ directory exists
  mkdir -p "${RUNS_DIR}"
  echo "[convergio-db-migrate] INFO: ${RUNS_DIR} ready."
  exit 0
fi

echo "[convergio-db-migrate] Starting migration of execution_runs ..."

# --- Migration (single transaction) -----------------------------------------

sqlite3 "${DB}" <<'SQL'
BEGIN EXCLUSIVE;

-- Step 1: Create replacement table with new columns and updated CHECK.
CREATE TABLE execution_runs_new (
  id               INTEGER PRIMARY KEY AUTOINCREMENT,
  goal             TEXT    NOT NULL,
  team             TEXT    DEFAULT '[]',
  status           TEXT    DEFAULT 'running'
                           CHECK(status IN (
                             'running','completed','failed','cancelled','paused'
                           )),
  result           TEXT,
  cost_usd         REAL    DEFAULT 0,
  agents_used      INTEGER DEFAULT 0,
  plan_id          INTEGER,
  started_at       TEXT    DEFAULT (datetime('now')),
  completed_at     TEXT,
  duration_minutes REAL,
  context_path     TEXT,
  paused_at        TEXT,
  paused_context   TEXT
);

-- Step 2: Copy all existing rows; new columns default to NULL.
INSERT INTO execution_runs_new (
  id, goal, team, status, result,
  cost_usd, agents_used, plan_id,
  started_at, completed_at, duration_minutes
)
SELECT
  id, goal, team, status, result,
  cost_usd, agents_used, plan_id,
  started_at, completed_at, duration_minutes
FROM execution_runs;

-- Step 3: Drop old table.
DROP TABLE execution_runs;

-- Step 4: Rename new table into place.
ALTER TABLE execution_runs_new RENAME TO execution_runs;

-- Step 5: Recreate indexes.
CREATE INDEX IF NOT EXISTS idx_execution_runs_status
  ON execution_runs (status);

CREATE INDEX IF NOT EXISTS idx_execution_runs_plan_id
  ON execution_runs (plan_id);

CREATE INDEX IF NOT EXISTS idx_execution_runs_started_at
  ON execution_runs (started_at);

COMMIT;
SQL

echo "[convergio-db-migrate] Migration completed successfully."

# --- data/runs directory -----------------------------------------------------

mkdir -p "${RUNS_DIR}"
echo "[convergio-db-migrate] INFO: ${RUNS_DIR} ready."

# --- Quick smoke test --------------------------------------------------------

col_check=$(sqlite3 "${DB}" \
  "SELECT COUNT(*) FROM pragma_table_info('execution_runs') WHERE name='context_path';")

if [[ "${col_check}" != "1" ]]; then
  echo "[convergio-db-migrate] ERROR: post-migration column check failed." >&2
  exit 1
fi

status_check=$(sqlite3 "${DB}" \
  "SELECT sql FROM sqlite_master WHERE type='table' AND name='execution_runs';" | \
  grep -c "paused" || true)

if [[ "${status_check}" -lt 1 ]]; then
  echo "[convergio-db-migrate] ERROR: post-migration CHECK constraint missing 'paused'." >&2
  exit 1
fi

echo "[convergio-db-migrate] Smoke test PASSED."
