#!/usr/bin/env bash
# setup-claude-symlinks.sh — Create project-level .claude/ symlinks to claude-config/
# These are relative symlinks, portable across machines, committable to git.
# Does NOT touch ~/.claude/ (global config stays clean).
set -euo pipefail

PLATFORM_DIR="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
CLAUDE_DIR="$PLATFORM_DIR/.claude"
CONFIG_SRC="$PLATFORM_DIR/claude-config"

if [ ! -d "$CONFIG_SRC" ]; then
  echo "ERROR: $CONFIG_SRC not found." >&2
  exit 1
fi

echo "=== Project-level .claude symlinks ==="
echo "Target: $CLAUDE_DIR"
echo "Source: claude-config/ (relative)"
echo ""

# Symlink targets (relative paths for portability)
declare -A LINKS=(
  [commands]="../claude-config/commands"
  [agents]="../claude-config/agents"
  [rules]="../claude-config/rules"
  [reference]="../claude-config/reference"
  [CLAUDE.md]="../claude-config/CLAUDE.md"
)

for name in "${!LINKS[@]}"; do
  target="${LINKS[$name]}"
  link="$CLAUDE_DIR/$name"

  # Back up real file/dir (not symlink)
  if [ -e "$link" ] && [ ! -L "$link" ]; then
    mv "$link" "${link}.local-backup"
    echo "  Backed up: $name → ${name}.local-backup"
  fi

  ln -sf "$target" "$link"
  echo "  $name → $target"
done

echo ""
echo "=== Verify ==="
for name in "${!LINKS[@]}"; do
  if [ -e "$CLAUDE_DIR/$name" ]; then
    echo "  OK: $name"
  else
    echo "  BROKEN: $name" >&2
  fi
done

echo ""
echo "Done. No global ~/.claude/ files modified."
