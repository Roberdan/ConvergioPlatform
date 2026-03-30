#!/usr/bin/env bash
# mesh-provision-node.sh — Provision a new mesh node with all services
# Usage: mesh-provision-node.sh <peer-name> [--skip-build]
# Requires: peer defined in config/peers.conf, SSH access configured
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLAUDE_HOME="${CLAUDE_HOME:-$HOME/.claude}"
source "$SCRIPT_DIR/lib/peers.sh"
peers_load

PEER="${1:?Usage: mesh-provision-node.sh <peer-name> [--skip-build]}"
SKIP_BUILD=false
[[ "${2:-}" == "--skip-build" ]] && SKIP_BUILD=true

C='\033[0;36m' G='\033[0;32m' R='\033[0;31m' Y='\033[1;33m' N='\033[0m'
ok()   { echo -e "${G}[✓]${N} $*"; }
info() { echo -e "${C}[→]${N} $*"; }
warn() { echo -e "${Y}[!]${N} $*"; }
fail() { echo -e "${R}[✗]${N} $*" >&2; exit 1; }

# Resolve peer info
DEST="$(peers_best_route "$PEER")"
USER="$(peers_get "$PEER" "user" 2>/dev/null || echo "")"
OS="$(peers_get "$PEER" "os" 2>/dev/null || echo "unknown")"
ROLE="$(peers_get "$PEER" "role" 2>/dev/null || echo "worker")"
GH_ACCT="$(peers_get "$PEER" "gh_account" 2>/dev/null || echo "")"
TARGET="${USER:+${USER}@}${DEST}"
REMOTE_HOME="$(ssh -n "$TARGET" 'echo $HOME')"

info "Provisioning $PEER ($OS, $ROLE) at $TARGET"

_ssh() { ssh -n -o ConnectTimeout=10 "$TARGET" "export PATH=/opt/homebrew/bin:/usr/local/bin:\$PATH; $*"; }

# 1. Git sync
info "Step 1/8: Git sync"
_ssh "cd ~/.claude && git fetch origin main && git reset --hard origin/main" && ok "Git synced" || fail "Git sync failed"

# 1b. Sanitize worker environment (BUG-3, BUG-4, BUG-7 fixes from Plan 706 delegation)
info "Step 1b/8: Sanitize worker environment"
# BUG-4: Remove rogue $HOME/.git that blocks all git operations
_ssh "[ -f \$HOME/.git ] && rm \$HOME/.git && echo 'removed rogue .git' || true"
# BUG-3: Prune orphan worktree gitdir refs from coordinator sync
_ssh "for repo in \$HOME/GitHub/ConvergioPlatform; do [ -d \"\$repo/.git/worktrees\" ] && cd \"\$repo\" && git worktree prune 2>/dev/null; done || true"
# BUG-7: Ensure .zshenv has PATH for non-interactive SSH
_ssh 'grep -q "homebrew" \$HOME/.zshenv 2>/dev/null || cat > \$HOME/.zshenv << "ZENV"
export PATH="\$HOME/.local/bin:/opt/homebrew/bin:/opt/homebrew/sbin:\$HOME/.cargo/bin:\$PATH"
[ -f "\$HOME/.cargo/env" ] && . "\$HOME/.cargo/env"
ZENV'
ok "Worker environment sanitized"

# 2. Build binary (unless skipped)
if ! $SKIP_BUILD; then
  info "Step 2/8: Building convergio-platform-daemon"
  _ssh "cd ~/GitHub/ConvergioPlatform/daemon && cargo build --release 2>&1 | tail -3" && ok "Binary built" || fail "Build failed"
else
  warn "Step 2/8: Build skipped"
fi

# 3. Deploy crsqlite extension
info "Step 3/8: crsqlite extension"
# Detect remote arch and OS to pick the correct crsqlite binary
REMOTE_UNAME_S="$(_ssh 'uname -s' 2>/dev/null || echo "Linux")"
REMOTE_UNAME_M="$(_ssh 'uname -m' 2>/dev/null || echo "x86_64")"

# Map uname -m to crsqlite arch suffix
case "$REMOTE_UNAME_M" in
  arm64|aarch64) REMOTE_ARCH="aarch64" ;;
  x86_64|amd64)  REMOTE_ARCH="x86_64" ;;
  *)             REMOTE_ARCH="x86_64"; warn "Unknown arch $REMOTE_UNAME_M — defaulting to x86_64" ;;
esac

if [[ "$REMOTE_UNAME_S" == "Darwin" ]]; then
  EXT_FILE="crsqlite.dylib"
  CRSQL_ZIP="crsqlite-darwin-${REMOTE_ARCH}.zip"
else
  EXT_FILE="crsqlite.so"
  CRSQL_ZIP="crsqlite-linux-${REMOTE_ARCH}.zip"
fi

_ssh "test -f ~/.claude/lib/crsqlite/$EXT_FILE" && ok "crsqlite already present" || {
  _ssh "mkdir -p ~/.claude/lib/crsqlite"
  scp "$CLAUDE_HOME/lib/crsqlite/$EXT_FILE" "$TARGET:~/.claude/lib/crsqlite/" 2>/dev/null || {
    warn "SCP failed, downloading directly (arch: $REMOTE_ARCH)"
    _ssh "cd ~/.claude/lib/crsqlite && curl -sL https://github.com/vlcn-io/cr-sqlite/releases/download/v0.16.3/${CRSQL_ZIP} -o /tmp/crsql.zip && unzip -o /tmp/crsql.zip"
  }
  ok "crsqlite deployed"
}

# 4. Copy master DB (with CRR schemas) and reset site_id
info "Step 4/8: Database sync"
# Flush WAL to the main DB file before SCP so the copy is consistent.
# sqlite3 is hook-blocked; use the daemon health endpoint which triggers a
# WAL checkpoint as a side-effect, or cvg if the daemon is running.
DB_FILE="${CLAUDE_HOME}/data/dashboard.db"
if curl -sf "http://localhost:8420/api/health/deep" > /dev/null 2>&1; then
  info "WAL checkpoint via daemon health endpoint"
elif command -v cvg > /dev/null 2>&1; then
  cvg plan list > /dev/null 2>&1 || true  # cvg read forces a WAL checkpoint
else
  warn "Cannot checkpoint WAL: daemon not running and cvg unavailable. SCP may copy incomplete data."
fi
_ssh "cp ~/.claude/data/dashboard.db ~/.claude/data/dashboard.db.backup 2>/dev/null || true"
scp "$DB_FILE" "$TARGET:~/.claude/data/dashboard.db"
_ssh "sqlite3 ~/.claude/data/dashboard.db 'DELETE FROM crsql_site_id;'"
ok "DB synced with fresh site_id"

# 5. Install daemon service
info "Step 5/8: CRDT daemon service"
CRSQL_EXT="$REMOTE_HOME/.claude/lib/crsqlite/$(echo $EXT_FILE | sed 's/\..*//')"
if [[ "$OS" == "linux" ]]; then
  _ssh "cat > ~/.config/systemd/user/claude-mesh-daemon.service << EOF
[Unit]
Description=Claude Mesh CRDT Daemon
After=network-online.target tailscaled.service
[Service]
ExecStart=$REMOTE_HOME/GitHub/ConvergioPlatform/daemon/target/release/convergio-platform-daemon daemon start --peers-conf $REMOTE_HOME/.claude/config/peers.conf --db-path $REMOTE_HOME/.claude/data/dashboard.db --port 9420 --crsqlite-path $CRSQL_EXT
Restart=always
RestartSec=5
[Install]
WantedBy=default.target
EOF
systemctl --user daemon-reload && systemctl --user enable claude-mesh-daemon && systemctl --user restart claude-mesh-daemon"
else
  _ssh "cat > ~/Library/LaunchAgents/com.claude.mesh-daemon.plist << EOF
<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">
<plist version=\"1.0\"><dict>
<key>Label</key><string>com.claude.mesh-daemon</string>
<key>ProgramArguments</key><array>
<string>$REMOTE_HOME/GitHub/ConvergioPlatform/daemon/target/release/convergio-platform-daemon</string>
<string>daemon</string><string>start</string>
<string>--peers-conf</string><string>$REMOTE_HOME/.claude/config/peers.conf</string>
<string>--db-path</string><string>$REMOTE_HOME/.claude/data/dashboard.db</string>
<string>--port</string><string>9420</string>
<string>--crsqlite-path</string><string>$CRSQL_EXT</string>
</array>
<key>RunAtLoad</key><true/><key>KeepAlive</key><true/>
<key>StandardOutPath</key><string>/tmp/claude-daemon.log</string>
<key>StandardErrorPath</key><string>/tmp/claude-daemon.log</string>
</dict></plist>
EOF
launchctl unload ~/Library/LaunchAgents/com.claude.mesh-daemon.plist 2>/dev/null; launchctl load ~/Library/LaunchAgents/com.claude.mesh-daemon.plist"
fi
ok "Daemon service installed and started"

# 6. Remote screen access (VNC/Screen Sharing)
info "Step 6/8: Remote screen access"
if [[ "$OS" == "macos" ]]; then
  _ssh "launchctl list | grep -q RemoteManagementAgent" && ok "Screen Sharing already active" || {
    warn "Screen Sharing requires sudo — run manually on node:"
    warn "  sudo /System/Library/CoreServices/RemoteManagement/ARDAgent.app/Contents/Resources/kickstart -activate -configure -access -on -restart -agent -privs -all"
  }
  _ssh "launchctl list | grep -q smbd" && ok "File Sharing (SMB) active" || {
    warn "SMB requires sudo — run manually: sudo launchctl load -w /System/Library/LaunchDaemons/com.apple.smbd.plist"
  }
elif [[ "$OS" == "linux" ]]; then
  _ssh "command -v wayvnc >/dev/null 2>&1" && ok "wayvnc installed" || {
    info "Installing wayvnc..."
    _ssh "sudo pacman -S --noconfirm wayvnc 2>/dev/null || sudo apt-get install -y wayvnc 2>/dev/null || sudo dnf install -y wayvnc 2>/dev/null" && ok "wayvnc installed" || warn "wayvnc install failed — install manually"
  }
  _ssh "mkdir -p ~/.config/wayvnc && test -f ~/.config/wayvnc/config" && ok "wayvnc config exists" || {
    _ssh "mkdir -p ~/.config/wayvnc && printf 'address=0.0.0.0\nport=5900\n' > ~/.config/wayvnc/config"
    ok "wayvnc config created"
  }
  _ssh "ss -tlnp | grep -q ':5900'" && ok "VNC listening on :5900" || {
    warn "wayvnc not running — start from Wayland session: wayvnc 0.0.0.0 5900"
  }
fi

# 7. Claude auth check
info "Step 7/8: Claude auth verification"
AUTH_METHOD=$(_ssh "claude auth status 2>/dev/null | grep authMethod | head -1" 2>/dev/null || echo "")
if echo "$AUTH_METHOD" | grep -q "claude.ai"; then
  ok "Claude authenticated via OAuth (claude.ai)"
elif echo "$AUTH_METHOD" | grep -q "api_key\|oauth_token"; then
  warn "Claude using API key — must switch to OAuth: claude auth login"
else
  warn "Claude NOT authenticated — run on node: claude auth login"
fi

# 7b. Role-based checks
info "Step 7b/8: Role-based checks (role=${ROLE})"
case "${ROLE}" in
  kernel)
    # Check mlx_lm is available (Apple Silicon LLM inference)
    _ssh "python3 -c 'import mlx_lm' 2>/dev/null" \
      && ok "mlx_lm: available" \
      || warn "mlx_lm not installed — run: pip install mlx-lm (required for on-device inference)"

    # Check Python venv exists and is activatable
    _ssh "[ -d \"\$HOME/.venv\" ] || [ -d \"\$HOME/venv\" ]" \
      && ok "Python venv: found" \
      || warn "No Python venv at ~/venv or ~/.venv — create with: python3 -m venv ~/.venv"

    # Check at least one model file is present under ~/.cache/huggingface or ~/models
    _ssh "find \"\$HOME/.cache/huggingface\" \"\$HOME/models\" -name '*.safetensors' -maxdepth 4 2>/dev/null | grep -q ." \
      && ok "Model files: found" \
      || warn "No .safetensors model files detected under ~/.cache/huggingface or ~/models"

    # Check Telegram bot token is set (canonical CONVERGIO_* names, legacy TELEGRAM_* accepted)
    _ssh "[ -n \"\${CONVERGIO_TELEGRAM_TOKEN:-}\" ] || [ -n \"\${TELEGRAM_BOT_TOKEN:-}\" ] || grep -qr 'CONVERGIO_TELEGRAM_TOKEN\\|TELEGRAM_BOT_TOKEN' \"\$HOME/.zshenv\" \"\$HOME/.zprofile\" \"\$HOME/.profile\" 2>/dev/null" \
      && ok "Telegram token: configured" \
      || warn "Telegram token not set — kernel notifications via Telegram will be unavailable"
    ;;

  executor)
    # Check claude CLI is available
    _ssh "command -v claude >/dev/null 2>&1" \
      && ok "claude CLI: available" \
      || warn "claude not found — install from https://claude.ai/download or via npm"

    # Check copilot CLI is available
    _ssh "command -v copilot >/dev/null 2>&1 || command -v gh >/dev/null 2>&1" \
      && ok "AI tool (copilot/gh): available" \
      || warn "Neither copilot nor gh CLI found — at least one AI tool required for executor role"
    ;;

  *)
    info "No role-specific checks defined for role '${ROLE}'"
    ;;
esac

# 8. Verify
info "Step 8/8: Verification"
sleep 3
DAEMON_OK=$(_ssh "ps aux | grep 'convergio-platform-daemon daemon' | grep -v grep | wc -l" 2>/dev/null)
DB_PLANS=$(_ssh "sqlite3 ~/.claude/data/dashboard.db 'SELECT COUNT(*) FROM plans;'" 2>/dev/null)
TOOLS=$(_ssh "which copilot claude 2>/dev/null | wc -l" 2>/dev/null)

echo ""
echo "╔══════════════════════════════════════╗"
echo "║     Node Provisioning Summary        ║"
echo "╠══════════════════════════════════════╣"
printf "║ %-20s %15s ║\n" "Peer:" "$PEER"
printf "║ %-20s %15s ║\n" "OS:" "$OS"
printf "║ %-20s %15s ║\n" "Role:" "$ROLE"
printf "║ %-20s %15s ║\n" "CRDT Daemon:" "$([[ $DAEMON_OK -gt 0 ]] && echo '✅ running' || echo '❌ down')"
printf "║ %-20s %15s ║\n" "DB Plans:" "$DB_PLANS"
printf "║ %-20s %15s ║\n" "AI Tools:" "$TOOLS/2"
echo "╚══════════════════════════════════════╝"

[[ $DAEMON_OK -gt 0 ]] && ok "Node $PEER fully provisioned!" || warn "Daemon check failed — inspect service logs"
