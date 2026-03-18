#!/usr/bin/env bash
# Load local config. Source this at the top of scripts that need node/infra config.
# Usage: source "$(dirname "${BASH_SOURCE[0]}")/../../config/load-config.sh"  (adjust depth)

_CONFIG_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
_LOCAL_ENV="$_CONFIG_DIR/local.env"

if [ -f "$_LOCAL_ENV" ]; then
  set -a
  # shellcheck source=/dev/null
  source "$_LOCAL_ENV"
  set +a
else
  # Fallback: search common locations
  for _try in \
    "$HOME/GitHub/ConvergioPlatform/config/local.env" \
    "$_CONFIG_DIR/../../config/local.env"; do
    if [ -f "$_try" ]; then
      set -a
      # shellcheck source=/dev/null
      source "$_try"
      set +a
      break
    fi
  done
fi

# Warn if still unconfigured, but don't fail (backward compat)
if [ -z "${MESH_COORDINATOR_HOST:-}" ]; then
  echo "[WARN] No config/local.env found. Using environment defaults." >&2
fi

unset _CONFIG_DIR _LOCAL_ENV _try
