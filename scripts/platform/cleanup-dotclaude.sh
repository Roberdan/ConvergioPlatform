#!/usr/bin/env bash
# cleanup-dotclaude.sh — Remove migrated content from ~/.claude
# RUN ONLY after verifying ConvergioPlatform works correctly
#
# Pre-flight: ensure ConvergioPlatform dashboard and daemon are functional
# Rollback: re-run the migration rsync commands to restore from git history
set -euo pipefail

echo "=== ConvergioPlatform .claude Cleanup ==="
echo "This will remove migrated content from ~/.claude"
echo "Press Ctrl+C within 10s to abort..."
sleep 10

# Step 1: Replace dashboard_web with symlink
echo "[1/4] Replacing dashboard_web with symlink..."
if [ -d ~/.claude/scripts/dashboard_web ] && [ ! -L ~/.claude/scripts/dashboard_web ]; then
  mv ~/.claude/scripts/dashboard_web ~/.claude/scripts/dashboard_web.bak
  ln -sf ~/GitHub/ConvergioPlatform/dashboard ~/.claude/scripts/dashboard_web
  echo "  OK: dashboard_web → ConvergioPlatform/dashboard"
fi

# Step 2: Replace rust/claude-core with symlink
echo "[2/4] Replacing rust/claude-core with symlink..."
if [ -d ~/.claude/rust/claude-core ] && [ ! -L ~/.claude/rust/claude-core ]; then
  mv ~/.claude/rust/claude-core ~/.claude/rust/claude-core.bak
  ln -sf ~/GitHub/ConvergioPlatform/daemon ~/.claude/rust/claude-core
  echo "  OK: claude-core → ConvergioPlatform/daemon"
fi

# Step 3: Verify symlinks work
echo "[3/4] Verifying..."
test -f ~/.claude/scripts/dashboard_web/index.html && echo "  Dashboard symlink OK" || echo "  FAIL: dashboard symlink broken"
test -f ~/.claude/rust/claude-core/Cargo.toml && echo "  Daemon symlink OK" || echo "  FAIL: daemon symlink broken"

# Step 4: Report
echo "[4/4] Size check..."
du -sh ~/.claude/
echo ""
echo "Backup dirs created:"
echo "  ~/.claude/scripts/dashboard_web.bak (safe to rm after validation)"
echo "  ~/.claude/rust/claude-core.bak (safe to rm after validation)"
echo ""
echo "To fully reclaim space, run:"
echo "  rm -rf ~/.claude/scripts/dashboard_web.bak"
echo "  rm -rf ~/.claude/rust/claude-core.bak"
