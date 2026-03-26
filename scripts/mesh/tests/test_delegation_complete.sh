#!/usr/bin/env bash
# test_delegation_complete.sh — TDD tests for delegation-complete.sh
# Tests: prompt file cleanup, tmux session kill, delegation API call
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DELEGATION_COMPLETE="${SCRIPT_DIR}/../delegation-complete.sh"

PASS=0
FAIL=0

pass() { echo "  PASS: $1"; PASS=$((PASS + 1)); }
fail() { echo "  FAIL: $1"; FAIL=$((FAIL + 1)); }

# Create a stub dir with fake executables for curl/tmux interception.
# Prefix PATH so delegation-complete.sh picks up stubs first.
make_stub_dir() {
  local stubdir
  stubdir="$(mktemp -d)"
  echo "$stubdir"
}

write_stub() {
  local stubdir="$1" name="$2" body="$3"
  printf '#!/usr/bin/env bash\n%s\n' "$body" > "${stubdir}/${name}"
  chmod +x "${stubdir}/${name}"
}

# ── test: script exists and is executable ────────────────────────────────────

echo ""
echo "=== Test: script exists ==="
if [[ -x "$DELEGATION_COMPLETE" ]]; then
  pass "delegation-complete.sh is executable"
else
  fail "delegation-complete.sh not found or not executable at $DELEGATION_COMPLETE"
fi

# ── test: usage / required args ──────────────────────────────────────────────

echo ""
echo "=== Test: required args validation ==="
output="$("$DELEGATION_COMPLETE" 2>&1 || true)"
if echo "$output" | grep -qi "usage\|required\|session-name\|error"; then
  pass "no-args exits with usage message"
else
  fail "no-args should print usage, got: $output"
fi

# ── test: prompt file cleanup ────────────────────────────────────────────────

echo ""
echo "=== Test: prompt file cleanup ==="
TMPDIR_TEST="$(mktemp -d)"
PROMPT_FILE="${TMPDIR_TEST}/convergio-delegate-test.md"
echo "test prompt content" > "$PROMPT_FILE"

STUBDIR="$(make_stub_dir)"
# Stub curl to be a no-op
write_stub "$STUBDIR" curl 'echo "stub-curl $*" >/dev/null; exit 0'
# Stub git to be a no-op
write_stub "$STUBDIR" git 'echo "stub-git $*" >/dev/null; exit 0'
# Stub tmux to be a no-op
write_stub "$STUBDIR" tmux 'echo "stub-tmux $*" >/dev/null; exit 0'

if PATH="${STUBDIR}:${PATH}" DRY_RUN=1 "$DELEGATION_COMPLETE" \
    --session-name "test-session-$$" \
    --prompt-file "$PROMPT_FILE" \
    --plan-id 0 2>&1; then
  if [[ ! -f "$PROMPT_FILE" ]]; then
    pass "prompt file removed after delegation-complete"
  else
    fail "prompt file still exists after delegation-complete"
  fi
else
  fail "delegation-complete.sh exited non-zero in DRY_RUN mode"
fi
rm -rf "$TMPDIR_TEST" "$STUBDIR"

# ── test: delegation marked done via API (stub curl without DRY_RUN) ───────────
# DRY_RUN bypasses real curl; here we stub the binary and run without DRY_RUN
# so the script actually executes the stub, letting us capture the call.

echo ""
echo "=== Test: delegation API call ==="
TMPDIR_TEST="$(mktemp -d)"
PROMPT_FILE="${TMPDIR_TEST}/convergio-delegate-test.md"
echo "test prompt" > "$PROMPT_FILE"
API_LOG="${TMPDIR_TEST}/api.log"

STUBDIR="$(make_stub_dir)"
# Stub curl: log args to file, exit 0 (mimics success)
write_stub "$STUBDIR" curl "echo \"curl \$*\" >> \"${API_LOG}\"; exit 0"
# Stub git to prevent real push
write_stub "$STUBDIR" git "echo \"stub-git \$*\" >/dev/null; exit 0"
# Stub tmux to prevent real session ops
write_stub "$STUBDIR" tmux "echo \"stub-tmux \$*\" >/dev/null; exit 0"

# Run WITHOUT DRY_RUN so the script calls the stub curl binary
PATH="${STUBDIR}:${PATH}" "$DELEGATION_COMPLETE" \
    --session-name "test-session-$$" \
    --prompt-file "$PROMPT_FILE" \
    --plan-id 42 2>&1 || true

if [[ -f "$API_LOG" ]] && grep -q "delegation\|progress\|complete\|done\|agent" "$API_LOG" 2>/dev/null; then
  pass "delegation-complete posts to delegation API"
else
  fail "delegation-complete did not call API; curl log: $(cat "$API_LOG" 2>/dev/null || echo '(empty)')"
fi
rm -rf "$TMPDIR_TEST" "$STUBDIR"

# ── test: tmux session killed ─────────────────────────────────────────────────

echo ""
echo "=== Test: tmux session cleanup ==="
TMPDIR_TEST="$(mktemp -d)"
PROMPT_FILE="${TMPDIR_TEST}/convergio-delegate-tmux.md"
echo "prompt" > "$PROMPT_FILE"
TMUX_LOG="${TMPDIR_TEST}/tmux.log"

STUBDIR="$(make_stub_dir)"
write_stub "$STUBDIR" curl 'exit 0'
write_stub "$STUBDIR" git  'exit 0'
# Stub tmux: log all invocations, also handle has-session check
write_stub "$STUBDIR" tmux "echo \"tmux \$*\" >> \"${TMUX_LOG}\"; exit 0"

PATH="${STUBDIR}:${PATH}" DRY_RUN=1 "$DELEGATION_COMPLETE" \
    --session-name "tmux-test-$$" \
    --prompt-file "$PROMPT_FILE" \
    --plan-id 0 2>&1 || true

if [[ -f "$TMUX_LOG" ]] && grep -q "kill" "$TMUX_LOG" 2>/dev/null; then
  pass "delegation-complete kills tmux session"
else
  fail "delegation-complete did not kill tmux session; tmux log: $(cat "$TMUX_LOG" 2>/dev/null || echo '(empty)')"
fi
rm -rf "$TMPDIR_TEST" "$STUBDIR"

# ── summary ──────────────────────────────────────────────────────────────────

echo ""
echo "=== Results: ${PASS} passed, ${FAIL} failed ==="
[[ $FAIL -eq 0 ]]
