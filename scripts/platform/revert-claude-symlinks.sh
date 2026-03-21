#!/usr/bin/env bash
# revert-claude-symlinks.sh — Remove project-level .claude/ symlinks
# Restores .local-backup files if they exist.
# Does NOT touch ~/.claude/ (nothing was put there).
set -euo pipefail

PLATFORM_DIR="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
CLAUDE_DIR="$PLATFORM_DIR/.claude"

echo "=== Reverting .claude symlinks ==="

LINKS=(commands agents rules reference CLAUDE.md)

for name in "${LINKS[@]}"; do
  link="$CLAUDE_DIR/$name"

  if [ -L "$link" ]; then
    rm "$link"
    echo "  Removed: $name"

    # Restore backup if exists
    if [ -e "${link}.local-backup" ]; then
      mv "${link}.local-backup" "$link"
      echo "  Restored: ${name}.local-backup → $name"
    fi
  else
    echo "  Skip: $name (not a symlink)"
  fi
done

# Clean DASHBOARD_DB from shell profile (optional, pass --env flag)
if [[ "${2:-}" == "--env" ]]; then
  for profile in "$HOME/.zshenv" "$HOME/.bashrc"; do
    if [ -f "$profile" ] && grep -q 'DASHBOARD_DB' "$profile"; then
      sed -i.bak '/DASHBOARD_DB/d' "$profile"
      echo "  Removed DASHBOARD_DB from $profile"
    fi
  done

  # Remove global DB symlink
  if [ -L "$HOME/.claude/data/dashboard.db" ]; then
    rm "$HOME/.claude/data/dashboard.db"
    echo "  Removed ~/.claude/data/dashboard.db symlink"
  fi
fi

echo ""
echo "Done. Project .claude/ cleaned. Pass --env to also remove env vars."
