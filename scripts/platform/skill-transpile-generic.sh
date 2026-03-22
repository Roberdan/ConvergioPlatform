#!/usr/bin/env bash
# skill-transpile-generic.sh — Transpile a universal skill to a plain-text system prompt.
# Reads skill.yaml + SKILL.md from a skill directory and generates a system prompt
# usable with any LLM API (OpenAI, Anthropic, Mistral, etc.).
#
# Usage: skill-transpile-generic.sh <skill-dir> [output-dir]
#   skill-dir    Directory containing skill.yaml and SKILL.md
#   output-dir   Output directory (default: current directory)
#
# Output: <output-dir>/<name>.system-prompt.txt
#
# Examples:
#   skill-transpile-generic.sh .claude/skills/planner/ ./output/
#   skill-transpile-generic.sh /path/to/skill
set -euo pipefail

# --------------------------------------------------------------------------- #
# Cleanup                                                                       #
# --------------------------------------------------------------------------- #

_TMPFILE=""
cleanup() {
  if [[ -n "${_TMPFILE:-}" && -f "${_TMPFILE:-}" ]]; then
    rm -f "$_TMPFILE"
  fi
}
trap cleanup EXIT

# --------------------------------------------------------------------------- #
# Helpers                                                                       #
# --------------------------------------------------------------------------- #

usage() {
  sed -n '2,12s/^# \{0,1\}//p' "${BASH_SOURCE[0]}"
  exit 0
}

die() { echo "ERROR: $*" >&2; exit 1; }

# Parse a scalar value from a YAML file (key: value — single line, unquoted or quoted).
# Usage: yaml_get <file> <key>
yaml_get() {
  local file="$1"
  local key="$2"
  local raw
  raw=$(grep -E "^${key}:" "$file" | head -1 | sed -E "s/^${key}:[[:space:]]*//" | sed -E 's/^["'"'"']//; s/["'"'"']$//' | sed 's/[[:space:]]*$//')
  echo "$raw"
}

# Parse a multi-line block scalar (key: >\n  lines...) — returns joined single line.
# Fallback: returns same as yaml_get if not a block scalar.
yaml_get_block() {
  local file="$1"
  local key="$2"
  local in_block=0
  local result=""
  while IFS= read -r line; do
    if [[ "$in_block" -eq 0 ]]; then
      if echo "$line" | grep -qE "^${key}:[[:space:]]*[>|][[:space:]]*$"; then
        in_block=1
        continue
      fi
      if echo "$line" | grep -qE "^${key}:"; then
        yaml_get "$file" "$key"
        return
      fi
    else
      # Block ends at non-indented line that is not empty
      if [[ -n "$line" && ! "$line" =~ ^[[:space:]] ]]; then
        break
      fi
      result="${result} $(echo "$line" | sed 's/^[[:space:]]*//')"
    fi
  done < "$file"
  echo "${result# }"
}

# --------------------------------------------------------------------------- #
# Argument parsing                                                              #
# --------------------------------------------------------------------------- #

[[ "${1:-}" == "--help" || "${1:-}" == "-h" ]] && usage
[[ $# -lt 1 ]] && { usage; }

SKILL_DIR="${1%/}"
OUTPUT_DIR="${2:-.}"

# --------------------------------------------------------------------------- #
# Validate inputs                                                               #
# --------------------------------------------------------------------------- #

[[ -d "$SKILL_DIR" ]] || die "Skill directory not found: $SKILL_DIR"

SKILL_YAML="${SKILL_DIR}/skill.yaml"
SKILL_MD="${SKILL_DIR}/SKILL.md"

[[ -f "$SKILL_YAML" ]] || die "skill.yaml not found in: $SKILL_DIR"
[[ -f "$SKILL_MD" ]]   || die "SKILL.md not found in: $SKILL_DIR"

# --------------------------------------------------------------------------- #
# Parse skill.yaml                                                              #
# --------------------------------------------------------------------------- #

name=$(yaml_get "$SKILL_YAML" "name")
description=$(yaml_get_block "$SKILL_YAML" "description")
domain=$(yaml_get "$SKILL_YAML" "domain")
constitution_version=$(yaml_get "$SKILL_YAML" "constitution_version")
license=$(yaml_get "$SKILL_YAML" "license")

# Apply defaults for optional fields
[[ -z "$name" ]]                 && die "skill.yaml is missing required field: name"
[[ -z "$description" ]]          && die "skill.yaml is missing required field: description"
[[ -z "$domain" ]]               && domain="general"
[[ -z "$constitution_version" ]] && constitution_version="2.0.0"
[[ -z "$license" ]]              && license="MPL-2.0"

# --------------------------------------------------------------------------- #
# Parse guardrails / constraints from skill.yaml (optional list under guardrails:) #
# --------------------------------------------------------------------------- #

# Extract lines under 'guardrails:' or 'constraints:' key (list items starting with '  - ')
guardrails=""
in_section=0
while IFS= read -r line; do
  if echo "$line" | grep -qE "^(guardrails|constraints):[[:space:]]*$"; then
    in_section=1
    continue
  fi
  if [[ "$in_section" -eq 1 ]]; then
    if echo "$line" | grep -qE "^[[:space:]]+-[[:space:]]"; then
      item=$(echo "$line" | sed -E 's/^[[:space:]]+-[[:space:]]*//')
      guardrails="${guardrails}- ${item}"$'\n'
    elif [[ -n "$line" && ! "$line" =~ ^[[:space:]] ]]; then
      in_section=0
    fi
  fi
done < "$SKILL_YAML"

# --------------------------------------------------------------------------- #
# Build output                                                                  #
# --------------------------------------------------------------------------- #

mkdir -p "$OUTPUT_DIR"

# Sanitize name for use as filename (lowercase, replace spaces with hyphens)
safe_name=$(echo "$name" | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9_-]/-/g' | sed 's/-\+/-/g' | sed 's/^-//;s/-$//')
output_file="${OUTPUT_DIR}/${safe_name}.system-prompt.txt"

_TMPFILE=$(mktemp)

{
  printf 'You are %s, a %s.\n\n' "$name" "$description"
  printf 'Domain: %s\n' "$domain"
  printf 'Constitution: v%s\n' "$constitution_version"
  printf 'License: %s\n' "$license"
  printf '\n## Instructions\n\n'
  cat "$SKILL_MD"
  printf '\n\n## Constraints\n'
  printf -- '- Follow the Convergio Constitution v%s\n' "$constitution_version"
  printf -- '- %s licensed\n' "$license"
  if [[ -n "$guardrails" ]]; then
    printf '%s' "$guardrails"
  fi
} > "$_TMPFILE"

mv "$_TMPFILE" "$output_file"
_TMPFILE=""  # prevent double cleanup

echo "OK: system prompt written to ${output_file}"
