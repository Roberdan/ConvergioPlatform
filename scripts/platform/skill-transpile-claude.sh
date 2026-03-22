#!/usr/bin/env bash
# skill-transpile-claude.sh — Transpile universal skill to Claude Code command .md
# Reads skill.yaml + SKILL.md, outputs <name>.md in Claude Code commands/ format.
# Usage: skill-transpile-claude.sh <skill-dir> [output-dir]

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
  # Reads a YAML sequence under key:, returns space-joined values
  local file="$1" key="$2"
  awk "/^${key}:/{found=1; next} found && /^[[:space:]]*-/{gsub(/^[[:space:]]*-[[:space:]]*/,\"\"); gsub(/[\"']/,\"\"); print; next} found && /^[^[:space:]]/{exit}" "$file"
}

# --- Arg validation ---
[[ $# -lt 1 ]] && die "Usage: skill-transpile-claude.sh <skill-dir> [output-dir]"

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
SKILL_ARGUMENTS="$(parse_yaml_scalar "$YAML_FILE" arguments)"

[[ -n "$SKILL_NAME" ]]    || die "skill.yaml missing 'name'"
[[ -n "$SKILL_VERSION" ]] || die "skill.yaml missing 'version'"

# Collect @reference lines present in SKILL.md
REFERENCES="$(grep -o '@reference[^ ]*' "$MD_FILE" 2>/dev/null || true)"

# --- Build output path ---
OUTPUT_FILE="${OUTPUT_DIR}/${SKILL_NAME}.md"
TMPFILE="$(mktemp)"

# --- Generate Claude Code command .md ---
{
  # YAML frontmatter
  printf -- '---\n'
  printf 'name: %s\n' "$SKILL_NAME"
  printf 'version: "%s"\n' "$SKILL_VERSION"
  printf -- '---\n'
  printf '\n'

  # HTML version comment (standard Claude command header)
  printf '<!-- v%s -->\n' "$SKILL_VERSION"
  printf '\n'

  # Title
  printf '# %s\n' "$SKILL_NAME"
  printf '\n'

  # Description (only if non-empty)
  if [[ -n "$SKILL_DESCRIPTION" ]]; then
    printf '%s\n' "$SKILL_DESCRIPTION"
    printf '\n'
  fi

  # ARGUMENTS line (only if arguments field is set and not "none")
  if [[ -n "$SKILL_ARGUMENTS" && "$SKILL_ARGUMENTS" != "none" ]]; then
    printf 'ARGUMENTS: %s\n' "$SKILL_ARGUMENTS"
    printf '\n'
  fi

  # SKILL.md body — strip its own H1 title if present to avoid duplication
  awk 'NR==1 && /^# /{next} {print}' "$MD_FILE"

  # @reference includes (append any not already in SKILL.md body)
  if [[ -n "$REFERENCES" ]]; then
    while IFS= read -r ref; do
      # Only append refs not already present verbatim in SKILL.md
      if ! grep -qF "$ref" "$MD_FILE" 2>/dev/null; then
        printf '\n%s\n' "$ref"
      fi
    done <<< "$REFERENCES"
  fi

} > "$TMPFILE"

mv "$TMPFILE" "$OUTPUT_FILE"
echo "Written: ${OUTPUT_FILE}"
