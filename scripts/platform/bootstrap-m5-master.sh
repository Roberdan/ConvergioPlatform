#!/usr/bin/env bash
# bootstrap-m5-master.sh — Set up M5 Max as new ConvergioPlatform master
# Run this ON the M5 Mac (via SSH or terminal)
set -euo pipefail

_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../../config/load-config.sh
source "$_SCRIPT_DIR/../../config/load-config.sh" 2>/dev/null || true
unset _SCRIPT_DIR

[[ -z "${NODE_M3_HOST:-}" ]] && { echo "ERROR: NODE_M3_HOST not set. Run: cp config/local.env.example config/local.env" >&2; exit 1; }
M3_TS="${NODE_M3_HOST}"
PLATFORM_DIR="$HOME/GitHub/ConvergioPlatform"
CLAUDE_DIR="$HOME/.claude"

echo "=== ConvergioPlatform M5 Master Bootstrap ==="
echo "Target: $(hostname) ($(scutil --get ComputerName 2>/dev/null))"
echo ""

# Ensure PATH
export PATH="/opt/homebrew/bin:/opt/homebrew/sbin:$HOME/.claude/scripts:$PATH"

# ------ Step 1: Clone ConvergioPlatform ------
echo "[1/9] Cloning ConvergioPlatform..."
if [ -d "$PLATFORM_DIR" ]; then
  echo "  Already exists — pulling latest"
  cd "$PLATFORM_DIR" && git pull origin main
else
  cd ~/GitHub
  gh repo clone Roberdan/ConvergioPlatform
fi
echo "  OK: $PLATFORM_DIR"

# ------ Step 2: Copy dashboard.db from M3 ------
echo "[2/9] Copying dashboard.db from M3..."
mkdir -p "$PLATFORM_DIR/data"
scp "$M3_TS:GitHub/ConvergioPlatform/data/dashboard.db" "$PLATFORM_DIR/data/dashboard.db"
echo "  OK: $(du -sh "$PLATFORM_DIR/data/dashboard.db" | cut -f1)"

# Copy supporting data
scp "$M3_TS:GitHub/ConvergioPlatform/data/session-learnings.jsonl" "$PLATFORM_DIR/data/" 2>/dev/null || true
scp "$M3_TS:GitHub/ConvergioPlatform/data/thor-audit.jsonl" "$PLATFORM_DIR/data/" 2>/dev/null || true

# ------ Step 3: Set up .claude essentials ------
echo "[3/9] Setting up .claude config..."
mkdir -p "$CLAUDE_DIR/data" "$CLAUDE_DIR/rules" "$CLAUDE_DIR/agents" "$CLAUDE_DIR/scripts" "$CLAUDE_DIR/reference"

# Symlink DB into .claude for backward compat
ln -sf "$PLATFORM_DIR/data/dashboard.db" "$CLAUDE_DIR/data/dashboard.db"
echo "  Symlink: .claude/data/dashboard.db → ConvergioPlatform/data/"

# Copy rules, agents, CLAUDE.md from M3
echo "  Copying rules and agents from M3..."
scp "$M3_TS:.claude/CLAUDE.md" "$CLAUDE_DIR/CLAUDE.md"
scp -r "$M3_TS:.claude/rules/" "$CLAUDE_DIR/rules/" 2>/dev/null || true
scp -r "$M3_TS:.claude/agents/" "$CLAUDE_DIR/agents/" 2>/dev/null || true
scp -r "$M3_TS:.claude/reference/" "$CLAUDE_DIR/reference/" 2>/dev/null || true
scp -r "$M3_TS:.claude/commands/" "$CLAUDE_DIR/commands/" 2>/dev/null || true

# Copy scripts (digest/hook tooling only — not dashboard or rust)
echo "  Copying scripts from M3..."
scp -r "$M3_TS:.claude/scripts/*.sh" "$CLAUDE_DIR/scripts/" 2>/dev/null || true
scp -r "$M3_TS:.claude/scripts/lib/" "$CLAUDE_DIR/scripts/lib/" 2>/dev/null || true
scp -r "$M3_TS:.claude/scripts/archive/" "$CLAUDE_DIR/scripts/archive/" 2>/dev/null || true

# ------ Step 4: Set up peers.conf ------
echo "[4/9] Setting up mesh config..."
mkdir -p "$CLAUDE_DIR/config"
scp "$M3_TS:.claude/config/peers.conf" "$CLAUDE_DIR/config/peers.conf"
echo "  OK: peers.conf copied"

# ------ Step 5: Set DASHBOARD_DB env var ------
echo "[5/9] Setting environment..."
if ! grep -q 'DASHBOARD_DB' ~/.zshenv 2>/dev/null; then
  echo "export DASHBOARD_DB=\"$PLATFORM_DIR/data/dashboard.db\"" >> ~/.zshenv
  echo "  Added DASHBOARD_DB to .zshenv"
fi
export DASHBOARD_DB="$PLATFORM_DIR/data/dashboard.db"

# ------ Step 6: Verify tools ------
echo "[6/9] Verifying tools..."
claude --version 2>/dev/null | head -1 && echo "  ✅ Claude CLI" || echo "  ❌ Claude CLI missing"
gh --version 2>/dev/null | head -1 && echo "  ✅ gh" || echo "  ❌ gh missing"
cargo --version 2>/dev/null && echo "  ✅ Rust" || echo "  ❌ Rust missing"
node --version 2>/dev/null && echo "  ✅ Node" || echo "  ❌ Node missing"
python3 --version 2>/dev/null && echo "  ✅ Python" || echo "  ❌ Python missing"

# ------ Step 7: Verify DB ------
echo "[7/9] Verifying database..."
PLANS=$(sqlite3 "$PLATFORM_DIR/data/dashboard.db" "SELECT count(*) FROM plans;" 2>/dev/null)
TASKS=$(sqlite3 "$PLATFORM_DIR/data/dashboard.db" "SELECT count(*) FROM tasks;" 2>/dev/null)
echo "  Plans: $PLANS"
echo "  Tasks: $TASKS"
echo "  Plan 664: $(sqlite3 "$PLATFORM_DIR/data/dashboard.db" "SELECT status FROM plans WHERE id=664;")"
echo "  Plan 659: $(sqlite3 "$PLATFORM_DIR/data/dashboard.db" "SELECT status FROM plans WHERE id=659;")"

# ------ Step 8: Set up LLM infrastructure ------
echo "[8/9] Setting up local LLM infrastructure..."
bash "$PLATFORM_DIR/scripts/llm/setup-llm-symlinks.sh" "$PLATFORM_DIR"
echo "  To install oMLX + LiteLLM: convergio-llm.sh setup"

# ------ Step 9: Verify plan-db.sh works ------
echo "[9/9] Verifying plan-db.sh..."
export PATH="$CLAUDE_DIR/scripts:$PATH"
plan-db.sh status convergio 2>/dev/null | head -10 || echo "  ⚠️ plan-db.sh needs setup"

echo ""
echo "========================================="
echo "  M5 Master Bootstrap COMPLETE"
echo "========================================="
echo ""
echo "  ConvergioPlatform: $PLATFORM_DIR"
echo "  Dashboard DB:      $PLATFORM_DIR/data/dashboard.db"
echo "  .claude config:    $CLAUDE_DIR (config-only)"
echo "  Mesh role:         coordinator"
echo ""
echo "  Next steps:"
echo "    1. Install Copilot CLI if needed"
echo "    2. Run: cd $PLATFORM_DIR/daemon && cargo build --release"
echo "    3. Run: cd $PLATFORM_DIR/dashboard && ./start.sh"
echo "    4. Start Plan 659: plan-db.sh start 659"
echo ""
