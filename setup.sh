#!/usr/bin/env bash
# setup.sh — ConvergioPlatform installer
# Configures env vars and verifies project state.
# Symlinks (.claude/commands etc.) are committed to git — no global ~/.claude mutation.
# Usage: git clone <repo> && cd ConvergioPlatform && ./setup.sh
set -euo pipefail

PLATFORM_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

shell_profile() {
  if [[ -n "${ZSH_VERSION:-}" ]] || [[ "$SHELL" == */zsh ]]; then
    echo "$HOME/.zshenv"
  else
    echo "$HOME/.bashrc"
  fi
}

ok()   { echo "  [OK]   $1"; }
warn() { echo "  [WARN] $1"; }
fail() { echo "  [FAIL] $1"; }

main() {
  local profile db_path
  echo "=== ConvergioPlatform Setup ==="
  echo "Platform: $(uname -s) | Directory: $PLATFORM_DIR"
  echo ""

  # 1. Verify project-level symlinks (committed to git, no setup needed)
  local missing=0
  for link in commands agents rules reference CLAUDE.md; do
    if [[ -L "$PLATFORM_DIR/.claude/$link" ]]; then
      ok "Symlink: .claude/$link → $(readlink "$PLATFORM_DIR/.claude/$link")"
    else
      fail "Missing symlink: .claude/$link (run: scripts/platform/setup-claude-symlinks.sh)"
      missing=1
    fi
  done

  # 2. Recreate symlinks if any missing
  if [[ "$missing" -eq 1 ]]; then
    echo ""
    echo "Recreating missing symlinks..."
    bash "$PLATFORM_DIR/scripts/platform/setup-claude-symlinks.sh" "$PLATFORM_DIR"
  fi
  echo ""

  # 3. Set DASHBOARD_DB in shell profile (only global mutation)
  db_path="$PLATFORM_DIR/data/dashboard.db"
  profile="$(shell_profile)"
  if grep -q 'DASHBOARD_DB' "$profile" 2>/dev/null; then
    ok "Database: DASHBOARD_DB already set"
  else
    echo "export DASHBOARD_DB=\"$db_path\"" >> "$profile"
    ok "Database: DASHBOARD_DB=$db_path (added to $profile)"
  fi
  export DASHBOARD_DB="$db_path"

  # 4. Ensure data/dashboard.db symlink in ~/.claude/data/ (for global tools)
  mkdir -p "$HOME/.claude/data"
  if [[ ! -L "$HOME/.claude/data/dashboard.db" ]]; then
    ln -sf "$db_path" "$HOME/.claude/data/dashboard.db"
    ok "Global DB symlink: ~/.claude/data/dashboard.db"
  else
    ok "Global DB symlink: already exists"
  fi

  # 5. Check daemon binary
  local daemon_bin="$PLATFORM_DIR/daemon/target/release/claude-core"
  if [[ -x "$daemon_bin" ]]; then
    ok "Daemon: $daemon_bin"
  else
    warn "Daemon: not built (run: cd daemon && cargo build --release)"
  fi

  echo ""
  echo "Done. Global ~/.claude/ untouched (settings, commands, agents stay yours)."
  echo "Project config active via .claude/ symlinks → claude-config/."
  [[ -n "${missing:-}" && "$missing" -eq 1 ]] && echo "Restart shell or: source $profile"
}

main "$@"
