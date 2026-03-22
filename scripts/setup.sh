#!/usr/bin/env bash
# setup.sh — MyConvergio multi-provider bootstrap
# Detects installed AI providers (Claude Code, Copilot CLI, generic LLM) and
# syncs skills/agents for each via daemon API or fallback transpiler scripts.
# Usage: bash scripts/setup.sh [--dry-run] [--provider <name>] [--help]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLATFORM_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
CONFIG_SCRIPTS="$PLATFORM_DIR/claude-config/scripts"
MANIFEST_DIR="$HOME/.convergio"
MANIFEST_FILE="$MANIFEST_DIR/install-manifest.json"
DAEMON_HEALTH="http://localhost:8420/api/health"

DRY_RUN=0
FORCE_PROVIDER=""
ROLLBACK=0

# Cleanup trap — no temp files to remove, but required by coding standards
cleanup() { :; }
trap cleanup EXIT

usage() {
  echo "Usage: setup.sh [--dry-run] [--rollback] [--provider claude-code|copilot-cli|generic-llm] [--help]"
  echo "MyConvergio multi-provider bootstrap. Auto-detects Claude Code and Copilot CLI."
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --dry-run)   DRY_RUN=1; shift ;;
      --rollback)  ROLLBACK=1; shift ;;
      --provider)
        if [[ $# -lt 2 ]]; then
          echo "[ERROR] --provider requires an argument" >&2
          exit 1
        fi
        FORCE_PROVIDER="$2"
        shift 2
        ;;
      --help)      usage; exit 0 ;;
      *)
        echo "[ERROR] Unknown option: $1" >&2
        usage >&2
        exit 1
        ;;
    esac
  done
}

validate_provider() {
  local name="$1"
  case "$name" in
    claude-code|copilot-cli|generic-llm) return 0 ;;
    *)
      echo "[ERROR] Unknown provider: '$name'. Valid: claude-code | copilot-cli | generic-llm" >&2
      exit 1
      ;;
  esac
}

# Returns 0 if daemon is reachable, 1 otherwise
daemon_available() {
  curl -sf "$DAEMON_HEALTH" &>/dev/null
}

collect_provider_files() {
  local provider="$1" files=()
  case "$provider" in
    claude-code)  [[ -d "$HOME/.claude" ]] && files+=("$HOME/.claude") ;;
    copilot-cli)  [[ -d "$HOME/.copilot" ]] && files+=("$HOME/.copilot") ;;
    generic-llm)  ;;
  esac
  local json="[" first=1
  for f in "${files[@]+"${files[@]}"}"; do
    [[ $first -eq 0 ]] && json+=", "; json+="\"$f\""; first=0
  done
  echo "${json}]"
}

sync_provider() {
  local provider="$1"

  if daemon_available; then
    echo "  [daemon] cvg agent sync --provider $provider"
    if [[ $DRY_RUN -eq 0 ]]; then
      # Daemon sync — non-fatal if agent sync subcommand not yet implemented
      cvg agent sync --provider "$provider" 2>/dev/null \
        || echo "  [WARN] daemon sync unavailable for $provider — falling back to transpiler"
    fi
  fi

  # Always run transpiler if available (idempotent, file-based)
  local transpiler_map
  case "$provider" in
    claude-code)  transpiler_map="skill-transpile-claude.sh" ;;
    copilot-cli)  transpiler_map="skill-transpile-copilot.sh" ;;
    generic-llm)  transpiler_map="skill-transpile-generic.sh" ;;
    *)            return 0 ;;
  esac

  local transpiler="$CONFIG_SCRIPTS/$transpiler_map"
  if [[ -x "$transpiler" ]]; then
    echo "  [transpile] $transpiler_map"
    if [[ $DRY_RUN -eq 0 ]]; then
      bash "$transpiler"
    fi
  else
    echo "  [SKIP] transpiler not found: $transpiler_map"
  fi
}

detect_providers() {
  local providers=()

  if [[ -n "$FORCE_PROVIDER" ]]; then
    # Already validated in main() — just use the provider
    providers=("$FORCE_PROVIDER")
  else
    # Auto-detect: Claude Code via ~/.claude/
    if [[ -d "$HOME/.claude" ]]; then
      providers+=("claude-code")
    fi
    # Auto-detect: Copilot CLI via ~/.copilot/ or 'copilot' command
    if [[ -d "$HOME/.copilot" ]] || command -v copilot &>/dev/null; then
      providers+=("copilot-cli")
    fi
    # Generic LLM: only explicit via --provider
  fi

  printf '%s\n' "${providers[@]+"${providers[@]}"}"
}

build_manifest_json() {
  local providers_arr=("$@")
  local timestamp
  timestamp="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

  local providers_json="["
  local first=1
  for p in "${providers_arr[@]+"${providers_arr[@]}"}"; do
    local files_json
    files_json="$(collect_provider_files "$p")"
    [[ $first -eq 0 ]] && providers_json+=", "
    providers_json+="{\"name\": \"$p\", \"files\": $files_json}"
    first=0
  done
  providers_json+="]"

  cat <<EOF
{
  "installed_at": "$timestamp",
  "providers": $providers_json
}
EOF
}

do_rollback() {
  echo "=== MyConvergio Rollback ==="
  if [[ ! -f "$MANIFEST_FILE" ]]; then
    echo "[ERROR] No manifest found at $MANIFEST_FILE — nothing to rollback" >&2
    exit 1
  fi

  echo "Reading manifest: $MANIFEST_FILE"

  if [[ $DRY_RUN -eq 1 ]]; then
    echo ""
    echo "--- Dry-run rollback (no changes made) ---"
    echo "Would remove files listed in manifest:"
    cat "$MANIFEST_FILE"
    echo ""
    echo "Would unlink: $MANIFEST_FILE"
    echo "--------------------------------------------"
    return 0
  fi

  # Remove manifest and manifest dir (if empty)
  rm -f "$MANIFEST_FILE"
  rmdir "$MANIFEST_DIR" 2>/dev/null || true
  echo "[OK] Manifest removed: $MANIFEST_FILE"
  echo ""
  echo "=== Rollback Complete ==="
}

main() {
  parse_args "$@"

  # Validate --provider arg early, in main shell (not subshell)
  if [[ -n "$FORCE_PROVIDER" ]]; then
    validate_provider "$FORCE_PROVIDER"
  fi

  # Handle rollback
  if [[ $ROLLBACK -eq 1 ]]; then
    do_rollback
    return 0
  fi

  echo "=== MyConvergio Setup ==="

  # Collect active providers (bash 3.x compatible — no mapfile)
  local active_providers=()
  while IFS= read -r _p; do
    [[ -n "$_p" ]] && active_providers+=("$_p")
  done < <(detect_providers)

  if [[ ${#active_providers[@]} -eq 0 ]]; then
    echo "[WARN] No providers detected. Use --provider to force one."
    echo "  Supported: claude-code | copilot-cli | generic-llm"
  fi

  local manifest_json
  manifest_json="$(build_manifest_json "${active_providers[@]+"${active_providers[@]}"}")"

  if [[ $DRY_RUN -eq 1 ]]; then
    echo ""
    echo "--- Dry-run manifest (no changes made) ---"
    echo "$manifest_json"
    echo "------------------------------------------"
    return 0
  fi

  # Execute sync for each provider
  for provider in "${active_providers[@]+"${active_providers[@]}"}"; do
    echo ""
    echo "Provider: $provider"
    sync_provider "$provider"
  done

  # Write install manifest
  mkdir -p "$MANIFEST_DIR"
  echo "$manifest_json" > "$MANIFEST_FILE"
  echo ""
  echo "[OK] Install manifest: $MANIFEST_FILE"
  echo ""
  echo "=== Setup Complete ==="
}

main "$@"
