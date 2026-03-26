#!/usr/bin/env bash
# test-setup-coordinator-remote.sh — Tests for setup-coordinator-remote.sh
# Tests use local git repos to simulate peer behavior without SSH
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SETUP_SCRIPT="$SCRIPT_DIR/../setup-coordinator-remote.sh"
PASS=0
FAIL=0
TMPDIR_BASE=""

# Cleanup on exit
cleanup() {
  if [[ -n "$TMPDIR_BASE" && -d "$TMPDIR_BASE" ]]; then
    rm -rf "$TMPDIR_BASE"
  fi
}
trap cleanup EXIT

ok()   { echo "  PASS: $*"; PASS=$((PASS + 1)); }
fail() { echo "  FAIL: $*"; FAIL=$((FAIL + 1)); }

# Create isolated temp workspace
TMPDIR_BASE="$(mktemp -d)"
FAKE_PEER_REPO="$TMPDIR_BASE/peer-repo"
FAKE_COORD_REPO="$TMPDIR_BASE/coordinator-repo"

# Setup: create a bare "coordinator" repo
setup_coordinator_repo() {
  git init --bare "$FAKE_COORD_REPO" -q
}

# Setup: create a peer repo with no remotes
setup_peer_repo_clean() {
  rm -rf "$FAKE_PEER_REPO"
  git init "$FAKE_PEER_REPO" -q
  # Need at least one commit for git remote to work properly
  git -C "$FAKE_PEER_REPO" commit --allow-empty -m "init" -q
}

# Setup: create a peer repo that already has 'coordinator' remote
setup_peer_repo_with_remote() {
  rm -rf "$FAKE_PEER_REPO"
  git init "$FAKE_PEER_REPO" -q
  git -C "$FAKE_PEER_REPO" commit --allow-empty -m "init" -q
  git -C "$FAKE_PEER_REPO" remote add coordinator "$FAKE_COORD_REPO"
}

# Source the script in "test mode" — skip SSH, operate on local REPO_PATH
# We override SSH calls by passing LOCAL_REPO_PATH env var
run_setup_local() {
  local repo_path="$1"
  local coord_url="$2"
  LOCAL_REPO_PATH="$repo_path" \
  SKIP_SSH_CHECK=1 \
  bash "$SETUP_SCRIPT" --local-repo "$repo_path" --coordinator-url "$coord_url"
}

echo "=== setup-coordinator-remote.sh tests ==="
echo ""

setup_coordinator_repo

# Test 1: Adds 'coordinator' remote when it does not exist
echo "[T1] Adds coordinator remote when missing"
setup_peer_repo_clean
if run_setup_local "$FAKE_PEER_REPO" "$FAKE_COORD_REPO" >/dev/null 2>&1; then
  actual_url="$(git -C "$FAKE_PEER_REPO" remote get-url coordinator 2>/dev/null || echo '')"
  if [[ "$actual_url" == "$FAKE_COORD_REPO" ]]; then
    ok "remote 'coordinator' added with correct URL"
  else
    fail "remote URL mismatch: got '$actual_url', expected '$FAKE_COORD_REPO'"
  fi
else
  fail "script exited non-zero when adding missing remote"
fi

# Test 2: Skips (returns 0) when 'coordinator' remote already exists with same URL
echo "[T2] Skips gracefully when coordinator remote already exists"
setup_peer_repo_with_remote
if run_setup_local "$FAKE_PEER_REPO" "$FAKE_COORD_REPO" >/dev/null 2>&1; then
  actual_url="$(git -C "$FAKE_PEER_REPO" remote get-url coordinator 2>/dev/null || echo '')"
  if [[ "$actual_url" == "$FAKE_COORD_REPO" ]]; then
    ok "existing remote kept, script returned 0"
  else
    fail "remote URL changed unexpectedly: '$actual_url'"
  fi
else
  fail "script exited non-zero on already-configured repo"
fi

# Test 3: Exits 1 when required args are missing
echo "[T3] Exits 1 when --local-repo missing"
if bash "$SETUP_SCRIPT" --coordinator-url "ssh://user@host/repo" >/dev/null 2>&1; then
  fail "should have exited non-zero with missing --local-repo"
else
  ok "exits 1 when --local-repo missing"
fi

# Test 4: Exits 1 when --coordinator-url missing
echo "[T4] Exits 1 when --coordinator-url missing"
if bash "$SETUP_SCRIPT" --local-repo "$FAKE_PEER_REPO" >/dev/null 2>&1; then
  fail "should have exited non-zero with missing --coordinator-url"
else
  ok "exits 1 when --coordinator-url missing"
fi

# Test 5: SSH peer mode — missing --peer fails without SKIP_SSH_CHECK
echo "[T5] SSH mode: exits 1 when --peer given but SSH not available (BatchMode)"
# We expect this to fail fast when BatchMode=yes and host is unreachable
if bash "$SETUP_SCRIPT" \
     --peer "192.0.2.1" \
     --peer-repo-path "/nonexistent" \
     --coordinator-url "ssh://coordinator/repo" \
     >/dev/null 2>&1; then
  fail "should have exited non-zero for unreachable SSH host"
else
  ok "exits 1 for unreachable SSH host"
fi

echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="
[[ $FAIL -eq 0 ]] && exit 0 || exit 1
