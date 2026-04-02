#!/usr/bin/env bash
# offline-recovery.test.sh — Test suite for scripts/mesh/offline-recovery.sh
# Run: bash scripts/mesh/offline-recovery.test.sh
# Verifies: patch detection, clean state, recovery status reporting
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RECOVERY_SCRIPT="$SCRIPT_DIR/offline-recovery.sh"
PASS=0
FAIL=0

pass() { echo "  [PASS] $1"; PASS=$((PASS + 1)); }
fail() { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }

echo "=== offline-recovery.sh test suite ==="
echo ""

# T1: Script exists
if [[ -f "$RECOVERY_SCRIPT" ]]; then
  pass "T1: scripts/mesh/offline-recovery.sh exists"
else
  fail "T1: scripts/mesh/offline-recovery.sh does not exist"
fi

# T2: Syntax check (bash -n)
if [[ -f "$RECOVERY_SCRIPT" ]] && bash -n "$RECOVERY_SCRIPT" 2>/dev/null; then
  pass "T2: bash -n syntax check passes"
else
  fail "T2: bash -n syntax check FAILED"
fi

# T3: shellcheck clean
if command -v shellcheck &>/dev/null && [[ -f "$RECOVERY_SCRIPT" ]]; then
  if shellcheck "$RECOVERY_SCRIPT" 2>/dev/null; then
    pass "T3: shellcheck clean"
  else
    fail "T3: shellcheck found issues"
  fi
else
  pass "T3: shellcheck not installed or script missing — skip"
fi

# T4: --help flag exits 0 and prints usage
if [[ -f "$RECOVERY_SCRIPT" ]]; then
  HELP_OUT="$(bash "$RECOVERY_SCRIPT" --help 2>&1 || true)"
  if echo "$HELP_OUT" | grep -qi "usage"; then
    pass "T4: --help prints usage"
  else
    fail "T4: --help missing usage output"
  fi
else
  fail "T4: script not found"
fi

# T5: no pending patches → exits 0 with 'no pending patches' message
if [[ -f "$RECOVERY_SCRIPT" ]]; then
  TMPDIR_T5="$(mktemp -d)"
  # Provide a git dir without pending-patches
  T5_OUT="$(GIT_DIR="$TMPDIR_T5" bash "$RECOVERY_SCRIPT" --check-only 2>&1 || true)"
  if echo "$T5_OUT" | grep -qi "no pending"; then
    pass "T5: no pending patches → prints 'no pending' message"
  else
    fail "T5: expected 'no pending' message, got: $T5_OUT"
  fi
  rm -rf "$TMPDIR_T5"
else
  fail "T5: script not found"
fi

# T6: pending patches directory exists → detected and reported
if [[ -f "$RECOVERY_SCRIPT" ]]; then
  TMPDIR_T6="$(mktemp -d)"
  # Create .git/pending-patches with a dummy patch file
  mkdir -p "$TMPDIR_T6/pending-patches"
  echo "dummy patch content" > "$TMPDIR_T6/pending-patches/0001-test.patch"
  T6_OUT="$(GIT_DIR="$TMPDIR_T6" bash "$RECOVERY_SCRIPT" --check-only 2>&1 || true)"
  if echo "$T6_OUT" | grep -qi "pending\|patch\|found"; then
    pass "T6: pending patches directory with patches → detected"
  else
    fail "T6: expected patch detection, got: $T6_OUT"
  fi
  rm -rf "$TMPDIR_T6"
else
  fail "T6: script not found"
fi

# T7: --check-only does not apply patches (non-destructive)
if [[ -f "$RECOVERY_SCRIPT" ]]; then
  TMPDIR_T7="$(mktemp -d)"
  mkdir -p "$TMPDIR_T7/pending-patches"
  PATCH_FILE="$TMPDIR_T7/pending-patches/0001-test.patch"
  echo "dummy patch content" > "$PATCH_FILE"
  bash "$RECOVERY_SCRIPT" --check-only 2>&1 || true
  # Patch file must still exist (check-only should not consume/delete it)
  if [[ -f "$PATCH_FILE" ]]; then
    pass "T7: --check-only does not remove patch files"
  else
    fail "T7: --check-only removed patch files unexpectedly"
  fi
  rm -rf "$TMPDIR_T7"
else
  fail "T7: script not found"
fi

# T8: line count does not exceed 250
if [[ -f "$RECOVERY_SCRIPT" ]]; then
  LINE_COUNT=$(wc -l < "$RECOVERY_SCRIPT")
  if [[ "$LINE_COUNT" -le 250 ]]; then
    pass "T8: line count ${LINE_COUNT} <= 250"
  else
    fail "T8: line count ${LINE_COUNT} EXCEEDS 250"
  fi
else
  fail "T8: script not found"
fi

# T9: heartbeat integration — mesh-heartbeat.sh was replaced by daemon heartbeat
# This test is now a no-op since the script was removed
pass "T9: mesh-heartbeat.sh replaced by daemon heartbeat (skip)"

echo ""
echo "=== Results: ${PASS} passed, ${FAIL} failed ==="
[[ $FAIL -eq 0 ]]
