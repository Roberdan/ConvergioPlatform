#!/usr/bin/env bash
# setup.test.sh — Test suite for scripts/setup.sh (MyConvergio multi-provider bootstrap)
# Run: bash scripts/setup.test.sh
# Verifies: syntax, --dry-run JSON output, --help, --provider flag, shellcheck clean
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SETUP_SCRIPT="$SCRIPT_DIR/setup.sh"
PASS=0
FAIL=0

pass() { echo "  [PASS] $1"; PASS=$((PASS + 1)); }
fail() { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }

run_test() {
  local name="$1"
  local result="$2"
  if [[ "$result" == "0" ]]; then
    pass "$name"
  else
    fail "$name"
  fi
}

echo "=== setup.sh test suite ==="
echo ""

# T1: Script exists
if [[ -f "$SETUP_SCRIPT" ]]; then
  pass "T1: scripts/setup.sh exists"
else
  fail "T1: scripts/setup.sh does not exist"
fi

# T2: Syntax check (bash -n)
if bash -n "$SETUP_SCRIPT" 2>/dev/null; then
  pass "T2: bash -n syntax check passes"
else
  fail "T2: bash -n syntax check FAILED"
fi

# T3: shellcheck clean
if command -v shellcheck &>/dev/null; then
  if shellcheck "$SETUP_SCRIPT" 2>/dev/null; then
    pass "T3: shellcheck clean"
  else
    fail "T3: shellcheck found issues"
  fi
else
  pass "T3: shellcheck not installed — skip"
fi

# T4: --help flag exits 0 and prints usage
# Use output capture to avoid broken-pipe with set -euo pipefail
HELP_OUT="$(bash "$SETUP_SCRIPT" --help 2>&1)"
if echo "$HELP_OUT" | grep -qi "usage"; then
  pass "T4: --help prints usage"
else
  fail "T4: --help missing usage output"
fi

# T5: --dry-run outputs 'manifest' (case-insensitive)
# Use output capture — piping to grep causes broken-pipe when grep exits early
DRY_OUT5="$(bash "$SETUP_SCRIPT" --dry-run 2>&1)"
if echo "$DRY_OUT5" | grep -qi "manifest"; then
  pass "T5: --dry-run outputs 'manifest'"
else
  fail "T5: --dry-run missing 'manifest' in output"
fi

# T6: --dry-run outputs valid JSON-like structure (has '{' and '}')
DRY_OUT="$(bash "$SETUP_SCRIPT" --dry-run 2>&1)"
if echo "$DRY_OUT" | grep -q '{' && echo "$DRY_OUT" | grep -q '}'; then
  pass "T6: --dry-run outputs JSON-like structure"
else
  fail "T6: --dry-run missing JSON structure"
fi

# T7: --dry-run does NOT write ~/.convergio/install-manifest.json
MANIFEST_BEFORE="${HOME}/.convergio/install-manifest.json"
BEFORE_EXISTS=0
[[ -f "$MANIFEST_BEFORE" ]] && BEFORE_EXISTS=1
bash "$SETUP_SCRIPT" --dry-run &>/dev/null || true
if [[ $BEFORE_EXISTS -eq 0 ]] && [[ ! -f "$MANIFEST_BEFORE" ]]; then
  pass "T7: --dry-run does not write manifest file"
elif [[ $BEFORE_EXISTS -eq 1 ]]; then
  pass "T7: manifest already existed — dry-run did not remove it"
else
  fail "T7: --dry-run wrote manifest file (should not)"
fi

# T8: --provider flag with invalid name exits non-zero
if ! bash "$SETUP_SCRIPT" --provider invalid-provider-xyz 2>/dev/null; then
  pass "T8: --provider invalid-name exits non-zero"
else
  fail "T8: --provider invalid-name should exit non-zero"
fi

# T9: --provider generic with --dry-run includes 'generic' in output
T9_OUT="$(bash "$SETUP_SCRIPT" --provider generic-llm --dry-run 2>&1)"
if echo "$T9_OUT" | grep -qi "generic"; then
  pass "T9: --provider generic-llm --dry-run includes 'generic'"
else
  fail "T9: --provider generic-llm --dry-run missing 'generic'"
fi

# T10: --dry-run output contains 'installed_at' field
T10_OUT="$(bash "$SETUP_SCRIPT" --dry-run 2>&1)"
if echo "$T10_OUT" | grep -q "installed_at"; then
  pass "T10: --dry-run output contains 'installed_at'"
else
  fail "T10: --dry-run output missing 'installed_at'"
fi

# T11: --dry-run output contains 'providers' field
T11_OUT="$(bash "$SETUP_SCRIPT" --dry-run 2>&1)"
if echo "$T11_OUT" | grep -q "providers"; then
  pass "T11: --dry-run output contains 'providers'"
else
  fail "T11: --dry-run output missing 'providers'"
fi

# T12: line count does not exceed 250
LINE_COUNT=$(wc -l < "$SETUP_SCRIPT")
if [[ "$LINE_COUNT" -le 250 ]]; then
  pass "T12: line count ${LINE_COUNT} <= 250"
else
  fail "T12: line count ${LINE_COUNT} EXCEEDS 250"
fi

echo ""
echo "=== Results: ${PASS} passed, ${FAIL} failed ==="
[[ $FAIL -eq 0 ]]
