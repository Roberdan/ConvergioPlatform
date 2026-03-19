#!/usr/bin/env bash
# test-setup-e2e.sh — E2E test for setup.sh idempotency and correctness
# Output: TAP format (ok N description / not ok N description)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

TEST_NUM=0
FAILURES=0

tap_ok()   { TEST_NUM=$((TEST_NUM + 1)); echo "ok $TEST_NUM - $1"; }
tap_fail() { TEST_NUM=$((TEST_NUM + 1)); FAILURES=$((FAILURES + 1)); echo "not ok $TEST_NUM - $1"; }
tap_skip() { TEST_NUM=$((TEST_NUM + 1)); echo "ok $TEST_NUM - SKIP $1"; }

# shellcheck disable=SC2329
cleanup() {
  # Nothing destructive to undo; setup.sh is idempotent
  true
}
trap cleanup EXIT

# --- Test 1: setup.sh exists and is executable ---
test_setup_executable() {
  if [[ -x "$REPO_DIR/setup.sh" ]]; then
    tap_ok "setup.sh exists and is executable"
  else
    tap_fail "setup.sh not found or not executable at $REPO_DIR/setup.sh"
  fi
}

# --- Test 2: setup.sh runs successfully ---
test_setup_runs() {
  local exit_code=0
  (cd "$REPO_DIR" && bash setup.sh) >/dev/null 2>&1 || exit_code=$?
  if [[ "$exit_code" -eq 0 ]]; then
    tap_ok "setup.sh runs successfully (exit 0)"
  else
    tap_fail "setup.sh failed with exit code $exit_code"
  fi
}

# --- Test 3: CLAUDE.md symlink ---
test_claude_md_symlink() {
  if [[ -L "$HOME/.claude/CLAUDE.md" ]]; then
    tap_ok "CLAUDE.md symlink exists at ~/.claude/CLAUDE.md"
  else
    tap_fail "CLAUDE.md symlink missing at ~/.claude/CLAUDE.md"
  fi
}

# --- Test 4: rules/ symlink ---
test_rules_symlink() {
  if [[ -L "$HOME/.claude/rules" ]]; then
    tap_ok "rules/ symlink exists at ~/.claude/rules"
  else
    tap_fail "rules/ symlink missing at ~/.claude/rules"
  fi
}

# --- Test 5: commands/ symlink ---
test_commands_symlink() {
  if [[ -L "$HOME/.claude/commands" ]]; then
    tap_ok "commands/ symlink exists at ~/.claude/commands"
  elif [[ -d "$HOME/.claude/commands" ]]; then
    tap_skip "commands/ is a directory (not symlink)"
  else
    tap_fail "commands/ symlink missing at ~/.claude/commands"
  fi
}

# --- Test 6: agents/ symlink ---
test_agents_symlink() {
  if [[ -L "$HOME/.claude/agents" ]]; then
    tap_ok "agents/ symlink exists at ~/.claude/agents"
  elif [[ -d "$HOME/.claude/agents" ]]; then
    tap_skip "agents/ is a directory (not symlink)"
  else
    tap_fail "agents/ symlink missing at ~/.claude/agents"
  fi
}

# --- Test 7: settings.json exists (project-level) ---
test_settings_exists() {
  if [[ -f "$REPO_DIR/.claude/settings.json" ]]; then
    tap_ok "settings.json exists at .claude/settings.json"
  else
    tap_fail "settings.json missing at $REPO_DIR/.claude/settings.json"
  fi
}

# --- Test 8: settings.json valid JSON ---
test_settings_valid_json() {
  if [[ ! -f "$REPO_DIR/.claude/settings.json" ]]; then
    tap_skip "settings.json not found, cannot validate JSON"
    return
  fi
  if jq . "$REPO_DIR/.claude/settings.json" >/dev/null 2>&1; then
    tap_ok "settings.json is valid JSON"
  else
    tap_fail "settings.json is not valid JSON"
  fi
}

# --- Test 9: Hook count (at least 8 matchers) ---
test_hook_count() {
  if [[ ! -f "$REPO_DIR/.claude/settings.json" ]]; then
    tap_skip "settings.json not found, cannot count hooks"
    return
  fi
  local count
  count=$(jq '[.. | objects | select(.matcher?) | .matcher] | length' \
    "$REPO_DIR/.claude/settings.json" 2>/dev/null) || count=0
  if [[ "$count" -ge 8 ]]; then
    tap_ok "hook count: $count matchers (>= 8)"
  else
    tap_fail "hook count: $count matchers (expected >= 8)"
  fi
}

# --- Test 10: DASHBOARD_DB set in shell profile ---
test_dashboard_db() {
  if grep -q 'DASHBOARD_DB' "$HOME/.zshenv" 2>/dev/null; then
    tap_ok "DASHBOARD_DB found in ~/.zshenv"
  elif grep -q 'DASHBOARD_DB' "$HOME/.bashrc" 2>/dev/null; then
    tap_ok "DASHBOARD_DB found in ~/.bashrc"
  elif [[ -n "${DASHBOARD_DB:-}" ]]; then
    tap_ok "DASHBOARD_DB set in environment"
  else
    tap_fail "DASHBOARD_DB not found in shell profile or environment"
  fi
}

# --- Test 11: Idempotency (run setup.sh again) ---
test_idempotency() {
  local exit_code=0
  (cd "$REPO_DIR" && bash setup.sh) >/dev/null 2>&1 || exit_code=$?
  if [[ "$exit_code" -eq 0 ]]; then
    tap_ok "idempotency: second setup.sh run exits 0"
  else
    tap_fail "idempotency: second setup.sh run failed with exit code $exit_code"
  fi
}

# --- Run all tests ---
test_setup_executable
test_setup_runs
test_claude_md_symlink
test_rules_symlink
test_commands_symlink
test_agents_symlink
test_settings_exists
test_settings_valid_json
test_hook_count
test_dashboard_db
test_idempotency

echo "1..${TEST_NUM}"
if [[ "$FAILURES" -gt 0 ]]; then
  exit 1
fi
exit 0
