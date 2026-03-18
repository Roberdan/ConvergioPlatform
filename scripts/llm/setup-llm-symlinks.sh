#!/usr/bin/env bash
# setup-llm-symlinks.sh — Wire LLM infrastructure from ConvergioPlatform to system paths
# Run after cloning ConvergioPlatform. Safe to re-run (idempotent).
#
# Creates symlinks:
#   ~/bin/convergio-llm.sh          → scripts/llm/convergio-llm.sh
#   ~/bin/convergio-llm-download.sh → scripts/llm/convergio-llm-download.sh
#   ~/bin/convergio-llm-setup.sh    → scripts/llm/convergio-llm-setup.sh
#   ~/.continue/config.json         → config/llm/continue-config.json
#
# Also ensures .zshrc/.bashrc has the claude-local function.
set -euo pipefail

PLATFORM_DIR="${1:-$HOME/GitHub/ConvergioPlatform}"
LLM_SCRIPTS="$PLATFORM_DIR/scripts/llm"
LLM_CONFIG="$PLATFORM_DIR/config/llm"

if [ ! -d "$LLM_SCRIPTS" ]; then
  echo "ERROR: $LLM_SCRIPTS not found. Clone ConvergioPlatform first."
  exit 1
fi

echo "=== Setting up Convergio LLM symlinks ==="
echo "Source: $PLATFORM_DIR"
echo ""

# --- Helper: safe symlink (backup existing non-symlinks) ---
safe_link() {
  local src="$1" dst="$2" label="$3"
  if [ -f "$dst" ] && [ ! -L "$dst" ]; then
    mv "$dst" "${dst}.backup-$(date +%Y%m%d)"
    echo "  $label: backed up existing file"
  fi
  ln -sf "$src" "$dst"
  echo "  $label → symlink"
}

# --- 1. CLI scripts in ~/bin ---
mkdir -p "$HOME/bin"
safe_link "$LLM_SCRIPTS/convergio-llm.sh" "$HOME/bin/convergio-llm.sh" "convergio-llm.sh"
safe_link "$LLM_SCRIPTS/convergio-llm-download.sh" "$HOME/bin/convergio-llm-download.sh" "convergio-llm-download.sh"
safe_link "$LLM_SCRIPTS/convergio-llm-setup.sh" "$HOME/bin/convergio-llm-setup.sh" "convergio-llm-setup.sh"

# --- 2. Continue.dev config ---
mkdir -p "$HOME/.continue"
safe_link "$LLM_CONFIG/continue-config.json" "$HOME/.continue/config.json" "Continue.dev config"

# --- 3. Copilot instructions (already in .github/) ---
if [ -f "$PLATFORM_DIR/.github/copilot-instructions.md" ]; then
  echo "  Copilot instructions: OK (in .github/copilot-instructions.md)"
else
  echo "  Copilot instructions: MISSING"
fi

# --- 4. Shell aliases ---
# Detect shell config file
if [ -f "$HOME/.zshrc" ]; then
  RCFILE="$HOME/.zshrc"
elif [ -f "$HOME/.bashrc" ]; then
  RCFILE="$HOME/.bashrc"
else
  RCFILE="$HOME/.bashrc"
fi

if grep -q 'convergio-llm' "$RCFILE" 2>/dev/null; then
  echo "  $RCFILE: convergio-llm already present"
else
  cat >> "$RCFILE" << 'SHELLBLOCK'

# === Convergio LLM (local models via oMLX/vLLM/llama.cpp + LiteLLM) ===
alias convergio-llm="$HOME/GitHub/ConvergioPlatform/scripts/llm/convergio-llm.sh"
claude-local() {
  ANTHROPIC_BASE_URL=http://localhost:4000 \
  ANTHROPIC_AUTH_TOKEN=REDACTED_KEY \
  claude "$@"
}
SHELLBLOCK
  echo "  $RCFILE: added convergio-llm alias + claude-local function"
fi

# --- 5. Runtime dirs ---
mkdir -p "$HOME/.convergio-llm/logs"
mkdir -p "$HOME/models"
echo "  Runtime dirs: ~/.convergio-llm/ ~/models/"

echo ""
echo "=== Verify ==="
echo "Symlinks:"
ls -la "$HOME/bin/convergio-llm"*.sh 2>/dev/null | sed 's/^/  /'
ls -la "$HOME/.continue/config.json" 2>/dev/null | sed 's/^/  /'

echo ""
echo "=== Next ==="
echo "  1. source $RCFILE"
echo "  2. convergio-llm.sh setup     (install backend + LiteLLM)"
echo "  3. convergio-llm-download.sh <hf_repo> <name>"
echo "  4. convergio-llm.sh start ~/models/<model>"
echo "  5. claude-local"
echo ""
echo "Done."
