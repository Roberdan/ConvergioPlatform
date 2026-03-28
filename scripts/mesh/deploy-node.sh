#!/usr/bin/env bash
# deploy-node.sh — Single-command deploy to any mesh node.
# Usage: deploy-node.sh <ssh-alias|peer-name> [--kernel]
# Reads peer SSH alias from ~/.claude/config/peers.conf.
# Orchestrates only — daemon API performs final verification.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLAUDE_HOME="${CLAUDE_HOME:-$HOME/.claude}"
source "$SCRIPT_DIR/lib/peers.sh"
peers_load

readonly DAEMON_PORT="8420"
readonly HEALTH_URL="http://localhost:${DAEMON_PORT}/api/health"
readonly HEALTH_RETRIES=10
readonly HEALTH_SLEEP=2
readonly DAEMON_PROCESS="convergio-platform-daemon"
readonly DB_REL_PATH="GitHub/ConvergioPlatform/data/dashboard.db"
readonly REPO_PATH="~/GitHub/ConvergioPlatform"

C='\033[0;36m' G='\033[0;32m' R='\033[0;31m' Y='\033[1;33m' N='\033[0m'
ok()   { echo -e "${G}[OK]${N} $*"; }
info() { echo -e "${C}[->]${N} $*"; }
warn() { echo -e "${Y}[!]${N} $*"; }
fail() { echo -e "${R}[FAIL]${N} $*" >&2; exit 1; }

usage() {
  echo "Usage: deploy-node.sh <peer-name|ssh-alias> [--kernel]" >&2
  echo "  peer-name: section name in peers.conf (e.g. macProM1, omarchy)" >&2
  echo "  --kernel:  build with kernel feature + verify models loaded" >&2
  exit 1
}

[[ $# -lt 1 ]] && usage

PEER_ARG="$1"
WITH_KERNEL=false
[[ "${2:-}" == "--kernel" ]] && WITH_KERNEL=true

# Resolve peer from peers.conf; fall back to treating arg as raw SSH alias
PEER_NAME=""
SSH_TARGET=""
for name in $(peers_list); do
  alias_val="$(peers_get "$name" "ssh_alias" 2>/dev/null || true)"
  if [[ "$name" == "$PEER_ARG" || "$alias_val" == "$PEER_ARG" ]]; then
    PEER_NAME="$name"
    break
  fi
done

if [[ -n "$PEER_NAME" ]]; then
  DEST="$(peers_best_route "$PEER_NAME")"
  PEER_USER="$(peers_get "$PEER_NAME" "user" 2>/dev/null || echo "")"
  SSH_TARGET="${PEER_USER:+${PEER_USER}@}${DEST}"
else
  warn "Peer '$PEER_ARG' not in peers.conf — using as raw SSH target."
  SSH_TARGET="$PEER_ARG"
fi

info "=== Deploy: ${PEER_ARG} (kernel=${WITH_KERNEL}) ==="
info "SSH target: ${SSH_TARGET}"

_ssh() { ssh -n -o ConnectTimeout=15 -o BatchMode=yes "$SSH_TARGET" "export PATH=/opt/homebrew/bin:/opt/homebrew/sbin:/usr/local/bin:\$HOME/.cargo/bin:\$PATH; $*"; }

# Step 1: Git pull on node
info "[1/9] Git pull on node..."
_ssh "cd ${REPO_PATH} && git pull origin main" && ok "Git pull complete" || fail "Git pull failed"

# Step 2: Build daemon on node
if [[ "$WITH_KERNEL" == "true" ]]; then
  info "[2/9] Building with --features kernel --release..."
  _ssh "cd ${REPO_PATH}/daemon && cargo build --features kernel --release 2>&1 | tail -5" \
    && ok "Build (kernel) complete" || fail "Build failed"
else
  info "[2/9] Building --release..."
  _ssh "cd ${REPO_PATH}/daemon && cargo build --release 2>&1 | tail -5" \
    && ok "Build complete" || fail "Build failed"
fi

# Step 3: Stop daemon on node
info "[3/9] Stopping daemon on node..."
_ssh "pkill -f '${DAEMON_PROCESS}' 2>/dev/null && echo 'daemon stopped' || echo 'daemon was not running'"
sleep 3

# Step 4: Sync DB from this Mac to node (direct rsync, no sync-db.sh dependency)
info "[4/9] Syncing DB to node..."
LOCAL_DB="$(readlink -f "${HOME}/${DB_REL_PATH}" 2>/dev/null || echo "${HOME}/${DB_REL_PATH}")"
if [[ -f "$LOCAL_DB" ]]; then
  REMOTE_HOME="$(_ssh 'echo $HOME' 2>/dev/null | tr -d '[:space:]')"
  REMOTE_DB="${REMOTE_HOME}/${DB_REL_PATH}"
  rsync -aLz "$LOCAL_DB" "${SSH_TARGET}:${REMOTE_DB}" 2>/dev/null && ok "DB synced" || warn "DB rsync failed"
  rsync -aLz "${LOCAL_DB}-wal" "${SSH_TARGET}:${REMOTE_DB}-wal" 2>/dev/null || true
  rsync -aLz "${LOCAL_DB}-shm" "${SSH_TARGET}:${REMOTE_DB}-shm" 2>/dev/null || true
else
  warn "Local DB not found at ${LOCAL_DB} — skipping sync"
fi

# Step 5: Replicate secrets from this Mac's keychain to node
info "[5/9] Replicating keychain secrets to node..."
TG_TOKEN=""
TG_CHAT_ID=""
TG_TOKEN="$(security find-generic-password -a telegram-bot -s convergio-platform -w 2>/dev/null || true)"
TG_CHAT_ID="$(security find-generic-password -a telegram-chat-id -s convergio-platform -w 2>/dev/null || true)"

HF_TOKEN="$(security find-generic-password -a huggingface -s convergio-platform -w 2>/dev/null || true)"

_ssh "mkdir -p ~/.convergio && : > ~/.convergio/env && chmod 600 ~/.convergio/env"
[[ -n "$TG_TOKEN" ]] && _ssh "echo 'CONVERGIO_TELEGRAM_TOKEN=${TG_TOKEN}' >> ~/.convergio/env"
[[ -n "$TG_CHAT_ID" ]] && _ssh "echo 'CONVERGIO_TELEGRAM_CHAT_ID=${TG_CHAT_ID}' >> ~/.convergio/env"
[[ -n "$HF_TOKEN" ]] && _ssh "echo 'HF_TOKEN=${HF_TOKEN}' >> ~/.convergio/env"

if [[ -n "$TG_TOKEN" ]] || [[ -n "$HF_TOKEN" ]]; then
  ok "Secrets written to ~/.convergio/env (TG=$([ -n \"$TG_TOKEN\" ] && echo yes || echo no), HF=$([ -n \"$HF_TOKEN\" ] && echo yes || echo no))"
  # Also login huggingface if token available
  [[ -n "$HF_TOKEN" ]] && _ssh "source ~/convergio-env/bin/activate 2>/dev/null && python3 -c \"from huggingface_hub import login; login(token='${HF_TOKEN}', add_to_git_credential=False)\" 2>/dev/null" \
    && ok "HuggingFace login OK" || warn "HF login failed (non-fatal)"
else
  warn "No secrets found in local keychain — skipping"
fi

# Step 6: Create Convergio tmux session + start daemon
info "[6/9] Starting daemon on node..."
_ssh "tmux has-session -t Convergio 2>/dev/null || tmux new-session -d -s Convergio -n kernel -c ~/GitHub/ConvergioPlatform" 2>/dev/null || true
_ssh "bash -lc '
  # Source secrets from env file (written by deploy step 5)
  [[ -f ~/.convergio/env ]] && source ~/.convergio/env
  plist=\"\${HOME}/Library/LaunchAgents/com.convergio.kernel.plist\"
  if [[ -f \"\$plist\" ]]; then
    launchctl unload \"\$plist\" 2>/dev/null || true
    launchctl load \"\$plist\" && echo \"launchd agent loaded\"
  else
    echo \"WARNING: plist not found — starting daemon directly\" >&2
    [[ -n \"\$token\" ]] && export CONVERGIO_TELEGRAM_TOKEN=\"\$token\"
    [[ -n \"\$chat_id\" ]] && export CONVERGIO_TELEGRAM_CHAT_ID=\"\$chat_id\"
    nohup \"${REPO_PATH}/daemon/target/release/${DAEMON_PROCESS}\" serve \
      >/tmp/convergio-daemon.log 2>&1 &
    echo \"daemon started (pid \$!)\"
  fi
'"

# Step 7: Wait for /api/health
info "[7/9] Waiting for daemon health (${HEALTH_RETRIES}x${HEALTH_SLEEP}s)..."
attempt=1
health_ok=false
while [[ $attempt -le $HEALTH_RETRIES ]]; do
  http_code="$(_ssh "curl -s -o /dev/null -w '%{http_code}' '${HEALTH_URL}' 2>/dev/null || echo 000")"
  if [[ "$http_code" == "200" ]]; then
    ok "Health check PASSED (HTTP 200, attempt ${attempt})"
    health_ok=true
    break
  fi
  info "Attempt ${attempt}/${HEALTH_RETRIES}: status=${http_code}, retrying in ${HEALTH_SLEEP}s..."
  sleep "$HEALTH_SLEEP"
  (( attempt++ ))
done
[[ "$health_ok" == "false" ]] && fail "Daemon did not become healthy after ${HEALTH_RETRIES} attempts"

# Step 8: Node readiness
info "[8/9] Checking node readiness..."
readiness="$(_ssh "curl -s 'http://localhost:${DAEMON_PORT}/api/node/readiness' 2>/dev/null || echo '{\"error\":\"unreachable\"}'")";
echo "Readiness: ${readiness}"

# Step 9: Kernel model verification (--kernel only)
if [[ "$WITH_KERNEL" == "true" ]]; then
  info "[9/9] Verifying kernel models (--kernel)..."
  kernel_status="$(_ssh "curl -s 'http://localhost:${DAEMON_PORT}/api/kernel/status' 2>/dev/null || echo '{\"models_loaded\":0}'")";
  models_loaded="$(echo "$kernel_status" | grep -oE '"models_loaded"\s*:\s*[0-9]+' | grep -oE '[0-9]+' || echo 0)"
  if [[ "${models_loaded:-0}" -ge 1 ]]; then
    ok "Kernel OK: models_loaded=${models_loaded}"
  else
    fail "Kernel not ready: models_loaded=${models_loaded:-0}. Status: ${kernel_status}"
  fi
else
  info "[9/9] Skipping kernel check (no --kernel flag)"
fi

ok "=== Deploy complete: ${PEER_ARG} ==="
