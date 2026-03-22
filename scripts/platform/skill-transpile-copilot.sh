#!/usr/bin/env bash
# skill-transpile-copilot.sh — Transpile universal skill to Copilot CLI agent .md
# Reads skill.yaml + SKILL.md, outputs <name>.agent.md in Copilot CLI format.
# Usage: skill-transpile-copilot.sh <skill-dir> [output-dir]

set -euo pipefail

# --- Cleanup ---
cleanup() {
  rm -f "${TMPFILE:-}"
}
trap cleanup EXIT

# --- Helpers ---
die() { echo "ERROR: $1" >&2; exit 1; }

parse_yaml_scalar() {
  local file="$1" key="$2"
  # Matches: key: value or key: "value" (single-level, no nesting)
  grep -m1 "^${key}:" "$file" \
    | sed "s/^${key}:[[:space:]]*//" \
    | sed 's/^"//' \
    | sed 's/"$//' \
    | sed "s/^'//" \
    | sed "s/'$//" \
    | tr -d '\r'
}

parse_yaml_list() {
  # Reads a YAML sequence under key:, returns newline-separated values
  local file="$1" key="$2"
  awk "/^${key}:/{found=1; next} found && /^[[:space:]]*-/{gsub(/^[[:space:]]*-[[:space:]]*/,\"\"); gsub(/[\"']/,\"\"); print; next} found && /^[^[:space:]]/{exit}" "$file"
}

# --- Capitalise first letter of a string (bash 3.2 compatible) ---
capitalise() {
  local str="$1"
  local first rest
  first="$(echo "${str:0:1}" | tr '[:lower:]' '[:upper:]')"
  rest="${str:1}"
  echo "${first}${rest}"
}

# --- Arg validation ---
[[ $# -lt 1 ]] && die "Usage: skill-transpile-copilot.sh <skill-dir> [output-dir]"

SKILL_DIR="${1%/}"
OUTPUT_DIR="${2:-$(pwd)}"

YAML_FILE="${SKILL_DIR}/skill.yaml"
MD_FILE="${SKILL_DIR}/SKILL.md"

[[ -f "$YAML_FILE" ]] || die "Missing: ${YAML_FILE}"
[[ -f "$MD_FILE"   ]] || die "Missing: ${MD_FILE}"
[[ -d "$OUTPUT_DIR" ]] || die "Output dir not found: ${OUTPUT_DIR}"

# --- Parse skill.yaml ---
SKILL_NAME="$(parse_yaml_scalar "$YAML_FILE" name)"
SKILL_VERSION="$(parse_yaml_scalar "$YAML_FILE" version)"
SKILL_DESCRIPTION="$(parse_yaml_scalar "$YAML_FILE" description)"
SKILL_MODEL="$(parse_yaml_scalar "$YAML_FILE" model)"

[[ -n "$SKILL_NAME" ]]    || die "skill.yaml missing 'name'"
[[ -n "$SKILL_VERSION" ]] || die "skill.yaml missing 'version'"

# Collect tools as comma-separated list for Copilot frontmatter
TOOLS_RAW="$(parse_yaml_list "$YAML_FILE" tools)"
TOOLS_CSV=""
if [[ -n "$TOOLS_RAW" ]]; then
  # Build ['tool1', 'tool2', ...] array notation used by Copilot agent frontmatter
  while IFS= read -r tool; do
    [[ -z "$tool" ]] && continue
    tool_lc="$(echo "$tool" | tr '[:upper:]' '[:lower:]')"
    if [[ -n "$TOOLS_CSV" ]]; then
      TOOLS_CSV="${TOOLS_CSV}, '${tool_lc}'"
    else
      TOOLS_CSV="'${tool_lc}'"
    fi
  done <<< "$TOOLS_RAW"
fi

# --- Build output path ---
OUTPUT_FILE="${OUTPUT_DIR}/${SKILL_NAME}.agent.md"
TMPFILE="$(mktemp)"

# --- Generate Copilot CLI agent .md ---
{
  # YAML frontmatter
  printf -- '---\n'
  printf 'name: %s\n' "$SKILL_NAME"
  if [[ -n "$SKILL_DESCRIPTION" ]]; then
    printf 'description: %s\n' "$SKILL_DESCRIPTION"
  fi
  if [[ -n "$TOOLS_CSV" ]]; then
    printf 'tools: [%s]\n' "$TOOLS_CSV"
  fi
  if [[ -n "$SKILL_MODEL" ]]; then
    printf 'model: %s\n' "$SKILL_MODEL"
  fi
  printf -- '---\n'
  printf '\n'

  # Title using capitalised skill name
  printf '# %s\n' "$(capitalise "$SKILL_NAME")"
  printf '\n'

  # SKILL.md body — strip its own H1 title if present to avoid duplication
  awk 'NR==1 && /^# /{next} {print}' "$MD_FILE"

} > "$TMPFILE"

mv "$TMPFILE" "$OUTPUT_FILE"
echo "Written: ${OUTPUT_FILE}"
