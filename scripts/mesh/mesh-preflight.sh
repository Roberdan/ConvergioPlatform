#!/usr/bin/env bash
set -euo pipefail
# mesh-preflight.sh — Full health check for all mesh nodes (tools + auth + versions)
# Usage: mesh-preflight.sh [--json] [--peer NAME]
# Outputs table (default) or JSON (for dashboard ingestion)
# Exit 0 = all green, Exit 1 = issues found

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/lib/peers.sh"
peers_load

JSON=false
TARGET_PEER=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --json) JSON=true; shift ;;
    --peer) TARGET_PEER="${2:-}"; shift 2 ;;
    *) shift ;;
  esac
done

SELF="$(peers_self)"
LOCAL_SHA="$(cd ~/.claude && git rev-parse --short HEAD)"
LOCAL_CLAUDE="$(claude --version 2>/dev/null | head -1)"
ISSUES=0
RESULTS_JSON="[]"
RPATH='export PATH="/opt/homebrew/bin:/usr/local/bin:$HOME/.local/bin:$HOME/.npm-global/bin:$PATH";'

_check_node() {
  local peer="$1" is_self=false
  [[ "$peer" == "$SELF" ]] && is_self=true

  local dest="" user=""
  if ! $is_self; then
    if ! peers_check "$peer" 2>/dev/null; then
      _emit "$peer" "OFFLINE" "-" "-" "-" "-" "-" "-" "-" "-"
      return
    fi
    dest="$(peers_best_route "$peer" 2>/dev/null)"
    user="$(peers_get "$peer" "user" 2>/dev/null || echo "")"
    [[ -n "$user" ]] && dest="${user}@${dest}"
  fi

  # Gather all info in one SSH call (or locally)
  local info
  local CHECK_SCRIPT="${RPATH}"'
sha=$(cd ~/.claude && git rev-parse --short=7 HEAD 2>/dev/null || echo "?")
branch=$(cd ~/.claude && git branch --show-current 2>/dev/null || echo "?")
claude_ver=$(claude --version 2>/dev/null | head -1 || echo "missing")
auth_json=$(claude auth status 2>/dev/null || echo "{}")
logged=$(echo "$auth_json" | grep -o "\"loggedIn\": *[a-z]*" | head -1 | grep -o "true\|false" || echo "false")
method=$(echo "$auth_json" | grep -o "\"authMethod\": *\"[^\"]*\"" | head -1 | sed "s/.*\"\([^\"]*\)\"/\1/" || echo "none")
gh_auth="none"
command -v gh >/dev/null 2>&1 && gh auth status 2>&1 | grep -q "Logged in" && gh_auth="ok"
copilot_ver="missing"
command -v gh >/dev/null 2>&1 && copilot_ver=$(gh copilot --version 2>/dev/null | head -1 || echo "missing")
if [ "$copilot_ver" = "missing" ]; then
  command -v copilot >/dev/null 2>&1 && copilot_ver=$(copilot --version 2>/dev/null | head -1 || echo "missing")
fi
printf "SHA=%s\nBRANCH=%s\nCLAUDE_VER=%s\nCLAUDE_LOGGED=%s\nCLAUDE_METHOD=%s\nGH_AUTH=%s\nCOPILOT_VER=%s\n" \
  "$sha" "$branch" "$claude_ver" "$logged" "$method" "$gh_auth" "$copilot_ver"
'

  if $is_self; then
    info=$(bash -c "$CHECK_SCRIPT" 2>/dev/null) || info=""
  else
    info=$(ssh -n -o ConnectTimeout=5 -o BatchMode=yes "$dest" "$CHECK_SCRIPT" 2>/dev/null) || info=""
  fi

  local sha branch claude_ver claude_logged claude_method gh_auth copilot_ver
  sha=$(echo "$info" | sed -n 's/^SHA=//p')
  branch=$(echo "$info" | sed -n 's/^BRANCH=//p')
  claude_ver=$(echo "$info" | sed -n 's/^CLAUDE_VER=//p')
  claude_logged=$(echo "$info" | sed -n 's/^CLAUDE_LOGGED=//p')
  claude_method=$(echo "$info" | sed -n 's/^CLAUDE_METHOD=//p')
  gh_auth=$(echo "$info" | sed -n 's/^GH_AUTH=//p')
  copilot_ver=$(echo "$info" | sed -n 's/^COPILOT_VER=//p')

  _emit "$peer" "${sha:-?}" "${branch:-?}" "${claude_ver:-missing}" \
    "$claude_logged" "${claude_method:-none}" "${gh_auth:-none}" "${copilot_ver:-missing}" \
    "$LOCAL_SHA" "$LOCAL_CLAUDE"
}

_emit() {
  local peer="$1" sha="$2" branch="$3" claude_ver="$4"
  local claude_auth="$5" claude_method="$6" gh_auth="$7" copilot_ver="$8"
  local ref_sha="${9:-}" ref_claude="${10:-}"

  # Determine issues
  local sha_ok="ok" claude_ver_ok="ok" claude_auth_ok="ok" gh_auth_ok="ok" copilot_ok="ok"

  [[ "$sha" == "OFFLINE" ]] && { sha_ok="OFFLINE"; claude_ver_ok="-"; claude_auth_ok="-"; gh_auth_ok="-"; copilot_ok="-"; }
  [[ "$sha" != "OFFLINE" && "$sha" != "$ref_sha" ]] && sha_ok="DRIFT"
  [[ "$claude_ver" == "missing" ]] && claude_ver_ok="MISSING"
  [[ -n "$ref_claude" && "$claude_ver" != "missing" && "$claude_ver" != "$ref_claude" ]] && claude_ver_ok="OUTDATED"
  if [[ "$sha" != "OFFLINE" ]]; then
    if [[ "$claude_logged" != "true" ]]; then
      claude_auth_ok="NO-AUTH"
    else
      # Only claude.ai (OAuth via Max/Pro subscription) is acceptable
      case "$claude_method" in
        claude.ai) claude_auth_ok="ok" ;;
        api_key|oauth_token) claude_auth_ok="API-KEY" ;;
        *) claude_auth_ok="UNKNOWN" ;;
      esac
    fi
  fi
  [[ "$gh_auth" != "ok" && "$sha" != "OFFLINE" ]] && gh_auth_ok="NO-AUTH"
  [[ "$copilot_ver" == "missing" && "$sha" != "OFFLINE" ]] && copilot_ok="MISSING"

  # Count issues
  local node_issues=0
  for s in "$sha_ok" "$claude_ver_ok" "$claude_auth_ok" "$gh_auth_ok" "$copilot_ok"; do
    [[ "$s" != "ok" && "$s" != "-" ]] && node_issues=$((node_issues + 1))
  done
  ISSUES=$((ISSUES + node_issues))

  if $JSON; then
    local entry
    entry=$(printf '{"node":"%s","sha":"%s","branch":"%s","claude_ver":"%s","claude_auth":"%s","claude_method":"%s","gh_auth":"%s","copilot_ver":"%s","issues":%d}' \
      "$peer" "$sha" "$branch" "$claude_ver" "$claude_auth_ok" "$claude_method" "$gh_auth_ok" "$copilot_ver" "$node_issues")
    RESULTS_JSON=$(echo "$RESULTS_JSON" | sed "s/]$/,$entry]/;s/\[,/[/")
  else
    # Color codes
    local G='\033[0;32m' Y='\033[1;33m' R='\033[0;31m' N='\033[0m'
    _c() { [[ "$1" == "ok" ]] && printf "${G}%s${N}" "$1" || printf "${R}%s${N}" "$1"; }

    printf "%-10s %-9s %-6s %-20s %-10s %-10s %s\n" \
      "$peer" "$sha($(_c "$sha_ok"))" "$branch" \
      "$claude_ver($(_c "$claude_auth_ok"))" \
      "$(_c "$gh_auth_ok")" "$(_c "$copilot_ok")" "$copilot_ver"
  fi
}

# Write JSON results to data dir for dashboard
_write_dashboard_data() {
  local ts data_dir data_file
  ts=$(date -u +%Y-%m-%dT%H:%M:%SZ)
  data_dir="$HOME/.claude/data"
  data_file="$data_dir/mesh-preflight.json"
  mkdir -p "$data_dir"
  printf '{"timestamp":"%s","ref_sha":"%s","ref_claude":"%s","nodes":%s,"total_issues":%d}\n' \
    "$ts" "$LOCAL_SHA" "$LOCAL_CLAUDE" "$RESULTS_JSON" "$ISSUES" > "$data_file"
}

# Header
if ! $JSON; then
  printf "%-10s %-9s %-6s %-20s %-10s %-10s %s\n" \
    "NODE" "COMMIT" "BRANCH" "CLAUDE(AUTH)" "GH-AUTH" "COPILOT" "COP-VER"
  printf "%-10s %-9s %-6s %-20s %-10s %-10s %s\n" \
    "----" "------" "------" "------------" "-------" "-------" "-------"
fi

# Run checks
if [[ -n "$TARGET_PEER" ]]; then
  _check_node "$TARGET_PEER"
else
  for peer in $(peers_list); do
    _check_node "$peer"
  done
fi

# Always write JSON for dashboard
SAVE_JSON=$JSON
JSON=true
if ! $SAVE_JSON; then
  # Re-run in JSON mode silently for dashboard data
  RESULTS_JSON="[]"
  ISSUES=0
  for peer in $(peers_list); do
    _check_node "$peer"
  done
fi
_write_dashboard_data

if $SAVE_JSON; then
  echo "$RESULTS_JSON"
fi

exit $( [[ $ISSUES -gt 0 ]] && echo 1 || echo 0 )
