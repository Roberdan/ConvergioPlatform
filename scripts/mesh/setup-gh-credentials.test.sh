#!/usr/bin/env bash
# setup-gh-credentials.test.sh — TDD test suite for setup-gh-credentials.sh
# Run: bash scripts/mesh/setup-gh-credentials.test.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_SCRIPT="$SCRIPT_DIR/setup-gh-credentials.sh"
PASS=0
FAIL=0

pass() { echo "  [PASS] $1"; PASS=$((PASS + 1)); }
fail() { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }

echo "=== setup-gh-credentials.sh test suite ==="
echo ""

# T1: Script exists
if [[ -f "$TARGET_SCRIPT" ]]; then
  pass "T1: setup-gh-credentials.sh exists"
else
  fail "T1: setup-gh-credentials.sh does not exist"
fi

# T2: Bash syntax check
if bash -n "$TARGET_SCRIPT" 2>/dev/null; then
  pass "T2: bash -n syntax check passes"
else
  fail "T2: bash -n syntax check FAILED"
fi

# T3: shellcheck clean
if command -v shellcheck &>/dev/null; then
  if shellcheck "$TARGET_SCRIPT" 2>/dev/null; then
    pass "T3: shellcheck clean"
  else
    fail "T3: shellcheck found issues"
  fi
else
  pass "T3: shellcheck not installed — skip"
fi

# T4: --help exits 0 and prints usage
if HELP_OUT="$(bash "$TARGET_SCRIPT" --help 2>&1)"; then
  if echo "$HELP_OUT" | grep -qi "usage"; then
    pass "T4: --help prints usage"
  else
    fail "T4: --help missing 'usage' in output"
  fi
else
  fail "T4: --help exited non-zero"
fi

# T5: --dry-run flag prints actions without modifying git config
TMPDIR_TEST="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_TEST"' EXIT

# Create two fake repos
REPO_A="$TMPDIR_TEST/repos/repo-a"
REPO_B="$TMPDIR_TEST/repos/repo-b"
mkdir -p "$REPO_A" "$REPO_B"

cd "$REPO_A" && git init -q
git -C "$REPO_A" remote add origin "https://github.com/personal/repo-a.git"

cd "$REPO_B" && git init -q
git -C "$REPO_B" remote add origin "https://github.com/org-work/repo-b.git"

DRY_OUT="$(bash "$TARGET_SCRIPT" --scan-dir "$TMPDIR_TEST/repos" --dry-run 2>&1)"
if echo "$DRY_OUT" | grep -q "repo-a"; then
  pass "T5: --dry-run lists repo-a"
else
  fail "T5: --dry-run does not list repo-a"
fi

# T6: Credential helper is set after real run
bash "$TARGET_SCRIPT" --scan-dir "$TMPDIR_TEST/repos" 2>/dev/null || true
HELPER_A="$(git -C "$REPO_A" config --local credential.helper 2>/dev/null || echo "")"
if [[ -n "$HELPER_A" ]]; then
  pass "T6: credential.helper set for repo-a"
else
  fail "T6: credential.helper NOT set for repo-a"
fi

# T7: Handles repos without github.com remotes gracefully
REPO_C="$TMPDIR_TEST/repos/repo-c"
mkdir -p "$REPO_C"
cd "$REPO_C" && git init -q
git -C "$REPO_C" remote add origin "https://gitlab.com/someone/repo-c.git"

OUT7="$(bash "$TARGET_SCRIPT" --scan-dir "$TMPDIR_TEST/repos" --dry-run 2>&1)"
if echo "$OUT7" | grep -q "repo-c"; then
  fail "T7: non-github repo should be skipped"
else
  pass "T7: non-github repo correctly skipped"
fi

# T8: SSH-style remotes (git@github.com:...) are detected
REPO_D="$TMPDIR_TEST/repos/repo-d"
mkdir -p "$REPO_D"
cd "$REPO_D" && git init -q
git -C "$REPO_D" remote add origin "git@github.com:personal/repo-d.git"

OUT8="$(bash "$TARGET_SCRIPT" --scan-dir "$TMPDIR_TEST/repos" --dry-run 2>&1)"
if echo "$OUT8" | grep -q "repo-d"; then
  pass "T8: SSH-style github remote detected"
else
  fail "T8: SSH-style github remote NOT detected"
fi

# T9: Line count <= 250
LINE_COUNT=$(wc -l < "$TARGET_SCRIPT")
if [[ "$LINE_COUNT" -le 250 ]]; then
  pass "T9: line count ${LINE_COUNT} <= 250"
else
  fail "T9: line count ${LINE_COUNT} EXCEEDS 250"
fi

# T10: Script is executable
if [[ -x "$TARGET_SCRIPT" ]]; then
  pass "T10: script is executable"
else
  fail "T10: script is not executable"
fi

echo ""
echo "=== Results: ${PASS} passed, ${FAIL} failed ==="
[[ $FAIL -eq 0 ]]
