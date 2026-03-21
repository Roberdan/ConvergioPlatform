#!/usr/bin/env bash
# convergio-import-agents.sh — Import agent catalog into ConvergioPlatform
# Reads agent .md files, copies to claude-config/agents/, populates capability registry
# Usage: convergio-import-agents.sh <source-dir> [--dry-run]
set -uo pipefail

SOURCE="${1:?Usage: convergio-import-agents.sh <source-agents-dir> [--dry-run]}"
DRY_RUN="${2:-}"
PLATFORM_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DEST="$PLATFORM_DIR/claude-config/agents"
DB="${DASHBOARD_DB:-$PLATFORM_DIR/data/dashboard.db}"

if [ ! -d "$SOURCE" ]; then
  echo "ERROR: $SOURCE not found" >&2
  exit 1
fi

echo "=== Convergio Agent Import ==="
echo "Source: $SOURCE"
echo "Dest:   $DEST"
echo "DB:     $DB"
[ "$DRY_RUN" = "--dry-run" ] && echo "MODE: DRY RUN"
echo ""

# Skip files that aren't agents
SKIP_PATTERNS="README|CONSTITUTION|EXECUTION_DISCIPLINE|CommonValues|MICROSOFT_VALUES|SECURITY_FRAMEWORK_TEMPLATE"

imported=0
skipped=0
skills_added=0

# Ensure DB has skills table
if [ -f "$DB" ] && [ "$DRY_RUN" != "--dry-run" ]; then
  sqlite3 "$DB" "CREATE TABLE IF NOT EXISTS agent_catalog (
    name TEXT PRIMARY KEY,
    category TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    model TEXT NOT NULL DEFAULT 'sonnet',
    tools TEXT DEFAULT '[]',
    skills TEXT DEFAULT '[]',
    source_repo TEXT DEFAULT '',
    imported_at TEXT DEFAULT (datetime('now'))
  );" 2>/dev/null
fi

find "$SOURCE" -name "*.md" -not -path "*archive*" | sort | while read f; do
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

  echo "  ✓ $name ($category) — $model"

  if [ "$DRY_RUN" != "--dry-run" ]; then
    # Copy to destination (preserve category structure)
    mkdir -p "$DEST/$category"
    cp "$f" "$DEST/$category/"

    # Insert into catalog DB
    if [ -f "$DB" ]; then
      sqlite3 "$DB" "INSERT OR REPLACE INTO agent_catalog
        (name, category, description, model, tools, skills, source_repo)
        VALUES ('$name', '$category', '$(echo "$desc" | sed "s/'/''/g")', '$model', '$tools', '$skills_raw', 'MyConvergio');" 2>/dev/null

      # Extract skills from description keywords and insert
      # Simple heuristic: words after key patterns
      for skill in $(echo "$desc" | tr ' ,.-' '\n' | grep -iE '^(debug|review|security|compliance|design|architecture|deploy|test|budget|strategy|analytics|marketing|sales|hr|legal|research|performance|data|devops|ux|ui|quality|validation)' | tr '[:upper:]' '[:lower:]' | sort -u); do
        sqlite3 "$DB" "INSERT OR IGNORE INTO ipc_agent_skills
          (agent_name, skill, confidence, source)
          VALUES ('$name', '$skill', 0.7, 'import');" 2>/dev/null
        skills_added=$((skills_added + 1))
      done
    fi
  fi

  imported=$((imported + 1))
done

echo ""
echo "=== Import Summary ==="
echo "  Imported: $imported agents"
echo "  Skipped:  $skipped files"
echo "  Skills:   $skills_added entries"
[ "$DRY_RUN" = "--dry-run" ] && echo "  (dry run — no files written)"
