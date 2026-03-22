#!/usr/bin/env bash
# setup.sh — ConvergioPlatform installer
# Sets up env vars, enables Convergio overlay, installs CLI aliases.
# Everything reversible: `convergio off` disables, `revert-claude-symlinks.sh` restores.
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

shell_rc() {
  if [[ -n "${ZSH_VERSION:-}" ]] || [[ "$SHELL" == */zsh ]]; then
    echo "$HOME/.zshrc"
  else
    echo "$HOME/.bashrc"
  fi
}

ok()   { echo "  [OK]   $1"; }
warn() { echo "  [WARN] $1"; }
fail() { echo "  [FAIL] $1"; }

main() {
  local profile rc db_path
  echo "=== ConvergioPlatform Setup ==="
  echo "Platform: $(uname -s) | Directory: $PLATFORM_DIR"
  echo ""

  # 1. Set DASHBOARD_DB env var
  db_path="$PLATFORM_DIR/data/dashboard.db"
  profile="$(shell_profile)"
  if grep -q 'DASHBOARD_DB' "$profile" 2>/dev/null; then
    ok "DASHBOARD_DB already set"
  else
    echo "export DASHBOARD_DB=\"$db_path\"" >> "$profile"
    ok "DASHBOARD_DB=$db_path (added to $profile)"
  fi

  # 2. Set CONVERGIO_PLATFORM_DIR env var
  if grep -q 'CONVERGIO_PLATFORM_DIR' "$profile" 2>/dev/null; then
    ok "CONVERGIO_PLATFORM_DIR already set"
  else
    echo "export CONVERGIO_PLATFORM_DIR=\"$PLATFORM_DIR\"" >> "$profile"
    ok "CONVERGIO_PLATFORM_DIR=$PLATFORM_DIR (added to $profile)"
  fi
  export DASHBOARD_DB="$db_path"
  export CONVERGIO_PLATFORM_DIR="$PLATFORM_DIR"

  # 3. Add CLI to PATH + aliases (in .zshrc, not .zshenv)
  rc="$(shell_rc)"
  if grep -q 'convergio-aliases.sh' "$rc" 2>/dev/null; then
    ok "CLI aliases already in $rc"
  else
    echo "" >> "$rc"
    echo "# === Convergio ===" >> "$rc"
    echo "source $PLATFORM_DIR/scripts/platform/convergio-aliases.sh" >> "$rc"
    ok "CLI aliases added to $rc"
  fi

  # 4. Global DB symlink (for tools that read ~/.claude/data/dashboard.db)
  mkdir -p "$HOME/.claude/data"
  if [[ ! -L "$HOME/.claude/data/dashboard.db" ]]; then
    ln -sf "$db_path" "$HOME/.claude/data/dashboard.db"
    ok "Global DB symlink: ~/.claude/data/dashboard.db"
  else
    ok "Global DB symlink: already exists"
  fi

  # 5. Enable Convergio overlay (global symlinks in ~/.claude/)
  echo ""
  bash "$PLATFORM_DIR/scripts/platform/convergio-toggle.sh" on

  # 6. Check daemon binary and create CLI symlinks
  echo ""
  local daemon_bin="$PLATFORM_DIR/daemon/target/release/convergio-platform-daemon"
  local bin_dir
  if [[ -d "$HOME/.local/bin" ]] || mkdir -p "$HOME/.local/bin" 2>/dev/null; then
    bin_dir="$HOME/.local/bin"
  else
    bin_dir="/usr/local/bin"
  fi

  if [[ -x "$daemon_bin" ]]; then
    ok "Daemon: $daemon_bin"

    # Create convergio symlink
    if [[ -L "$bin_dir/convergio" ]] || [[ -f "$bin_dir/convergio" ]]; then
      ok "Symlink: $bin_dir/convergio (already exists)"
    else
      ln -sf "$daemon_bin" "$bin_dir/convergio"
      ok "Symlink: $bin_dir/convergio → $daemon_bin"
    fi

    # Create cvg symlink (short alias)
    if [[ -L "$bin_dir/cvg" ]] || [[ -f "$bin_dir/cvg" ]]; then
      ok "Symlink: $bin_dir/cvg (already exists)"
    else
      ln -sf "$daemon_bin" "$bin_dir/cvg"
      ok "Symlink: $bin_dir/cvg → $daemon_bin"
    fi

    # Verify both commands respond (non-fatal — binary may need DB to be fully functional)
    if "$bin_dir/convergio" --version &>/dev/null || "$bin_dir/convergio" version &>/dev/null; then
      ok "Verify: convergio responds"
    else
      warn "Verify: convergio did not respond to version (may need daemon running)"
    fi
    if "$bin_dir/cvg" --version &>/dev/null || "$bin_dir/cvg" version &>/dev/null; then
      ok "Verify: cvg responds"
    else
      warn "Verify: cvg did not respond to version (may need daemon running)"
    fi

    # Ensure bin_dir is in PATH hint (only in .zshenv for ~/.local/bin)
    if [[ "$bin_dir" == "$HOME/.local/bin" ]]; then
      local path_export='export PATH="$HOME/.local/bin:$PATH"'
      if ! grep -q '.local/bin' "$profile" 2>/dev/null; then
        echo "$path_export" >> "$profile"
        ok "PATH: $bin_dir added to $profile"
      fi
    fi
  else
    warn "Daemon: not built — run: cd daemon && cargo build --release"
    warn "Symlinks skipped: build daemon first, then re-run setup.sh"
  fi

  echo ""
  echo "=== Setup Complete ==="
  echo ""
  echo "Commands available after shell restart:"
  echo "  convergio list       — see all agents"
  echo "  convergio solve ...  — give Ali a problem"
  echo "  convergio on/off     — enable/disable overlay"
  echo "  convergio status     — check state"
  echo "  cvg plan status      — plan status (short alias)"
  echo ""
  echo "To disable completely:"
  echo "  convergio off                         — remove ~/.claude/ symlinks"
  echo "  revert-claude-symlinks.sh --env       — also remove env vars"
  echo ""
  echo "Restart shell: source $rc"
}

main "$@"
