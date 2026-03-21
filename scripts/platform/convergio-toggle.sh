#!/usr/bin/env bash
# convergio-toggle.sh — Enable/disable Convergio as GLOBAL ~/.claude overlay
# Usage: convergio-toggle.sh on|off|status
#
# ON:  Creates symlinks in ~/.claude/ → ConvergioPlatform/claude-config/
#      This makes /planner, /execute, Thor, etc. available in ANY repo.
#      Repo-specific .claude/ config layers on TOP (Claude merges both).
# OFF: Removes symlinks, restores backups. Clean Claude/Copilot.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLATFORM_DIR="${CONVERGIO_PLATFORM_DIR:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
CLAUDE_DIR="$HOME/.claude"
CONFIG_SRC="$PLATFORM_DIR/claude-config"

# What gets symlinked globally
LINKS=(commands agents rules reference)
# CLAUDE.md stays project-level (each repo has its own)

status() {
  local active=0
  for name in "${LINKS[@]}"; do
    if [ -L "$CLAUDE_DIR/$name" ]; then
      active=$((active + 1))
    fi
  done

  if [ "$active" -eq "${#LINKS[@]}" ]; then
    echo "convergio: ON ($active/${#LINKS[@]} global symlinks)"
    echo "  Platform: $PLATFORM_DIR"
    return 0
  elif [ "$active" -eq 0 ]; then
    echo "convergio: OFF"
    return 1
  else
    echo "convergio: PARTIAL ($active/${#LINKS[@]})"
    for name in "${LINKS[@]}"; do
      if [ -L "$CLAUDE_DIR/$name" ]; then
        echo "  ON:  $name → $(readlink "$CLAUDE_DIR/$name")"
      else
        echo "  OFF: $name"
      fi
    done
    return 2
  fi
}

enable() {
  if [ ! -d "$CONFIG_SRC" ]; then
    echo "ERROR: $CONFIG_SRC not found." >&2
    exit 1
  fi

  for name in "${LINKS[@]}"; do
    local link="$CLAUDE_DIR/$name"

    # Back up existing real file/dir (not symlink)
    if [ -e "$link" ] && [ ! -L "$link" ]; then
      mv "$link" "${link}.convergio-backup"
      echo "  Backed up: $name → ${name}.convergio-backup"
    fi

    # Absolute symlink (cross-filesystem safe)
    ln -sf "$CONFIG_SRC/$name" "$link"
  done

  # Also ensure project-level .claude/ in ConvergioPlatform has its symlinks
  local proj_claude="$PLATFORM_DIR/.claude"
  for name in commands agents rules reference CLAUDE.md; do
    if [ ! -L "$proj_claude/$name" ] && [ ! -e "$proj_claude/$name" ]; then
      ln -sf "../claude-config/$name" "$proj_claude/$name"
    fi
  done

  echo "convergio: ON — workflow available in ALL repos"
  echo "  /planner /execute /prompt /check + Thor + agents"
}

disable() {
  for name in "${LINKS[@]}"; do
    local link="$CLAUDE_DIR/$name"

    if [ -L "$link" ]; then
      rm "$link"

      # Restore backup if exists
      if [ -e "${link}.convergio-backup" ]; then
        mv "${link}.convergio-backup" "$link"
        echo "  Restored: $name"
      fi
    fi
  done

  echo "convergio: OFF — clean Claude/Copilot"
}

case "${1:-status}" in
  on|enable)   enable ;;
  off|disable) disable ;;
  status)      status ;;
  *)
    echo "Usage: convergio-toggle.sh on|off|status"
    echo "  on     Convergio overlay in ~/.claude/ (works in any repo)"
    echo "  off    Remove overlay (clean Claude/Copilot)"
    echo "  status Show current state"
    exit 1
    ;;
esac
