#!/usr/bin/env bash
# setup-claude-symlinks.sh — Make .claude a thin layer pointing to ConvergioPlatform
# Run on any mesh node after cloning ConvergioPlatform
# Preserves: machine-local data (projects/, config/peers.conf, data/)
# Symlinks: rules, agents, commands, reference, scripts, CLAUDE.md
set -euo pipefail

PLATFORM_DIR="${1:-$HOME/GitHub/ConvergioPlatform}"
CLAUDE_DIR="$HOME/.claude"
CONFIG_SRC="$PLATFORM_DIR/claude-config"

if [ ! -d "$CONFIG_SRC" ]; then
  echo "ERROR: $CONFIG_SRC not found. Clone ConvergioPlatform first."
  exit 1
fi

echo "=== Setting up .claude symlinks ==="
echo "Source: $CONFIG_SRC"
echo "Target: $CLAUDE_DIR"
echo ""

mkdir -p "$CLAUDE_DIR/data"
mkdir -p "$CLAUDE_DIR/config"
mkdir -p "$CLAUDE_DIR/projects"

# --- CLAUDE.md ---
if [ -f "$CLAUDE_DIR/CLAUDE.md" ] && [ ! -L "$CLAUDE_DIR/CLAUDE.md" ]; then
  mv "$CLAUDE_DIR/CLAUDE.md" "$CLAUDE_DIR/CLAUDE.md.local-backup"
fi
ln -sf "$CONFIG_SRC/CLAUDE.md" "$CLAUDE_DIR/CLAUDE.md"
echo "  CLAUDE.md → symlink"

# --- rules/ ---
if [ -d "$CLAUDE_DIR/rules" ] && [ ! -L "$CLAUDE_DIR/rules" ]; then
  mv "$CLAUDE_DIR/rules" "$CLAUDE_DIR/rules.local-backup"
fi
ln -sf "$CONFIG_SRC/rules" "$CLAUDE_DIR/rules"
echo "  rules/ → symlink"

# --- agents/ ---
if [ -d "$CLAUDE_DIR/agents" ] && [ ! -L "$CLAUDE_DIR/agents" ]; then
  mv "$CLAUDE_DIR/agents" "$CLAUDE_DIR/agents.local-backup"
fi
ln -sf "$CONFIG_SRC/agents" "$CLAUDE_DIR/agents"
echo "  agents/ → symlink"

# --- commands/ ---
if [ -d "$CLAUDE_DIR/commands" ] && [ ! -L "$CLAUDE_DIR/commands" ]; then
  mv "$CLAUDE_DIR/commands" "$CLAUDE_DIR/commands.local-backup"
fi
ln -sf "$CONFIG_SRC/commands" "$CLAUDE_DIR/commands"
echo "  commands/ → symlink"

# --- reference/ ---
if [ -d "$CLAUDE_DIR/reference" ] && [ ! -L "$CLAUDE_DIR/reference" ]; then
  mv "$CLAUDE_DIR/reference" "$CLAUDE_DIR/reference.local-backup"
fi
ln -sf "$CONFIG_SRC/reference" "$CLAUDE_DIR/reference"
echo "  reference/ → symlink"

# --- scripts/ ---
if [ -d "$CLAUDE_DIR/scripts" ] && [ ! -L "$CLAUDE_DIR/scripts" ]; then
  mv "$CLAUDE_DIR/scripts" "$CLAUDE_DIR/scripts.local-backup"
fi
ln -sf "$CONFIG_SRC/scripts" "$CLAUDE_DIR/scripts"
echo "  scripts/ → symlink"

# --- data/dashboard.db ---
ln -sf "$PLATFORM_DIR/data/dashboard.db" "$CLAUDE_DIR/data/dashboard.db"
echo "  data/dashboard.db → symlink"

# --- DASHBOARD_DB env ---
if ! grep -q 'DASHBOARD_DB' ~/.zshenv 2>/dev/null; then
  echo "export DASHBOARD_DB=\"$PLATFORM_DIR/data/dashboard.db\"" >> ~/.zshenv
  echo "  DASHBOARD_DB added to .zshenv"
fi

echo ""
echo "=== Verify ==="
echo ".claude contents:"
ls -la "$CLAUDE_DIR/" | grep -E '^l|CLAUDE'

echo ""
echo "Machine-local (NOT symlinked):"
echo "  data/     — DB, audit, learnings (per-machine)"
echo "  config/   — peers.conf (per-machine)"
echo "  projects/ — per-project overlays (per-machine)"

echo ""
echo "=== To update all nodes ==="
echo "  cd ~/GitHub/ConvergioPlatform && git pull"
echo "  All symlinked config updates automatically."

echo ""
echo "Done. .claude is now a thin layer over ConvergioPlatform."
