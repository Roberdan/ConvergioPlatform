#!/usr/bin/env bash
# convergio-openclaw-skills.sh — Generate OpenClaw SKILL.md files from Convergio .agent.md definitions
# Usage: convergio-openclaw-skills.sh [--source-dir DIR] [--output-dir DIR] [--help]

set -euo pipefail

# --- Constants ---
DEFAULT_SOURCE_DIR="claude-config/agents"
DEFAULT_OUTPUT_DIR="integrations/openclaw-bridge/skills"
SCRIPT_NAME="$(basename "$0")"
VERSION="0.1.0"

# --- Cleanup ---
TMPFILES=()
cleanup() {
  if [[ ${#TMPFILES[@]} -gt 0 ]]; then
    for f in "${TMPFILES[@]}"; do
      rm -f "$f"
    done
  fi
}
trap cleanup EXIT

# --- Helpers ---
die() { echo "ERROR: $1" >&2; exit 1; }

usage() {
  cat <<EOF
Usage: $SCRIPT_NAME [--source-dir DIR] [--output-dir DIR] [--help]

Generate OpenClaw SKILL.md files from Convergio .agent.md definitions.

Options:
  --source-dir DIR   Directory containing *.agent.md files (default: $DEFAULT_SOURCE_DIR)
  --output-dir DIR   Output directory for generated skills (default: $DEFAULT_OUTPUT_DIR)
  --help             Show this usage message and exit

Each .agent.md with YAML frontmatter (name, description) produces:
  OUTPUT_DIR/name/SKILL.md   — OpenClaw skill definition
  OUTPUT_DIR/index.json       — JSON index of all generated skills
EOF
  exit 0
}

# parse_frontmatter_field — extract a scalar from YAML frontmatter (between --- markers)
# Only reads lines between the first and second --- delimiters
parse_frontmatter_field() {
  local file="$1" key="$2"
  local in_frontmatter=0
  local line_num=0

  while IFS= read -r line; do
    line_num=$((line_num + 1))
    if [[ "$line" =~ ^---[[:space:]]*$ ]]; then
      if [[ "$in_frontmatter" -eq 1 ]]; then
        break
      fi
      in_frontmatter=1
      continue
    fi
    if [[ "$in_frontmatter" -eq 1 ]] && [[ "$line" =~ ^${key}:[[:space:]]*(.*) ]]; then
      local value="${BASH_REMATCH[1]}"
      # Strip surrounding quotes
      value="${value#\"}"
      value="${value%\"}"
      value="${value#\'}"
      value="${value%\'}"
      printf '%s' "$value"
      return 0
    fi
  done < "$file"
  return 1
}

# extract_body — get markdown body after frontmatter closing ---
extract_body() {
  local file="$1"
  local fence_count=0

  while IFS= read -r line; do
    if [[ "$line" =~ ^---[[:space:]]*$ ]]; then
      fence_count=$((fence_count + 1))
      if [[ "$fence_count" -ge 2 ]]; then
        # Output remaining lines
        cat
        return 0
      fi
      continue
    fi
  done < "$file"
  return 1
}

# generate_skill — create SKILL.md for one agent
generate_skill() {
  local agent_file="$1" output_dir="$2"
  local name description skill_dir skill_file

  name="$(parse_frontmatter_field "$agent_file" "name")" || return 1
  description="$(parse_frontmatter_field "$agent_file" "description")" || description=""

  if [[ -z "$name" ]]; then
    echo "WARN: skipping $agent_file — no name in frontmatter" >&2
    return 1
  fi

  skill_dir="${output_dir}/${name}"
  skill_file="${skill_dir}/SKILL.md"
  mkdir -p "$skill_dir"

  cat > "$skill_file" <<SKILLEOF
---
name: "${name}"
description: "${description}"
version: "${VERSION}"
metadata:
  openclaw:
    requires:
      bins:
        - curl
        - jq
    primaryEnv: "CONVERGIO_DAEMON_URL"
---

# ${name}

${description}

## Invocation

\`\`\`bash
curl -s -X POST "\${CONVERGIO_DAEMON_URL:-http://localhost:8420}/api/openclaw/invoke" \\
  -H "Content-Type: application/json" \\
  -d '{
    "skill": "${name}",
    "input": {}
  }' | jq .
\`\`\`
SKILLEOF

  echo "  generated: ${skill_file}" >&2
}

# generate_index — create index.json listing all skills
generate_index() {
  local output_dir="$1"
  shift
  local names=("$@")
  local index_file="${output_dir}/index.json"
  local first=1

  printf '[\n' > "$index_file"
  for entry in "${names[@]}"; do
    local n d p
    n="${entry%%|*}"
    local rest="${entry#*|}"
    d="${rest%%|*}"
    p="${rest#*|}"

    if [[ "$first" -eq 1 ]]; then
      first=0
    else
      printf ',\n' >> "$index_file"
    fi
    # Escape double quotes in description for JSON safety
    d="${d//\\/\\\\}"
    d="${d//\"/\\\"}"
    printf '  {"name": "%s", "description": "%s", "path": "%s"}' "$n" "$d" "$p" >> "$index_file"
  done
  printf '\n]\n' >> "$index_file"

  echo "  generated: ${index_file}" >&2
}

# --- Arg parsing ---
SOURCE_DIR="$DEFAULT_SOURCE_DIR"
OUTPUT_DIR="$DEFAULT_OUTPUT_DIR"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --source-dir)
      [[ $# -lt 2 ]] && die "--source-dir requires an argument"
      SOURCE_DIR="$2"
      shift 2
      ;;
    --output-dir)
      [[ $# -lt 2 ]] && die "--output-dir requires an argument"
      OUTPUT_DIR="$2"
      shift 2
      ;;
    --help)
      usage
      ;;
    *)
      die "Unknown option: $1. Use --help for usage."
      ;;
  esac
done

# --- Validate ---
[[ -d "$SOURCE_DIR" ]] || die "Source directory not found: $SOURCE_DIR"

# --- Main ---
echo "OpenClaw skill generation" >&2
echo "  source: $SOURCE_DIR" >&2
echo "  output: $OUTPUT_DIR" >&2

mkdir -p "$OUTPUT_DIR"

# Collect agent files
AGENT_FILES=()
while IFS= read -r -d '' f; do
  AGENT_FILES+=("$f")
done < <(find "$SOURCE_DIR" -name '*.agent.md' -print0 | sort -z)

if [[ ${#AGENT_FILES[@]} -eq 0 ]]; then
  die "No *.agent.md files found in $SOURCE_DIR"
fi

echo "  found: ${#AGENT_FILES[@]} agent(s)" >&2

# Generate skills and collect index entries
INDEX_ENTRIES=()
GENERATED=0

for agent_file in "${AGENT_FILES[@]}"; do
  local_name="$(parse_frontmatter_field "$agent_file" "name" 2>/dev/null)" || continue
  local_desc="$(parse_frontmatter_field "$agent_file" "description" 2>/dev/null)" || local_desc=""

  if generate_skill "$agent_file" "$OUTPUT_DIR"; then
    INDEX_ENTRIES+=("${local_name}|${local_desc}|${local_name}/SKILL.md")
    GENERATED=$((GENERATED + 1))
  fi
done

if [[ "$GENERATED" -eq 0 ]]; then
  die "No skills generated — check that agent files have valid YAML frontmatter"
fi

# Generate index
generate_index "$OUTPUT_DIR" "${INDEX_ENTRIES[@]}"

echo "Done: ${GENERATED} skill(s) generated" >&2
