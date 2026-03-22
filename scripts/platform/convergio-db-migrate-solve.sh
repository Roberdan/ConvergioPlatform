#!/usr/bin/env bash
# Migration for solve_sessions table — stores /solve command session audit trail.
# Commands: migrate | rollback | save <json_file>
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
DB="${DASHBOARD_DB:-${REPO_ROOT}/data/dashboard.db}"

# --- Cleanup -----------------------------------------------------------------

cleanup() {
  local exit_code=$?
  if [[ ${exit_code} -ne 0 ]]; then
    echo "[convergio-db-migrate-solve] ERROR: command failed (exit ${exit_code})" >&2
  fi
}
trap cleanup EXIT

# --- Guard: DB must exist (except for rollback — table may not exist) ---------

require_db() {
  if [[ ! -f "${DB}" ]]; then
    echo "[convergio-db-migrate-solve] ERROR: database not found: ${DB}" >&2
    exit 1
  fi
}

# --- Usage -------------------------------------------------------------------

usage() {
  echo "Usage: $(basename "$0") <command> [args]" >&2
  echo "" >&2
  echo "Commands:" >&2
  echo "  migrate              Create solve_sessions table (idempotent)" >&2
  echo "  rollback             Drop solve_sessions table" >&2
  echo "  save <json_file>     Insert session from JSON file into solve_sessions" >&2
  exit 1
}

# --- Commands ----------------------------------------------------------------

cmd_migrate() {
  require_db
  echo "[convergio-db-migrate-solve] Creating solve_sessions table ..."
  sqlite3 "${DB}" <<'SQL'
CREATE TABLE IF NOT EXISTS solve_sessions (
  id                   INTEGER PRIMARY KEY AUTOINCREMENT,
  timestamp            TEXT    NOT NULL DEFAULT (datetime('now')),
  user_input           TEXT    NOT NULL,
  constitution_check   TEXT,
  triage_level         TEXT    CHECK(triage_level IN ('light', 'standard', 'full')),
  clarification_rounds TEXT,
  research_findings    TEXT,
  problem_statement    TEXT,
  requirements_json    TEXT,
  acceptance_invariants TEXT,
  routed_to            TEXT,
  decision_audit       TEXT,
  plan_id              INTEGER REFERENCES plans(id)
);

CREATE INDEX IF NOT EXISTS idx_solve_sessions_timestamp
  ON solve_sessions (timestamp);

CREATE INDEX IF NOT EXISTS idx_solve_sessions_triage_level
  ON solve_sessions (triage_level);

CREATE INDEX IF NOT EXISTS idx_solve_sessions_plan_id
  ON solve_sessions (plan_id);
SQL
  echo "[convergio-db-migrate-solve] solve_sessions table ready."
}

cmd_rollback() {
  require_db
  echo "[convergio-db-migrate-solve] Dropping solve_sessions table ..."
  sqlite3 "${DB}" "DROP TABLE IF EXISTS solve_sessions;"
  echo "[convergio-db-migrate-solve] solve_sessions table dropped."
}

cmd_save() {
  local json_file="${1:-}"
  if [[ -z "${json_file}" ]]; then
    echo "[convergio-db-migrate-solve] ERROR: save requires a JSON file argument." >&2
    usage
  fi
  if [[ ! -f "${json_file}" ]]; then
    echo "[convergio-db-migrate-solve] ERROR: JSON file not found: ${json_file}" >&2
    exit 1
  fi

  require_db

  # Ensure table exists before inserting
  local table_exists
  table_exists=$(sqlite3 "${DB}" \
    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='solve_sessions';" 2>/dev/null || echo "0")
  if [[ "${table_exists}" != "1" ]]; then
    echo "[convergio-db-migrate-solve] INFO: table not found — running migrate first ..."
    cmd_migrate
  fi

  # Extract fields from JSON using python3 (available on macOS/Linux without extra deps)
  # Each field is extracted safely; missing keys default to empty string / null.
  local user_input constitution_check triage_level clarification_rounds
  local research_findings problem_statement requirements_json acceptance_invariants
  local routed_to decision_audit plan_id

  user_input=$(python3 -c "
import json, sys
d = json.load(open('${json_file}'))
print(d.get('request') or d.get('user_input') or '')
" 2>/dev/null || echo "")

  constitution_check=$(python3 -c "
import json, sys
d = json.load(open('${json_file}'))
v = d.get('constitution_check')
print(v if v is not None else '')
" 2>/dev/null || echo "")

  triage_level=$(python3 -c "
import json, sys
d = json.load(open('${json_file}'))
print(d.get('scale') or d.get('triage_level') or '')
" 2>/dev/null || echo "")

  clarification_rounds=$(python3 -c "
import json, sys
d = json.load(open('${json_file}'))
v = d.get('clarification_rounds')
print(json.dumps(v) if v is not None else '')
" 2>/dev/null || echo "")

  research_findings=$(python3 -c "
import json, sys
d = json.load(open('${json_file}'))
v = d.get('research_findings')
print(v if v is not None else '')
" 2>/dev/null || echo "")

  problem_statement=$(python3 -c "
import json, sys
d = json.load(open('${json_file}'))
v = d.get('problem_statement')
print(v if v is not None else '')
" 2>/dev/null || echo "")

  requirements_json=$(python3 -c "
import json, sys
d = json.load(open('${json_file}'))
v = d.get('f_xx') or d.get('requirements_json')
print(json.dumps(v) if v is not None else '')
" 2>/dev/null || echo "")

  acceptance_invariants=$(python3 -c "
import json, sys
d = json.load(open('${json_file}'))
v = d.get('acceptance_invariants')
print(json.dumps(v) if v is not None else '')
" 2>/dev/null || echo "")

  routed_to=$(python3 -c "
import json, sys
d = json.load(open('${json_file}'))
v = d.get('routed_to')
print(v if v is not None else '')
" 2>/dev/null || echo "")

  decision_audit=$(python3 -c "
import json, sys
d = json.load(open('${json_file}'))
v = d.get('reformulation') or d.get('decision_audit')
print(json.dumps(v) if v is not None else '')
" 2>/dev/null || echo "")

  plan_id=$(python3 -c "
import json, sys
d = json.load(open('${json_file}'))
v = d.get('plan_id')
print(v if v is not None else '')
" 2>/dev/null || echo "")

  # Use python3 to perform the INSERT via parameterized sqlite3 call.
  # This avoids shell quoting injection: python handles all escaping.
  python3 - "${DB}" <<PYEOF
import sys, sqlite3, json

db_path = sys.argv[1]
conn = sqlite3.connect(db_path)
cur = conn.cursor()

def none_if_empty(v):
    return None if v == '' else v

plan_id_val = none_if_empty("""${plan_id}""")
plan_id_int = int(plan_id_val) if plan_id_val is not None else None

cur.execute(
    """
    INSERT INTO solve_sessions (
        user_input, constitution_check, triage_level,
        clarification_rounds, research_findings, problem_statement,
        requirements_json, acceptance_invariants, routed_to,
        decision_audit, plan_id
    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    """,
    (
        """${user_input}""",
        none_if_empty("""${constitution_check}"""),
        none_if_empty("""${triage_level}"""),
        none_if_empty("""${clarification_rounds}"""),
        none_if_empty("""${research_findings}"""),
        none_if_empty("""${problem_statement}"""),
        none_if_empty("""${requirements_json}"""),
        none_if_empty("""${acceptance_invariants}"""),
        none_if_empty("""${routed_to}"""),
        none_if_empty("""${decision_audit}"""),
        plan_id_int,
    ),
)
conn.commit()
row_id = cur.lastrowid
conn.close()
print(f"[convergio-db-migrate-solve] Session saved: id={row_id}")
PYEOF
}

# --- Entry point -------------------------------------------------------------

COMMAND="${1:-}"
case "${COMMAND}" in
  migrate)   cmd_migrate ;;
  rollback)  cmd_rollback ;;
  save)      cmd_save "${2:-}" ;;
  *)         usage ;;
esac
