#!/bin/bash
set -euo pipefail

# convergio-llm-download.sh — Download MLX/AWQ/GGUF models from HuggingFace
# Usage: convergio-llm-download.sh <hf_repo> [local_name]
#
# Example:
#   convergio-llm-download.sh mlx-community/Qwen2.5-Coder-32B-Instruct-4bit qwen2.5-coder-32b

HF_REPO="${1:?Usage: convergio-llm-download.sh <hf_repo> [local_name]}"
LOCAL_NAME="${2:-$(basename "$HF_REPO")}"
MODEL_DIR="$HOME/models/$LOCAL_NAME"
VENV="$HOME/llm-local"

if [ ! -d "$VENV" ]; then
  echo "ERROR: venv not found. Run: convergio-llm.sh setup"
  exit 1
fi

source "$VENV/bin/activate"

echo "Downloading: $HF_REPO"
echo "To:          $MODEL_DIR"
echo ""

python3 -c "
from huggingface_hub import snapshot_download
snapshot_download(
    repo_id='$HF_REPO',
    local_dir='$MODEL_DIR',
    local_dir_use_symlinks=False,
)
print()
print('Download complete: $MODEL_DIR')
"

echo ""
echo "Next: convergio-llm.sh start $MODEL_DIR"
