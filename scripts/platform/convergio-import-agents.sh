#!/usr/bin/env bash
# convergio-import-agents.sh — Import agent catalog into ConvergioPlatform
# Reads agent .md files, copies to claude-config/agents/, populates capability registry
# Usage: convergio-import-agents.sh <source-dir> [--dry-run]
set -euo pipefail

SOURCE="${1:?Usage: convergio-import-agents.sh <source-agents-dir> [--dry-run]}"
DRY_RUN="${2:-}"
PLATFORM_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DEST="$PLATFORM_DIR/claude-config/agents"
DAEMON_URL="${CONVERGIO_DAEMON_URL:-http://localhost:8420}"

command -v curl &>/dev/null || { echo "ERROR: curl required" >&2; exit 1; }

if [ ! -d "$SOURCE" ]; then
  echo "ERROR: $SOURCE not found" >&2
  exit 1
fi

echo "=== Convergio Agent Import ==="
echo "Source: $SOURCE"
echo "Dest:   $DEST"
echo "API:    $DAEMON_URL"
[ "$DRY_RUN" = "--dry-run" ] && echo "MODE: DRY RUN"
echo ""

_api_post() { curl -sf -X POST "${DAEMON_URL}${1}" -H 'Content-Type: application/json' -d "$2" 2>/dev/null; }

# Skip files that aren't agents
SKIP_PATTERNS="README|CONSTITUTION|EXECUTION_DISCIPLINE|CommonValues|MICROSOFT_VALUES|SECURITY_FRAMEWORK_TEMPLATE"

imported=0
skipped=0
skills_added=0

find "$SOURCE" -name "*.md" -not -path "*archive*" | sort | while read -r f; do
  filename=$(basename "$f" .md)

  # Skip non-agent files
  echo "$filename" | grep -qE "$SKIP_PATTERNS" && { skipped=$((skipped + 1)); continue; }

  # Extract frontmatter
  name=$(head -15 "$f" | grep '^name:' | sed 's/name: //' | tr -d '"')
  desc=$(head -15 "$f" | grep '^description:' | sed 's/description: //' | tr -d '"' | cut -c1-200)
  model=$(head -15 "$f" | grep '^model:' | sed 's/model: //' | tr -d '"')
  tools=$(head -15 "$f" | grep '^tools:' | sed 's/tools: //')
  skills_raw=$(head -15 "$f" | grep '^skills:' | sed 's/skills: //')

  # Skip if no name
  [ -z "$name" ] && { echo "  SKIP: $filename (no name in frontmatter)"; skipped=$((skipped + 1)); continue; }

  # Determine category from directory
  category=$(basename "$(dirname "$f")")
  [ "$category" = "agents" ] && category="general"

  # Default model
  [ -z "$model" ] && model="sonnet"

  echo "  OK $name ($category) — $model"

  if [ "$DRY_RUN" != "--dry-run" ]; then
    # Copy to destination (preserve category structure)
    mkdir -p "$DEST/$category"
    cp "$f" "$DEST/$category/"

    # Insert into catalog via daemon API
    _api_post "/api/plan-db/agent/catalog" \
      "{\"name\":\"${name}\",\"category\":\"${category}\",\"description\":\"$(echo "$desc" | sed 's/"/\\"/g')\",\"model\":\"${model}\",\"tools\":\"${tools}\",\"skills\":\"${skills_raw}\",\"source_repo\":\"MyConvergio\"}" 2>/dev/null || {
      echo "    WARN: failed to write to agent catalog via API" >&2
    }

    # Extract skills from description keywords and insert
    for skill in $(echo "$desc" | tr ' ,.-' '\n' | grep -iE '^(debug|review|security|compliance|design|architecture|deploy|test|budget|strategy|analytics|marketing|sales|hr|legal|research|performance|data|devops|ux|ui|quality|validation)' | tr '[:upper:]' '[:lower:]' | sort -u); do
      _api_post "/api/plan-db/agent/skill" \
        "{\"agent_name\":\"${name}\",\"skill\":\"${skill}\",\"confidence\":0.7,\"source\":\"import\"}" 2>/dev/null || true
      skills_added=$((skills_added + 1))
    done
  fi

  imported=$((imported + 1))
done

echo ""
echo "=== Import Summary ==="
echo "  Imported: $imported agents"
echo "  Skipped:  $skipped files"
echo "  Skills:   $skills_added entries"
[ "$DRY_RUN" = "--dry-run" ] && echo "  (dry run — no files written)"
