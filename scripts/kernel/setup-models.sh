#!/usr/bin/env bash
# Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
# Download all MLX models required by the Convergio kernel.
# Usage: ./scripts/kernel/setup-models.sh
# Models land in the standard HuggingFace cache: ~/.cache/huggingface/
set -euo pipefail

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

REQUIRED_GB=30
CACHE_DIR="${HF_HOME:-$HOME/.cache/huggingface}"

# Required models (name → HF repo)
declare -A REQUIRED_MODELS=(
  ["Mistral-7B-Instruct-4bit"]="mlx-community/Mistral-7B-Instruct-v0.3-4bit"
  ["Qwen2.5-7B-Instruct-4bit"]="mlx-community/Qwen2.5-7B-Instruct-4bit"
  ["Codestral-22B-4bit"]="mlx-community/Codestral-22B-v0.1-4bit"
  ["Whisper-small"]="mlx-community/whisper-small"
)

# Optional models — absence is a warning, not an error
declare -A OPTIONAL_MODELS=(
  ["Mistral-Small-3.1-4bit"]="mlx-community/Mistral-Small-3.1-24B-Instruct-2503-4bit"
  ["Voxtral-Mini-4bit"]="mlx-community/Voxtral-Mini-3B-2507-4bit"
)

DOWNLOADED=()
SKIPPED=()
WARNED=()

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

log()  { echo "[setup-models] $*"; }
warn() { echo "[setup-models] WARNING: $*" >&2; }
die()  { echo "[setup-models] ERROR: $*" >&2; exit 1; }

check_prereqs() {
  # python3
  if ! command -v python3 &>/dev/null; then
    die "python3 not found. Install Python 3.10+ before running this script."
  fi

  # pip
  if ! command -v pip3 &>/dev/null && ! python3 -m pip --version &>/dev/null 2>&1; then
    die "pip not found. Install pip before running this script."
  fi

  # huggingface-cli
  if ! command -v huggingface-cli &>/dev/null; then
    log "huggingface-cli not found — installing huggingface-hub via pip..."
    pip3 install --quiet huggingface-hub || \
      python3 -m pip install --quiet huggingface-hub || \
      die "Failed to install huggingface-hub."
    # Refresh PATH so the newly installed binary is visible
    if command -v huggingface-cli &>/dev/null; then
      log "huggingface-cli installed successfully."
    else
      die "huggingface-cli still not found after install. Add pip bin dir to PATH."
    fi
  fi
}

check_disk_space() {
  local available_kb
  available_kb=$(df -k "$HOME" | awk 'NR==2{print $4}')
  local available_gb=$(( available_kb / 1024 / 1024 ))
  log "Available disk space: ~${available_gb} GB (need ~${REQUIRED_GB} GB)"
  if (( available_gb < REQUIRED_GB )); then
    warn "Less than ${REQUIRED_GB} GB free (found ~${available_gb} GB). Download may fail."
  fi
}

download_model() {
  local label="$1"
  local repo="$2"
  local optional="${3:-false}"

  log "Downloading ${label} (${repo})..."
  if huggingface-cli download "${repo}" 2>&1; then
    DOWNLOADED+=("${label}")
    log "${label}: done."
  else
    if [[ "$optional" == "true" ]]; then
      warn "${label} (${repo}) not available or download failed — skipping (optional)."
      WARNED+=("${label}")
    else
      die "Failed to download required model: ${label} (${repo})"
    fi
  fi
}

print_summary() {
  echo ""
  echo "============================================================"
  echo "  setup-models: SUMMARY"
  echo "============================================================"

  if (( ${#DOWNLOADED[@]} > 0 )); then
    echo "  Downloaded (${#DOWNLOADED[@]}):"
    for m in "${DOWNLOADED[@]}"; do echo "    - $m"; done
  fi

  if (( ${#SKIPPED[@]} > 0 )); then
    echo "  Already cached (${#SKIPPED[@]}):"
    for m in "${SKIPPED[@]}"; do echo "    - $m"; done
  fi

  if (( ${#WARNED[@]} > 0 )); then
    echo "  Optional — not available (${#WARNED[@]}):"
    for m in "${WARNED[@]}"; do echo "    - $m"; done
  fi

  # Approximate total cache size
  local total_size
  total_size=$(du -sh "${CACHE_DIR}/hub" 2>/dev/null | awk '{print $1}' || echo "unknown")
  echo "  HuggingFace cache total: ${total_size}  (${CACHE_DIR}/hub)"
  echo "============================================================"
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

log "Starting MLX model setup..."
check_prereqs
check_disk_space

log "Required models: ${#REQUIRED_MODELS[@]}  |  Optional: ${#OPTIONAL_MODELS[@]}"

for label in "${!REQUIRED_MODELS[@]}"; do
  download_model "$label" "${REQUIRED_MODELS[$label]}" "false"
done

for label in "${!OPTIONAL_MODELS[@]}"; do
  download_model "$label" "${OPTIONAL_MODELS[$label]}" "true"
done

print_summary
log "Setup complete."
