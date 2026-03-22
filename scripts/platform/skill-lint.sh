#!/usr/bin/env bash
# skill-lint.sh — CI validation script for universal skill format compliance
# Checks: schema, constitution version, token budget, name/version format
# Usage: skill-lint.sh <skill-dir> | skill-lint.sh --all <skills-dir>
# Version: 1.0.0

set -euo pipefail

# Constants
readonly MIN_CONSTITUTION_VERSION="2.0.0"
readonly TOKEN_BUDGET_BYTES=6144
readonly REQUIRED_FIELDS=("name" "version" "description" "domain" "constitution-version" "license" "copyright")

# Counters
PASS_COUNT=0
FAIL_COUNT=0

# Cleanup function (no temp files used, but required by convention)
cleanup() { :; }
trap cleanup EXIT

# Color helpers
pass() { echo "[PASS] $1"; }
fail() { echo "[FAIL] $1"; }
warn() { echo "[WARN] $1"; }

# Compare semver: returns 0 if $1 >= $2
semver_ge() {
  local ver="$1"
  local min="$2"
  local ver_major ver_minor ver_patch
  local min_major min_minor min_patch

  ver_major=$(echo "$ver" | cut -d. -f1)
  ver_minor=$(echo "$ver" | cut -d. -f2)
  ver_patch=$(echo "$ver" | cut -d. -f3)
  min_major=$(echo "$min" | cut -d. -f1)
  min_minor=$(echo "$min" | cut -d. -f2)
  min_patch=$(echo "$min" | cut -d. -f3)

  if [[ "$ver_major" -gt "$min_major" ]]; then return 0; fi
  if [[ "$ver_major" -lt "$min_major" ]]; then return 1; fi
  if [[ "$ver_minor" -gt "$min_minor" ]]; then return 0; fi
  if [[ "$ver_minor" -lt "$min_minor" ]]; then return 1; fi
  if [[ "$ver_patch" -ge "$min_patch" ]]; then return 0; fi
  return 1
}

# Extract a scalar value from a simple YAML file (key: value, no nesting)
yaml_get() {
  local file="$1"
  local key="$2"
  grep -E "^${key}:[[:space:]]" "$file" 2>/dev/null \
    | head -1 \
    | sed "s/^${key}:[[:space:]]*//" \
    | sed "s/['\"]//g" \
    | sed 's/[[:space:]]*$//'
}

# Lint a single skill directory
lint_skill() {
  local skill_dir="$1"
  local skill_name
  skill_name=$(basename "$skill_dir")
  local skill_yaml="${skill_dir}/skill.yaml"
  local skill_md="${skill_dir}/SKILL.md"
  local failed=0

  # 1. skill.yaml exists
  if [[ -f "$skill_yaml" ]]; then
    pass "${skill_name}: skill.yaml exists"
    ((PASS_COUNT++))
  else
    fail "${skill_name}: skill.yaml missing"
    ((FAIL_COUNT++))
    failed=1
  fi

  # 2. Required fields (only if skill.yaml exists)
  if [[ "$failed" -eq 0 ]]; then
    local missing_fields=()
    local field
    for field in "${REQUIRED_FIELDS[@]}"; do
      local val
      val=$(yaml_get "$skill_yaml" "$field")
      if [[ -z "$val" ]]; then
        missing_fields+=("$field")
      fi
    done
    if [[ "${#missing_fields[@]}" -eq 0 ]]; then
      pass "${skill_name}: required fields present"
      ((PASS_COUNT++))
    else
      fail "${skill_name}: required fields missing: ${missing_fields[*]}"
      ((FAIL_COUNT++))
    fi
  fi

  # 3. SKILL.md exists
  if [[ -f "$skill_md" ]]; then
    pass "${skill_name}: SKILL.md exists"
    ((PASS_COUNT++))
  else
    fail "${skill_name}: SKILL.md missing"
    ((FAIL_COUNT++))
    failed=1
  fi

  # 4. Token budget (only if SKILL.md exists)
  if [[ -f "$skill_md" ]]; then
    local byte_size
    byte_size=$(wc -c < "$skill_md" | tr -d ' ')
    if [[ "$byte_size" -le "$TOKEN_BUDGET_BYTES" ]]; then
      pass "${skill_name}: token budget (${byte_size}/${TOKEN_BUDGET_BYTES} bytes)"
      ((PASS_COUNT++))
    else
      fail "${skill_name}: SKILL.md over token budget (${byte_size}/${TOKEN_BUDGET_BYTES} bytes)"
      ((FAIL_COUNT++))
    fi
  fi

  # 5. Constitution version (only if skill.yaml exists)
  if [[ -f "$skill_yaml" ]]; then
    local const_ver
    const_ver=$(yaml_get "$skill_yaml" "constitution-version")
    if [[ -z "$const_ver" ]]; then
      fail "${skill_name}: constitution-version not set"
      ((FAIL_COUNT++))
    elif semver_ge "$const_ver" "$MIN_CONSTITUTION_VERSION"; then
      pass "${skill_name}: constitution version ${const_ver} >= ${MIN_CONSTITUTION_VERSION}"
      ((PASS_COUNT++))
    else
      fail "${skill_name}: constitution version ${const_ver} < ${MIN_CONSTITUTION_VERSION} (minimum)"
      ((FAIL_COUNT++))
    fi

    # 6. License / copyright field non-empty
    local copyright_val
    copyright_val=$(yaml_get "$skill_yaml" "copyright")
    if [[ -n "$copyright_val" ]]; then
      pass "${skill_name}: copyright present"
      ((PASS_COUNT++))
    else
      fail "${skill_name}: copyright field missing or empty"
      ((FAIL_COUNT++))
    fi

    # 7. Name format: ^[a-z][a-z0-9-]*$
    local name_val
    name_val=$(yaml_get "$skill_yaml" "name")
    if [[ -n "$name_val" ]] && echo "$name_val" | grep -qE '^[a-z][a-z0-9-]*$'; then
      pass "${skill_name}: name format valid (${name_val})"
      ((PASS_COUNT++))
    else
      fail "${skill_name}: name format invalid (got: '${name_val:-<empty>}'), must match ^[a-z][a-z0-9-]*\$"
      ((FAIL_COUNT++))
    fi

    # 8. Version format: ^[0-9]+\.[0-9]+\.[0-9]+$
    local version_val
    version_val=$(yaml_get "$skill_yaml" "version")
    if [[ -n "$version_val" ]] && echo "$version_val" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
      pass "${skill_name}: version format valid (${version_val})"
      ((PASS_COUNT++))
    else
      fail "${skill_name}: version format invalid (got: '${version_val:-<empty>}'), must be semver"
      ((FAIL_COUNT++))
    fi
  fi
}

# Usage
usage() {
  echo "Usage: $0 <skill-dir>"
  echo "       $0 --all <skills-dir>"
  exit 1
}

# Main
main() {
  if [[ $# -eq 0 ]]; then
    usage
  fi

  if [[ "$1" == "--all" ]]; then
    [[ $# -lt 2 ]] && usage
    local skills_dir="$2"
    if [[ ! -d "$skills_dir" ]]; then
      echo "ERROR: skills directory not found: ${skills_dir}" >&2
      exit 1
    fi
    local found=0
    for skill_dir in "${skills_dir}"/*/; do
      [[ -d "$skill_dir" ]] || continue
      lint_skill "${skill_dir%/}"
      found=1
    done
    if [[ "$found" -eq 0 ]]; then
      echo "No skill directories found in: ${skills_dir}" >&2
      exit 1
    fi
  else
    local skill_dir="$1"
    if [[ ! -d "$skill_dir" ]]; then
      echo "ERROR: skill directory not found: ${skill_dir}" >&2
      exit 1
    fi
    lint_skill "$skill_dir"
  fi

  echo ""
  echo "Summary: ${PASS_COUNT} passed, ${FAIL_COUNT} failed"

  if [[ "$FAIL_COUNT" -gt 0 ]]; then
    exit 1
  fi
  exit 0
}

main "$@"
