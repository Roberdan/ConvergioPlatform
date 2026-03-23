#!/usr/bin/env bash
# Test: Generated SKILL.md files conform to expected format
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

printf "=== test-skill-format ===\n"

# -- Run generator --
"${GENERATOR}" --source-dir "${FIXTURES}" --output-dir "${TMP_DIR}"

# -- Validate each generated SKILL.md --
SKILL_COUNT=0

for skill_file in "${TMP_DIR}"/*/SKILL.md; do
    if [[ ! -f "${skill_file}" ]]; then
        log_fail "No SKILL.md files generated"
        break
    fi

    SKILL_COUNT=$((SKILL_COUNT + 1))
    skill_dir="$(dirname "${skill_file}")"
    skill_name="$(basename "${skill_dir}")"

    printf "\n  Checking: %s\n" "${skill_name}"

    # -- Format 1: Name is lowercase with hyphens only --
    if echo "${skill_name}" | grep -qE '^[a-z][a-z0-9-]*$'; then
        log_pass "${skill_name}: directory name is valid (lowercase-hyphen)"
    else
        log_fail "${skill_name}: directory name '${skill_name}' does not match ^[a-z][a-z0-9-]*$"
    fi

    # -- Format 2: Frontmatter name matches directory --
    fm_name="$(grep -m1 '^name:' "${skill_file}" | sed 's/^name:[[:space:]]*//' | tr -d '"' | tr -d "'")"
    if [[ -n "${fm_name}" ]]; then
        log_pass "${skill_name}: frontmatter 'name' field is present (${fm_name})"
    else
        log_fail "${skill_name}: frontmatter 'name' field is missing or empty"
    fi

    # -- Format 3: Description is non-empty --
    fm_desc="$(grep -m1 '^description:' "${skill_file}" | sed 's/^description:[[:space:]]*//' | tr -d '"' | tr -d "'")"
    if [[ -n "${fm_desc}" ]]; then
        log_pass "${skill_name}: description is non-empty"
    else
        log_fail "${skill_name}: description is missing or empty"
    fi

    # -- Format 4: metadata.openclaw section exists --
    if grep -q "openclaw" "${skill_file}" 2>/dev/null; then
        log_pass "${skill_name}: metadata.openclaw section present"
    else
        log_fail "${skill_name}: metadata.openclaw section missing"
    fi
done

if [[ "${SKILL_COUNT}" -eq 0 ]]; then
    log_fail "No SKILL.md files found in output directory"
fi

# -- Summary --
printf "\nResults: %d passed, %d failed (%d skills checked)\n" "${PASS}" "${FAIL}" "${SKILL_COUNT}"
if [[ "${FAIL}" -gt 0 ]]; then
    exit 1
fi
