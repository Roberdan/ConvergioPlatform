#!/usr/bin/env bash
# setup-coordinator-remote.sh — Configure 'coordinator' git remote on a mesh peer
#
# Usage (SSH mode, used by mesh-delegate-task.sh):
#   setup-coordinator-remote.sh --peer <ssh_host> --peer-repo-path <path> \
#                                --coordinator-url <ssh://user@host/path>
#
# Usage (local mode, used by tests):
#   setup-coordinator-remote.sh --local-repo <path> --coordinator-url <url>
#
# Returns 0 if remote exists and is functional (added or already present).
# Returns 1 on SSH access failure or misconfiguration.
#
# SKIP_SSH_CHECK=1 env var disables the SSH BatchMode verification step (tests only).
set -euo pipefail

# --- Parse arguments ---
PEER=""
PEER_REPO_PATH=""
COORDINATOR_URL=""
LOCAL_REPO=""  # test/local mode: operate directly on this path

while [[ $# -gt 0 ]]; do
  case "$1" in
    --peer)             PEER="$2";             shift 2 ;;
    --peer-repo-path)   PEER_REPO_PATH="$2";   shift 2 ;;
    --coordinator-url)  COORDINATOR_URL="$2";  shift 2 ;;
    --local-repo)       LOCAL_REPO="$2";       shift 2 ;;
    *) echo "Unknown argument: $1" >&2; exit 1 ;;
  esac
done

# --- Validate required arguments ---
if [[ -z "$COORDINATOR_URL" ]]; then
  echo "ERROR: --coordinator-url is required" >&2
  echo "Usage: $0 (--peer <host> --peer-repo-path <path> | --local-repo <path>) --coordinator-url <url>" >&2
  exit 1
fi

# Must have either (--peer + --peer-repo-path) or --local-repo
if [[ -z "$LOCAL_REPO" && ( -z "$PEER" || -z "$PEER_REPO_PATH" ) ]]; then
  echo "ERROR: provide either --local-repo OR (--peer + --peer-repo-path)" >&2
  echo "Usage: $0 (--peer <host> --peer-repo-path <path> | --local-repo <path>) --coordinator-url <url>" >&2
  exit 1
fi

# --- Helpers ---
G='\033[0;32m' R='\033[0;31m' Y='\033[1;33m' N='\033[0m'
ok()   { echo -e "${G}[coordinator-remote]${N} $*"; }
warn() { echo -e "${Y}[coordinator-remote]${N} $*"; }
err()  { echo -e "${R}[coordinator-remote] ERROR:${N} $*" >&2; }

SKIP_SSH_CHECK="${SKIP_SSH_CHECK:-0}"

# --- Local mode (tests + single-machine setup) ---
_configure_local() {
  local repo="$1"
  local url="$2"

  if [[ ! -d "$repo/.git" ]]; then
    err "Not a git repository: $repo"
    return 1
  fi

  local existing_url
  existing_url="$(git -C "$repo" remote get-url coordinator 2>/dev/null || echo '')"

  if [[ -n "$existing_url" ]]; then
    if [[ "$existing_url" == "$url" ]]; then
      ok "remote 'coordinator' already configured at $url — skipping"
      return 0
    else
      warn "remote 'coordinator' exists with different URL: $existing_url"
      warn "updating to: $url"
      git -C "$repo" remote set-url coordinator "$url"
      ok "remote 'coordinator' updated"
      return 0
    fi
  fi

  git -C "$repo" remote add coordinator "$url"
  ok "remote 'coordinator' added: $url"
  return 0
}

# --- SSH mode (production use on mesh peers) ---
_check_ssh_access() {
  local host="$1"
  if [[ "$SKIP_SSH_CHECK" == "1" ]]; then
    return 0
  fi
  # BatchMode=yes: fail immediately if interactive auth is needed
  if ! ssh -o BatchMode=yes -o ConnectTimeout=10 "$host" "exit 0" 2>/dev/null; then
    err "SSH key access denied or host unreachable: $host"
    return 1
  fi
  return 0
}

_configure_remote_peer() {
  local peer="$1"
  local repo_path="$2"
  local url="$3"

  ok "Verifying SSH access to $peer..."
  _check_ssh_access "$peer"

  ok "Configuring 'coordinator' remote on $peer:$repo_path..."

  # Single SSH call: check existing remote, add if missing
  # Using heredoc to keep complex quoting readable
  local remote_script
  remote_script="$(cat <<'RSCRIPT'
set -euo pipefail
REPO="$1"
URL="$2"
if [[ ! -d "$REPO/.git" ]]; then
  echo "ERROR: not a git repo: $REPO" >&2
  exit 1
fi
existing_url="$(git -C "$REPO" remote get-url coordinator 2>/dev/null || echo '')"
if [[ -n "$existing_url" ]]; then
  if [[ "$existing_url" == "$URL" ]]; then
    echo "SKIP: coordinator remote already configured"
  else
    git -C "$REPO" remote set-url coordinator "$URL"
    echo "UPDATED: coordinator remote updated to $URL"
  fi
else
  git -C "$REPO" remote add coordinator "$URL"
  echo "ADDED: coordinator remote -> $URL"
fi
RSCRIPT
)"

  local result
  result="$(ssh -n -o BatchMode=yes -o ConnectTimeout=10 "$peer" \
    "bash -s" -- "$repo_path" "$url" <<< "$remote_script" 2>&1)" || {
    err "Remote configuration failed on $peer"
    return 1
  }

  echo "$result"
  ok "Done: $peer configured"
  return 0
}

# --- Main ---
if [[ -n "$LOCAL_REPO" ]]; then
  _configure_local "$LOCAL_REPO" "$COORDINATOR_URL"
else
  _configure_remote_peer "$PEER" "$PEER_REPO_PATH" "$COORDINATOR_URL"
fi
