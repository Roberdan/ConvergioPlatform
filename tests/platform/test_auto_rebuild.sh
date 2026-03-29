#!/usr/bin/env bash
# test_auto_rebuild.sh — Tests for scripts/platform/auto-rebuild.sh
# Validates syntax, required functions, error handling, and notification logic.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SCRIPT="$REPO_ROOT/scripts/platform/auto-rebuild.sh"
PASS=0; FAIL=0; TOTAL=0

assert() {
  local desc="$1"; shift
  TOTAL=$((TOTAL + 1))
  if "$@" >/dev/null 2>&1; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    echo "FAIL: $desc"
  fi
}

assert_contains() {
  local desc="$1" file="$2" pattern="$3"
  TOTAL=$((TOTAL + 1))
  if grep -q "$pattern" "$file"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    echo "FAIL: $desc (pattern: $pattern)"
  fi
}

# --- Existence and syntax ---
assert "script exists" test -f "$SCRIPT"
assert "script is executable" test -x "$SCRIPT"
assert "bash -n syntax check" bash -n "$SCRIPT"

# --- Required patterns ---
assert_contains "uses set -euo pipefail" "$SCRIPT" "set -euo pipefail"
assert_contains "has cleanup trap" "$SCRIPT" "trap.*EXIT"
assert_contains "sources convergio env" "$SCRIPT" '\.convergio/env'
assert_contains "git pull --rebase" "$SCRIPT" "git pull --rebase origin main"
assert_contains "checks daemon/ changes" "$SCRIPT" 'daemon/'
assert_contains "cargo build with kernel feature" "$SCRIPT" "cargo build.*--features kernel.*--release"
assert_contains "restarts daemon via start.sh" "$SCRIPT" "start.sh"
assert_contains "sends success notification" "$SCRIPT" '/api/notify'
assert_contains "sends failure notification" "$SCRIPT" "notify_result.*fail"
assert_contains "has LOCK_FILE to prevent overlap" "$SCRIPT" "LOCK_FILE"
assert_contains "ulimit for file descriptors" "$SCRIPT" "ulimit"
assert_contains "log file output" "$SCRIPT" "LOG_FILE"

# --- Launchd plist ---
PLIST="$REPO_ROOT/scripts/platform/com.convergio.auto-rebuild.plist"
assert "plist exists" test -f "$PLIST"
assert_contains "plist label correct" "$PLIST" "com.convergio.auto-rebuild"
assert_contains "plist runs every 300s" "$PLIST" "300"
assert_contains "plist references auto-rebuild.sh" "$PLIST" "auto-rebuild.sh"
assert_contains "plist has stdout log" "$PLIST" "StandardOutPath"
assert_contains "plist has stderr log" "$PLIST" "StandardErrorPath"

# --- Summary ---
echo ""
echo "auto-rebuild tests: $PASS/$TOTAL passed, $FAIL failed"
[[ $FAIL -eq 0 ]] && exit 0 || exit 1
