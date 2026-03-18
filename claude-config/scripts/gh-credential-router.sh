#!/usr/bin/env bash
# gh-credential-router.sh — Route git credentials by repo directory
# Version: 1.1.0
#
# Maps repo directories to GitHub accounts automatically.
# Eliminates manual `gh auth switch` before every push.
# Account mappings driven by config/local.env (GH_DEFAULT_ACCOUNT,
# GH_MICROSOFT_ACCOUNT, GH_MICROSOFT_REPOS, GH_OTHER_ACCOUNTS).
#
# Install: add to ~/.gitconfig as credential helper (see bottom)
# Usage: called automatically by git on push/fetch operations

set -euo pipefail
trap 'echo "ERROR at line $LINENO" >&2' ERR

_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../../config/load-config.sh
source "$_SCRIPT_DIR/../../config/load-config.sh" 2>/dev/null || true
unset _SCRIPT_DIR

# === ACCOUNT MAPPING (config-driven) ===
# GH_MICROSOFT_REPOS: comma-separated repo names → GH_MICROSOFT_ACCOUNT
# GH_OTHER_ACCOUNTS:  comma-separated "RepoName:Account" pairs
# GH_DEFAULT_ACCOUNT: fallback for everything else

get_account_for_dir() {
    local dir="$1"

    # Check repos that belong to the Microsoft account
    IFS=',' read -ra _ms_repos <<< "${GH_MICROSOFT_REPOS:-VirtualBPM}"
    for _repo in "${_ms_repos[@]}"; do
        [[ "$dir" == *"/$_repo"* ]] && { echo "${GH_MICROSOFT_ACCOUNT:-}"; return; }
    done

    # Check other account mappings ("RepoName:Account" pairs)
    IFS=',' read -ra _other_pairs <<< "${GH_OTHER_ACCOUNTS:-}"
    for _pair in "${_other_pairs[@]}"; do
        local _repo="${_pair%%:*}"
        local _account="${_pair#*:}"
        [[ "$dir" == *"/$_repo"* ]] && { echo "$_account"; return; }
    done

    echo "${GH_DEFAULT_ACCOUNT:-Roberdan}"
}

# === CREDENTIAL HELPER PROTOCOL ===
# Git calls with: get, store, or erase

ACTION="${1:-get}"

if [ "$ACTION" != "get" ]; then
    # For store/erase, delegate to gh
    /opt/homebrew/bin/gh auth git-credential "$ACTION"
    exit $?
fi

# Read stdin (git sends host/protocol)
INPUT=$(cat)
HOST=$(echo "$INPUT" | grep "^host=" | cut -d= -f2)

# Only handle github.com
if [ "$HOST" != "github.com" ]; then
    echo "$INPUT" | /opt/homebrew/bin/gh auth git-credential get
    exit $?
fi

# Determine current repo directory
REPO_DIR=$(git rev-parse --show-toplevel 2>/dev/null || echo "$PWD")
ACCOUNT=$(get_account_for_dir "$REPO_DIR")

# Get token for the correct account
TOKEN=$(/opt/homebrew/bin/gh auth token --user "$ACCOUNT" 2>/dev/null)

if [ -z "$TOKEN" ]; then
    # Fallback to default gh credential
    echo "$INPUT" | /opt/homebrew/bin/gh auth git-credential get
    exit $?
fi

# Return credentials in git protocol format
echo "protocol=https"
echo "host=github.com"
echo "username=$ACCOUNT"
echo "password=$TOKEN"
