#!/usr/bin/env bash
# setup.sh — ConvergioPlatform single-entry-point installer
# Usage: git clone <repo> && cd ConvergioPlatform && ./setup.sh
set -euo pipefail

PLATFORM_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

detect_platform() {
  case "$(uname -s)" in
    Darwin) echo "macOS" ;;
    Linux)  echo "Linux" ;;
    *)      echo "Unknown"; return 1 ;;
  esac
}

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
  local platform profile db_path settings_src
  local s_sym="FAIL" s_set="SKIP" s_db="FAIL" s_daemon="WARN"

  echo "=== ConvergioPlatform Setup ==="
  platform="$(detect_platform)" || { fail "Unsupported platform"; exit 1; }
  echo "Platform: $platform | Directory: $PLATFORM_DIR"
  echo ""

  # 1. Symlinks via existing script
  local symlink_script="$PLATFORM_DIR/scripts/platform/setup-claude-symlinks.sh"
  if [[ -x "$symlink_script" ]]; then
    bash "$symlink_script" "$PLATFORM_DIR"
    s_sym="OK"
  else
    fail "Symlink script not found: $symlink_script"
    exit 1
  fi
  echo ""

  # 2. Copy .claude/settings.json if present in project
  settings_src="$PLATFORM_DIR/.claude/settings.json"
  if [[ -f "$settings_src" ]]; then
    mkdir -p "$HOME/.claude"
    cp "$settings_src" "$HOME/.claude/settings.json"
    s_set="OK"
  fi

  # 3. Set DASHBOARD_DB in shell profile if not already set
  db_path="$PLATFORM_DIR/data/dashboard.db"
  profile="$(shell_profile)"
  if grep -q 'DASHBOARD_DB' "$profile" 2>/dev/null; then
    s_db="OK"
  else
    echo "export DASHBOARD_DB=\"$db_path\"" >> "$profile"
    s_db="OK"
  fi
  export DASHBOARD_DB="$db_path"

  # 4. Check daemon binary
  local daemon_bin="$PLATFORM_DIR/daemon/target/release/claude-core"
  if [[ -x "$daemon_bin" ]]; then
    s_daemon="OK"
  fi

  # 5. Summary checklist
  echo "=== Setup Summary ==="
  local lnk_count
  lnk_count="$(find "$HOME/.claude" -maxdepth 1 -type l 2>/dev/null | wc -l | tr -d ' ')"

  if [[ "$s_sym" == "OK" ]]; then
    ok "Symlinks: ~/.claude/ -> ConvergioPlatform/claude-config/ ($lnk_count links)"
  else fail "Symlinks: setup failed"; fi

  if [[ "$s_set" == "OK" ]]; then
    ok "Settings: .claude/settings.json copied"
  else warn "Settings: no project settings.json (create .claude/settings.json to enable)"; fi

  if [[ "$s_db" == "OK" ]]; then
    ok "Database: DASHBOARD_DB=$db_path"
  else fail "Database: DASHBOARD_DB not configured"; fi

  if [[ "$s_daemon" == "OK" ]]; then
    ok "Daemon: $daemon_bin"
  else warn "Daemon: not built (run: cd daemon && cargo build --release)"; fi

  echo ""
  echo "Done. Restart your shell or run: source $profile"
}

main "$@"
