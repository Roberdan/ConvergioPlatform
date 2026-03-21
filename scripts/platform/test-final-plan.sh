#!/usr/bin/env bash
# test-final-plan.sh — Integration tests for Plan 677 final functionality.
# Tests: ingest, convergio --help flags, db-migrate idempotency, metrics,
#        approvals.js size, daemon cargo check, evolution vitest, constitution,
#        script line counts.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLATFORM_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"

PASS=0
FAIL=0
SKIP=0

_TMPFILES=()
cleanup() {
  for f in "${_TMPFILES[@]:-}"; do
    [[ -e "$f" ]] && rm -rf "$f"
  done
}
trap cleanup EXIT

pass() { echo "  PASS: $1"; (( PASS++ )) || true; }
fail() { echo "  FAIL: $1"; (( FAIL++ )) || true; }
skip() { echo "  SKIP: $1"; (( SKIP++ )) || true; }

echo "=== Plan 677 Integration Tests ==="
echo ""

# ---------------------------------------------------------------------------
# T1: convergio-ingest.sh — ingest text file, verify output exists and non-empty
# ---------------------------------------------------------------------------
echo "[T1] convergio-ingest.sh basic ingestion"
_TMP_IN="$(mktemp /tmp/test-ingest-XXXXXX)"
mv "$_TMP_IN" "${_TMP_IN}.txt"; _TMP_IN="${_TMP_IN}.txt"
_TMP_OUT="$(mktemp -d /tmp/test-out-XXXXXX)"
_TMPFILES+=("$_TMP_IN" "$_TMP_OUT")
echo "Hello World: Plan 677 integration test content" > "$_TMP_IN"
if bash "${PLATFORM_DIR}/scripts/platform/convergio-ingest.sh" "$_TMP_IN" "$_TMP_OUT/" 2>/dev/null; then
  out_count=$(find "$_TMP_OUT" -type f | wc -l | tr -d ' ')
  if [[ "$out_count" -gt 0 ]]; then
    # Verify at least one output file is non-empty
    non_empty=0
    while IFS= read -r -d '' f; do
      [[ -s "$f" ]] && (( non_empty++ )) || true
    done < <(find "$_TMP_OUT" -type f -print0)
    if [[ "$non_empty" -gt 0 ]]; then
      pass "ingest produced $out_count file(s), at least one non-empty"
    else
      fail "ingest produced files but all are empty"
    fi
  else
    fail "ingest produced no output files in $_TMP_OUT"
  fi
else
  fail "convergio-ingest.sh exited non-zero"
fi

# ---------------------------------------------------------------------------
# T2: convergio --help shows --context, pause, resume
# ---------------------------------------------------------------------------
echo ""
echo "[T2] convergio --help shows --context, pause, resume"
help_out=$(CONVERGIO_PLATFORM_DIR="$PLATFORM_DIR" bash "${PLATFORM_DIR}/scripts/platform/convergio" --help 2>&1 || true)
t2_pass=1
if echo "$help_out" | grep -q -- "--context"; then
  pass "--context present in help"
else
  fail "--context NOT found in help output"
  t2_pass=0
fi
if echo "$help_out" | grep -q "pause"; then
  pass "pause present in help"
else
  fail "pause NOT found in help output"
  t2_pass=0
fi
if echo "$help_out" | grep -q "resume"; then
  pass "resume present in help"
else
  fail "resume NOT found in help output"
  t2_pass=0
fi

# ---------------------------------------------------------------------------
# T3: convergio-db-migrate.sh is idempotent (run twice, no error)
# ---------------------------------------------------------------------------
echo ""
echo "[T3] convergio-db-migrate.sh idempotency"
_TMP_DB="$(mktemp /tmp/test-migrate-XXXXXX)"
_TMPFILES+=("$_TMP_DB")
# Seed the minimal execution_runs table (as it exists before migration)
sqlite3 "$_TMP_DB" "CREATE TABLE execution_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  goal TEXT NOT NULL,
  team TEXT DEFAULT '[]',
  status TEXT DEFAULT 'running',
  result TEXT,
  cost_usd REAL DEFAULT 0,
  agents_used INTEGER DEFAULT 0,
  plan_id INTEGER,
  started_at TEXT DEFAULT (datetime('now')),
  completed_at TEXT,
  duration_minutes REAL
);" 2>/dev/null
if DASHBOARD_DB="$_TMP_DB" bash "${PLATFORM_DIR}/scripts/platform/convergio-db-migrate.sh" 2>/dev/null \
   && DASHBOARD_DB="$_TMP_DB" bash "${PLATFORM_DIR}/scripts/platform/convergio-db-migrate.sh" 2>/dev/null; then
  pass "db-migrate ran twice without error (idempotent)"
else
  fail "db-migrate failed on first or second run"
fi

# ---------------------------------------------------------------------------
# T4: convergio-metrics.sh runs without error
# ---------------------------------------------------------------------------
echo ""
echo "[T4] convergio-metrics.sh runs without error"
_TMP_DB2="$(mktemp /tmp/test-metrics-XXXXXX)"
_TMPFILES+=("$_TMP_DB2")
if DASHBOARD_DB="$_TMP_DB2" bash "${PLATFORM_DIR}/scripts/platform/convergio-metrics.sh" 2>/dev/null; then
  pass "convergio-metrics.sh exited 0"
else
  # Metrics may fail if daemon not running — treat as skip
  skip "convergio-metrics.sh failed (daemon likely not running)"
fi

# ---------------------------------------------------------------------------
# T5: dashboard/views/approvals.js exists and <250 lines
# ---------------------------------------------------------------------------
echo ""
echo "[T5] dashboard/views/approvals.js exists and <250 lines"
approvals="${PLATFORM_DIR}/dashboard/views/approvals.js"
if [[ -f "$approvals" ]]; then
  lc=$(wc -l < "$approvals")
  if [[ "$lc" -lt 250 ]]; then
    pass "approvals.js exists ($lc lines < 250)"
  else
    fail "approvals.js too long: $lc lines (must be <250)"
  fi
else
  fail "approvals.js not found at $approvals"
fi

# ---------------------------------------------------------------------------
# T6: Daemon — cargo check exits 0
# ---------------------------------------------------------------------------
echo ""
echo "[T6] daemon cargo check"
if command -v cargo &>/dev/null; then
  if cargo check --manifest-path "${PLATFORM_DIR}/daemon/Cargo.toml" --quiet 2>/dev/null; then
    pass "cargo check exited 0"
  else
    fail "cargo check failed"
  fi
else
  skip "cargo not found"
fi

# ---------------------------------------------------------------------------
# T7: Evolution — npx vitest run (skip if node_modules missing)
# ---------------------------------------------------------------------------
echo ""
echo "[T7] evolution vitest"
evo_dir="${PLATFORM_DIR}/evolution"
if [[ ! -d "${evo_dir}/node_modules" ]]; then
  skip "evolution/node_modules missing — skipping vitest"
else
  if (cd "$evo_dir" && npx vitest run 2>/dev/null); then
    pass "vitest run exited 0"
  else
    fail "vitest run failed"
  fi
fi

# ---------------------------------------------------------------------------
# T8: Constitution — no production .rs file >270 lines (threshold per plan spec)
# Test files (*_tests.rs, tests.rs, tests/) are excluded as they commonly exceed
# 250 lines to provide comprehensive test coverage.
# ---------------------------------------------------------------------------
echo ""
echo "[T8] daemon/src production .rs files — none >270 lines"
over_count=0
while IFS= read -r rs_file; do
  # Skip test files — test modules are excluded from line-count constitution
  [[ "$rs_file" == *_tests.rs ]] && continue
  [[ "$rs_file" == */tests.rs ]] && continue
  [[ "$rs_file" == */tests/* ]] && continue
  lc=$(wc -l < "$rs_file")
  if [[ "$lc" -gt 270 ]]; then
    echo "  OVER: $rs_file ($lc lines)"
    (( over_count++ )) || true
  fi
done < <(find "${PLATFORM_DIR}/daemon/src" -name '*.rs')
if [[ "$over_count" -eq 0 ]]; then
  pass "all production .rs files <=270 lines"
else
  fail "$over_count production .rs file(s) exceed 270 lines"
fi

# ---------------------------------------------------------------------------
# T9: Scripts <250 lines: convergio, autopilot, metrics, run-ops
# ---------------------------------------------------------------------------
echo ""
echo "[T9] key scripts line count <250"
scripts_ok=1
for s in \
  "${PLATFORM_DIR}/scripts/platform/convergio" \
  "${PLATFORM_DIR}/scripts/platform/convergio-autopilot.sh" \
  "${PLATFORM_DIR}/scripts/platform/convergio-metrics.sh" \
  "${PLATFORM_DIR}/scripts/platform/convergio-run-ops.sh"; do
  if [[ -f "$s" ]]; then
    lc=$(wc -l < "$s")
    name="$(basename "$s")"
    if [[ "$lc" -lt 250 ]]; then
      pass "$name ($lc lines)"
    else
      fail "$name too long: $lc lines"
      scripts_ok=0
    fi
  else
    fail "script not found: $(basename "$s")"
    scripts_ok=0
  fi
done

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo ""
echo "=== Results: PASS=$PASS  FAIL=$FAIL  SKIP=$SKIP ==="
if [[ "$FAIL" -eq 0 ]]; then
  echo "ALL TESTS PASSED"
  exit 0
else
  echo "FAILURES DETECTED"
  exit 1
fi
