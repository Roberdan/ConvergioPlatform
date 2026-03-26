#!/usr/bin/env bash
# post-commit-sync.test.sh — TDD test suite for post-commit-sync.sh
# Run: bash scripts/mesh/post-commit-sync.test.sh
# Tests: patch-save logic, retry detection, dry-run, shellcheck
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SYNC_SCRIPT="$SCRIPT_DIR/post-commit-sync.sh"
PASS=0
FAIL=0

pass() { echo "  [PASS] $1"; PASS=$((PASS + 1)); }
fail() { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }

echo "=== post-commit-sync.sh test suite ==="
echo ""

# T1: Script exists
if [[ -f "$SYNC_SCRIPT" ]]; then
  pass "T1: post-commit-sync.sh exists"
else
  fail "T1: post-commit-sync.sh does not exist"
fi

# T2: Bash syntax check
if bash -n "$SYNC_SCRIPT" 2>/dev/null; then
  pass "T2: bash -n syntax check passes"
else
  fail "T2: bash -n syntax check FAILED"
fi

# T3: shellcheck clean
if command -v shellcheck &>/dev/null; then
  if shellcheck "$SYNC_SCRIPT" 2>/dev/null; then
    pass "T3: shellcheck clean"
  else
    fail "T3: shellcheck found issues"
  fi
else
  pass "T3: shellcheck not installed — skip"
fi

# T4: --help exits 0 and prints usage
if HELP_OUT="$(bash "$SYNC_SCRIPT" --help 2>&1)"; then
  if echo "$HELP_OUT" | grep -qi "usage"; then
    pass "T4: --help prints usage"
  else
    fail "T4: --help missing 'usage' in output"
  fi
else
  fail "T4: --help exited non-zero"
fi

# T5: Patch-save logic — when rsync fails, a patch is created in pending-patches
# Setup: temp git repo to simulate a real commit
T5_TMPDIR="$(mktemp -d)"
trap 'rm -rf "$T5_TMPDIR"' EXIT

T5_REPO="$T5_TMPDIR/repo"
mkdir -p "$T5_REPO"
cd "$T5_REPO"
git init -q
git config user.email "test@example.com"
git config user.name "Test"
echo "hello" > file.txt
git add file.txt
git commit -q -m "initial"

# Run sync with an unreachable peer → should save patch in .git/pending-patches
PATCH_DIR="$T5_REPO/.git/pending-patches"
DELEGATION_ID="test-delegation-123"

# Export vars the script needs; set an invalid peer to force rsync failure
export CONVERGIO_COORDINATOR_HOST="192.0.2.1"   # TEST-NET — unreachable by design
export CONVERGIO_COORDINATOR_PATH="/tmp/nonexistent"
export CONVERGIO_DELEGATION_ID="$DELEGATION_ID"
export CONVERGIO_DAEMON_URL="http://127.0.0.1:19999"  # not listening — non-fatal

cd "$T5_REPO"
if bash "$SYNC_SCRIPT" --repo-path "$T5_REPO" --dry-run-patch 2>/dev/null; then
  if [[ -d "$PATCH_DIR" ]]; then
    PATCH_COUNT="$(find "$PATCH_DIR" -name "*.patch" | wc -l | tr -d ' ')"
    if [[ "$PATCH_COUNT" -gt 0 ]]; then
      pass "T5: patch saved in .git/pending-patches on sync failure"
    else
      fail "T5: pending-patches dir exists but no .patch files found"
    fi
  else
    fail "T5: .git/pending-patches directory was not created"
  fi
else
  fail "T5: script exited non-zero in dry-run-patch mode"
fi

# T6: Retry detection — pending patches are listed when they exist
cd "$T5_REPO"
RETRY_OUT="$(bash "$SYNC_SCRIPT" --repo-path "$T5_REPO" --list-pending 2>&1)"
if echo "$RETRY_OUT" | grep -q "pending-patches"; then
  pass "T6: --list-pending reports pending patches"
else
  fail "T6: --list-pending missing pending-patches info in output"
fi

# T7: Integration — rsync succeeds with local target (loopback)
T7_SRC="$T5_REPO"
T7_DST="$T5_TMPDIR/coordinator"
mkdir -p "$T7_DST"

# Use local path as coordinator (no SSH, just path-based rsync)
export CONVERGIO_COORDINATOR_HOST="localhost"
export CONVERGIO_COORDINATOR_PATH="$T7_DST"
export CONVERGIO_COORDINATOR_LOCAL_PATH="$T7_DST"  # skip SSH, use direct path
export CONVERGIO_DELEGATION_ID="integ-test-456"

cd "$T7_SRC"
if bash "$SYNC_SCRIPT" --repo-path "$T7_SRC" --local-sync 2>/dev/null; then
  if [[ -f "$T7_DST/file.txt" ]]; then
    pass "T7: integration — rsync syncs worktree to coordinator (local)"
  else
    fail "T7: integration — file.txt not found in coordinator dest"
  fi
else
  fail "T7: integration — script exited non-zero on local rsync"
fi

# T8: No pending patches remain after successful sync
REMAINING="$(find "$T5_REPO/.git/pending-patches" -name "*.patch" 2>/dev/null | wc -l | tr -d ' ')"
if [[ "$REMAINING" -eq 0 ]]; then
  pass "T8: pending patches cleared after successful sync"
else
  fail "T8: $REMAINING patch(es) remain after successful sync"
fi

# T9: Line count <= 250
LINE_COUNT=$(wc -l < "$SYNC_SCRIPT")
if [[ "$LINE_COUNT" -le 250 ]]; then
  pass "T9: line count ${LINE_COUNT} <= 250"
else
  fail "T9: line count ${LINE_COUNT} EXCEEDS 250"
fi

# T10: Script is executable (can be installed as git hook)
if [[ -x "$SYNC_SCRIPT" ]]; then
  pass "T10: script is executable"
else
  fail "T10: script is not executable"
fi

echo ""
echo "=== Results: ${PASS} passed, ${FAIL} failed ==="
[[ $FAIL -eq 0 ]]
