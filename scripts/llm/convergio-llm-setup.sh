#!/bin/bash
set -euo pipefail

# convergio-llm-setup.sh — Bootstrap local LLM environment (multi-platform)
# Detects OS + GPU → installs correct backend + LiteLLM into ~/llm-local venv
# Safe to re-run (idempotent)

VENV="$HOME/llm-local"
OS="$(uname -s)"

echo "=== Convergio LLM Setup ==="
echo "OS: $OS"

# --- Detect platform ---
detect_platform() {
  case "$OS" in
    Darwin)
      if sysctl -n machdep.cpu.brand_string 2>/dev/null | grep -q "Apple"; then
        echo "macos-apple-silicon"
      else
        echo "macos-intel"
      fi
      ;;
    Linux)
      if command -v nvidia-smi &>/dev/null && nvidia-smi &>/dev/null; then
        echo "linux-nvidia"
      else
        echo "linux-cpu"
      fi
      ;;
    MINGW*|MSYS*|CYGWIN*)
      echo "windows"
      ;;
    *)
      echo "unknown"
      ;;
  esac
}

PLATFORM="$(detect_platform)"
echo "Platform: $PLATFORM"
echo ""

# --- Python ---
install_python() {
  case "$PLATFORM" in
    macos-*)
      if ! command -v /opt/homebrew/bin/python3.12 &>/dev/null; then
        echo "Installing Python 3.12 via Homebrew..."
        brew install python@3.12
      fi
      PYTHON="/opt/homebrew/bin/python3.12"
      ;;
    linux-*)
      if command -v python3.12 &>/dev/null; then
        PYTHON="python3.12"
      elif command -v python3.11 &>/dev/null; then
        PYTHON="python3.11"
      elif command -v python3.10 &>/dev/null; then
        PYTHON="python3.10"
      else
        echo "Installing Python 3.12..."
        if command -v apt-get &>/dev/null; then
          sudo apt-get update && sudo apt-get install -y python3.12 python3.12-venv
        elif command -v dnf &>/dev/null; then
          sudo dnf install -y python3.12
        fi
        PYTHON="python3.12"
      fi
      ;;
    windows)
      PYTHON="python3"
      ;;
  esac
  echo "Python: $($PYTHON --version)"
}

install_python

# --- Virtual environment ---
if [ ! -d "$VENV" ]; then
  echo "Creating venv at $VENV..."
  "$PYTHON" -m venv "$VENV"
else
  echo "Venv: OK ($VENV)"
fi

source "$VENV/bin/activate"
pip install --upgrade pip --quiet

# --- Backend (platform-specific) ---
case "$PLATFORM" in
  macos-apple-silicon)
    echo ""
    echo "=== Installing oMLX (Apple Silicon backend) ==="
    if ! python3 -c "import omlx" 2>/dev/null; then
      if [ ! -d /tmp/omlx ]; then
        git clone https://github.com/jundot/omlx.git /tmp/omlx
      fi
      pip install -e /tmp/omlx --quiet
    fi
    python3 -c "import omlx; print('oMLX: OK')"
    BACKEND="omlx"
    ;;

  macos-intel)
    echo ""
    echo "ERROR: Intel Mac not supported. oMLX requires Apple Silicon."
    echo "Use llama-cpp-python as fallback (slow)."
    pip install llama-cpp-python --quiet
    BACKEND="llama-cpp"
    ;;

  linux-nvidia)
    echo ""
    echo "=== Installing vLLM (NVIDIA backend) ==="
    if ! python3 -c "import vllm" 2>/dev/null; then
      pip install vllm --quiet
    fi
    python3 -c "import vllm; print('vLLM: OK')"
    BACKEND="vllm"
    ;;

  linux-cpu)
    echo ""
    echo "=== Installing llama-cpp-python (CPU backend) ==="
    if ! python3 -c "import llama_cpp" 2>/dev/null; then
      pip install 'llama-cpp-python[server]' --quiet
    fi
    python3 -c "import llama_cpp; print('llama-cpp-python: OK')"
    BACKEND="llama-cpp"
    ;;

  windows)
    echo ""
    echo "=== Windows: use WSL2 (recommended) or LM Studio ==="
    echo "For WSL2: run this script inside WSL2 Ubuntu"
    echo "For LM Studio: install from lmstudio.ai, no backend needed here"
    BACKEND="none"
    ;;
esac

# --- LiteLLM (all platforms) ---
echo ""
echo "=== Installing LiteLLM proxy ==="
if ! python3 -c "import litellm" 2>/dev/null; then
  pip install 'litellm[proxy]' --quiet
fi
python3 -c "import litellm; print('LiteLLM: OK')"

# --- Fix soundfile conflict if present ---
python3 -c "import soundfile" 2>/dev/null || pip install 'soundfile>=0.13.1' --quiet 2>/dev/null

# --- Directories ---
mkdir -p "$HOME/models" "$HOME/.convergio-llm/logs"

# --- Save platform info ---
echo "$PLATFORM" > "$HOME/.convergio-llm/platform"
echo "$BACKEND" > "$HOME/.convergio-llm/backend"

echo ""
echo "=== Setup Complete ==="
echo "Platform: $PLATFORM"
echo "Backend:  $BACKEND"
echo "Venv:     $VENV"
echo "Models:   ~/models/ (empty)"
echo "Logs:     ~/.convergio-llm/logs/"
echo ""
echo "Next:"
case "$PLATFORM" in
  macos-apple-silicon)
    echo "  convergio-llm-download.sh mlx-community/Qwen2.5-Coder-32B-Instruct-4bit qwen2.5-coder-32b"
    ;;
  linux-nvidia)
    echo "  convergio-llm-download.sh Qwen/Qwen2.5-Coder-32B-Instruct-AWQ qwen2.5-coder-32b"
    ;;
  linux-cpu)
    echo "  convergio-llm-download.sh bartowski/Qwen2.5-Coder-7B-Instruct-GGUF qwen2.5-coder-7b"
    ;;
esac
echo "  convergio-llm.sh start ~/models/<model>"
echo "  claude-local"
