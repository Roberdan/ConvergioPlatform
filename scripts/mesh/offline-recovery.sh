#!/usr/bin/env bash
# offline-recovery.sh — Apply pending commits when a peer reconnects to the mesh
# Version: 1.0.0
# Usage: offline-recovery.sh [--check-only] [--help]
#
# When a peer comes back online (heartbeat resumes), this script:
#   1. Checks .git/pending-patches/ for unsynced commits saved by post-commit-sync
#   2. Applies patches via git am (in order)
#   3. Reports recovery status to the daemon API
#
# Environment:
#   GIT_DIR    — path used as .git dir (default: auto-detect from cwd or $HOME/.claude)
#   DAEMON_API — daemon API base URL (default: http://localhost:8420)
set -euo pipefail

DAEMON_API="${DAEMON_API:-http://localhost:8420}"
CHECK_ONLY=false

C='\033[0;36m' G='\033[0;32m' Y='\033[1;33m' R='\033[0;31m' N='\033[0m'
info() { echo -e "${C}[offline-recovery]${N} $*"; }
ok()   { echo -e "${G}[offline-recovery]${N} $*"; }
warn() { echo -e "${Y}[offline-recovery]${N} $*" >&2; }
err()  { echo -e "${R}[offline-recovery]${N} $*" >&2; }

_usage() {
  cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Applies pending git patches when a peer reconnects to the mesh.

Options:
  --check-only   Detect pending patches without applying them
  --help, -h     Show this usage message

Environment:
  GIT_DIR        Override .git directory location
  DAEMON_API     Daemon API base URL (default: http://localhost:8420)

Examples:
  $(basename "$0")             # apply any pending patches
  $(basename "$0") --check-only  # just report patch count, do not apply
EOF
}

_resolve_git_dir() {
  # Use explicit GIT_DIR if set; otherwise walk up from cwd; fallback to ~/.claude
  if [[ -n "${GIT_DIR:-}" ]]; then
    echo "$GIT_DIR"
    return
  fi
  local d="$PWD"
  while [[ "$d" != "/" ]]; do
    if [[ -d "$d/.git" ]]; then
      echo "$d/.git"
      return
    fi
    d="$(dirname "$d")"
  done
  # Fallback: treat $HOME/.claude as a pseudo git dir for patch storage
  echo "$HOME/.claude"
}

_patches_dir() {
  local git_dir="$1"
  echo "${git_dir}/pending-patches"
}

_check_pending() {
  local patches_dir="$1"
  if [[ ! -d "$patches_dir" ]]; then
    info "no pending patches (directory absent: $patches_dir)"
    return 0
  fi
  # Count *.patch files
  local count
  count="$(find "$patches_dir" -maxdepth 1 -name '*.patch' 2>/dev/null | wc -l | tr -d ' ')"
  if [[ "$count" -eq 0 ]]; then
    info "no pending patches in $patches_dir"
    return 0
  fi
  info "found ${count} pending patch(es) in $patches_dir"
  return 0
}

_apply_patches() {
  local patches_dir="$1"
  if [[ ! -d "$patches_dir" ]]; then
    info "no pending patches (directory absent)"
    return 0
  fi
  local applied=0 failed=0
  # Apply in sorted order so commits replay in original sequence
  while IFS= read -r patch_file; do
    [[ -z "$patch_file" ]] && continue
    info "applying: $(basename "$patch_file")"
    if git am --ignore-whitespace "$patch_file" 2>/dev/null; then
      ok "applied: $(basename "$patch_file")"
      rm -f "$patch_file"
      applied=$((applied + 1))
    else
      warn "failed to apply: $(basename "$patch_file") — aborting git am"
      git am --abort 2>/dev/null || true
      failed=$((failed + 1))
      # Stop on first failure to preserve patch order
      break
    fi
  done < <(find "$patches_dir" -maxdepth 1 -name '*.patch' 2>/dev/null | sort)

  if [[ $applied -gt 0 ]]; then
    ok "recovery complete: ${applied} patch(es) applied"
  fi
  if [[ $failed -gt 0 ]]; then
    err "recovery partial: ${failed} patch(es) failed — manual intervention needed"
    return 1
  fi
  return 0
}

_report_to_daemon() {
  local status="$1" applied="${2:-0}" failed="${3:-0}"
  local peer_name
  peer_name="$(hostname -s 2>/dev/null || hostname)"
  local payload
  payload="{\"peer\":\"${peer_name}\",\"status\":\"${status}\",\"applied\":${applied},\"failed\":${failed}}"
  # Non-fatal: daemon may not be running
  curl -sf -X POST \
    -H "Content-Type: application/json" \
    -d "$payload" \
    "${DAEMON_API}/api/mesh/recovery" >/dev/null 2>&1 || true
}

# Parse arguments
for arg in "$@"; do
  case "$arg" in
    --check-only) CHECK_ONLY=true ;;
    --help | -h)  _usage; exit 0 ;;
    *) err "Unknown argument: $arg"; _usage >&2; exit 1 ;;
  esac
done

# Main
GIT_DIR_PATH="$(_resolve_git_dir)"
PATCHES_DIR="$(_patches_dir "$GIT_DIR_PATH")"

if [[ "$CHECK_ONLY" == true ]]; then
  _check_pending "$PATCHES_DIR"
  exit 0
fi

# Full recovery mode
if [[ ! -d "$PATCHES_DIR" ]] || [[ -z "$(find "$PATCHES_DIR" -maxdepth 1 -name '*.patch' 2>/dev/null)" ]]; then
  info "no pending patches — nothing to recover"
  _report_to_daemon "clean" 0 0
  exit 0
fi

count="$(find "$PATCHES_DIR" -maxdepth 1 -name '*.patch' 2>/dev/null | wc -l | tr -d ' ')"
info "starting offline recovery: ${count} patch(es) pending"

if _apply_patches "$PATCHES_DIR"; then
  applied="$(( count - $(find "$PATCHES_DIR" -maxdepth 1 -name '*.patch' 2>/dev/null | wc -l | tr -d ' ') ))"
  _report_to_daemon "recovered" "$applied" 0
else
  applied="$(( count - $(find "$PATCHES_DIR" -maxdepth 1 -name '*.patch' 2>/dev/null | wc -l | tr -d ' ') ))"
  remaining="$(find "$PATCHES_DIR" -maxdepth 1 -name '*.patch' 2>/dev/null | wc -l | tr -d ' ')"
  _report_to_daemon "partial" "$applied" "$remaining"
  exit 1
fi
