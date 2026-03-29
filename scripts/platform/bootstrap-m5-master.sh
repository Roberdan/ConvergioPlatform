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
DAEMON_URL="${CONVERGIO_DAEMON_URL:-http://localhost:8420}"

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
echo "  Symlink: .claude/data/dashboard.db -> ConvergioPlatform/data/"

# Copy rules, agents from M3 (CLAUDE.md lives at repo root, not .claude/)
echo "  Copying rules and agents from M3..."
scp -r "$M3_TS:.claude/rules/" "$CLAUDE_DIR/rules/" 2>/dev/null || true
scp -r "$M3_TS:.claude/agents/" "$CLAUDE_DIR/agents/" 2>/dev/null || true
scp -r "$M3_TS:.claude/reference/" "$CLAUDE_DIR/reference/" 2>/dev/null || true
scp -r "$M3_TS:.claude/commands/" "$CLAUDE_DIR/commands/" 2>/dev/null || true

# Symlink scripts from ConvergioPlatform (NOT copy — single source of truth)
echo "  Symlinking scripts from ConvergioPlatform..."
mkdir -p "$CLAUDE_DIR/scripts/lib"
for f in "$PLATFORM_DIR/claude-config/scripts/"*.sh; do
  ln -sf "$f" "$CLAUDE_DIR/scripts/$(basename "$f")"
done
for f in "$PLATFORM_DIR/claude-config/scripts/lib/"*.sh; do
  ln -sf "$f" "$CLAUDE_DIR/scripts/lib/$(basename "$f")"
done
echo "  OK: $(ls "$CLAUDE_DIR/scripts/"*.sh 2>/dev/null | wc -l | tr -d ' ') scripts symlinked"

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
if ! grep -q '_ZO_DOCTOR' ~/.zshenv 2>/dev/null; then
  echo 'export _ZO_DOCTOR=0' >> ~/.zshenv
  echo "  Added _ZO_DOCTOR=0 to .zshenv"
fi
if ! grep -q 'claude/scripts' ~/.zshenv 2>/dev/null; then
  echo 'export PATH="$HOME/.claude/scripts:$PATH"' >> ~/.zshenv
  echo "  Added ~/.claude/scripts to PATH in .zshenv"
fi
export DASHBOARD_DB="$PLATFORM_DIR/data/dashboard.db"

# ------ Step 6: Verify tools ------
echo "[6/9] Verifying tools..."
claude --version 2>/dev/null | head -1 && echo "  OK Claude CLI" || echo "  MISSING Claude CLI"
gh --version 2>/dev/null | head -1 && echo "  OK gh" || echo "  MISSING gh"
cargo --version 2>/dev/null && echo "  OK Rust" || echo "  MISSING Rust"
node --version 2>/dev/null && echo "  OK Node" || echo "  MISSING Node"
python3 --version 2>/dev/null && echo "  OK Python" || echo "  MISSING Python"

# ------ Step 7: Verify DB via daemon API ------
echo "[7/9] Verifying database..."
echo "  Starting daemon for verification..."
# Build and start daemon if not running
if ! curl -sf --connect-timeout 2 "${DAEMON_URL}/api/health" > /dev/null 2>&1; then
  echo "  Daemon not running — attempting to start..."
  if [ -f "$PLATFORM_DIR/daemon/start.sh" ]; then
    bash "$PLATFORM_DIR/daemon/start.sh" &
    sleep 3
  fi
fi

if curl -sf --connect-timeout 2 "${DAEMON_URL}/api/health" > /dev/null 2>&1; then
  local_json=$(curl -sf "${DAEMON_URL}/api/overview" 2>/dev/null || echo "{}")
  PLANS=$(echo "$local_json" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('total_plans', '?'))" 2>/dev/null || echo "?")
  TASKS=$(echo "$local_json" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('total_tasks', '?'))" 2>/dev/null || echo "?")
  echo "  Plans: $PLANS"
  echo "  Tasks: $TASKS"
  # Verify specific plans via cvg
  echo "  Plan 664: $(cvg plan show 664 2>/dev/null | python3 -c "import sys,json; print(json.load(sys.stdin).get('status','?'))" 2>/dev/null || echo '?')"
  echo "  Plan 659: $(cvg plan show 659 2>/dev/null | python3 -c "import sys,json; print(json.load(sys.stdin).get('status','?'))" 2>/dev/null || echo '?')"
else
  echo "  WARN: Daemon not reachable — skipping DB verification"
  echo "  Start daemon manually: cd $PLATFORM_DIR/daemon && cargo build --release && ./start.sh"
fi

# ------ Step 8: Set up LLM infrastructure ------
echo "[8/9] Setting up local LLM infrastructure..."
bash "$PLATFORM_DIR/scripts/llm/setup-llm-symlinks.sh" "$PLATFORM_DIR"
echo "  To install oMLX + LiteLLM: convergio-llm.sh setup"

# ------ Step 9: Verify plan-db.sh works ------
echo "[9/9] Verifying plan-db.sh..."
export PATH="$CLAUDE_DIR/scripts:$PATH"
plan-db.sh status convergio 2>/dev/null | head -10 || echo "  plan-db.sh needs setup"

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
