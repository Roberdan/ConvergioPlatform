#!/usr/bin/env bash
# Test: convergio-openclaw-skills.sh generates valid skill files
# Requires: scripts/platform/convergio-openclaw-skills.sh (created by T3-01)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
GENERATOR="${PROJECT_ROOT}/scripts/platform/convergio-openclaw-skills.sh"
FIXTURES="${PROJECT_ROOT}/tests/fixtures/openclaw"

PASS=0
FAIL=0
TMP_DIR=""

cleanup() {
    if [[ -n "${TMP_DIR}" && -d "${TMP_DIR}" ]]; then
        rm -rf "${TMP_DIR}"
    fi
}
trap cleanup EXIT

log_pass() {
    local msg="$1"
    PASS=$((PASS + 1))
    printf "  PASS: %s\n" "${msg}"
}

log_fail() {
    local msg="$1"
    FAIL=$((FAIL + 1))
    printf "  FAIL: %s\n" "${msg}" >&2
}

# -- Pre-flight --
if [[ ! -x "${GENERATOR}" ]]; then
    printf "SKIP: Generator not found at %s (parallel task T3-01 not complete)\n" "${GENERATOR}"
    exit 0
fi

if [[ ! -f "${FIXTURES}/test-agent.agent.md" ]]; then
    printf "ERROR: Test fixture missing at %s\n" "${FIXTURES}/test-agent.agent.md" >&2
    exit 1
fi

# -- Setup --
TMP_DIR="$(mktemp -d)"

printf "=== test-skill-generator ===\n"

# -- Run generator --
"${GENERATOR}" --source-dir "${FIXTURES}" --output-dir "${TMP_DIR}"

# -- Test 1: SKILL.md exists --
if [[ -f "${TMP_DIR}/test-reviewer/SKILL.md" ]]; then
    log_pass "test-reviewer/SKILL.md exists"
else
    log_fail "test-reviewer/SKILL.md not found in output"
fi

# -- Test 2: Frontmatter contains name --
if grep -qE 'name:[[:space:]]+"?test-reviewer"?' "${TMP_DIR}/test-reviewer/SKILL.md" 2>/dev/null; then
    log_pass "Frontmatter contains 'name: test-reviewer'"
else
    log_fail "Frontmatter missing 'name: test-reviewer'"
fi

# -- Test 3: Body references OpenClaw invoke endpoint --
if grep -q "api/openclaw/invoke" "${TMP_DIR}/test-reviewer/SKILL.md" 2>/dev/null; then
    log_pass "Body contains 'api/openclaw/invoke'"
else
    log_fail "Body missing 'api/openclaw/invoke'"
fi

# -- Test 4: index.json exists --
if [[ -f "${TMP_DIR}/index.json" ]]; then
    log_pass "index.json exists"
else
    log_fail "index.json not found in output"
fi

# -- Test 5: index.json contains agent name --
if grep -q "test-reviewer" "${TMP_DIR}/index.json" 2>/dev/null; then
    log_pass "index.json contains 'test-reviewer'"
else
    log_fail "index.json missing 'test-reviewer'"
fi

# -- Summary --
printf "\nResults: %d passed, %d failed\n" "${PASS}" "${FAIL}"
if [[ "${FAIL}" -gt 0 ]]; then
    exit 1
fi
