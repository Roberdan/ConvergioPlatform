#!/bin/bash
set -euo pipefail

# convergio-llm.sh — Local LLM server management (multi-platform)
# Part of ConvergioPlatform. Manages local inference on any machine.
#
# Backends: oMLX (macOS), vLLM (Linux NVIDIA), llama.cpp (Linux CPU/Intel Mac)
# Proxy: LiteLLM (all platforms)
#
# Usage: convergio-llm.sh {start|stop|status|test|models|logs|setup}

# Resolve real path even when called via symlink
REAL_SCRIPT="$(readlink -f "$0" 2>/dev/null || python3 -c "import os; print(os.path.realpath('$0'))")"
SCRIPT_DIR="$(cd "$(dirname "$REAL_SCRIPT")" && pwd)"
PLATFORM_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
VENV="$HOME/llm-local"
LITELLM_CONFIG="$PLATFORM_ROOT/config/llm/litellm.yaml"
LITELLM_PORT=4000
BACKEND_PORT=8321
RUNTIME_DIR="$HOME/.convergio-llm"
LITELLM_PID="$RUNTIME_DIR/litellm.pid"
BACKEND_PID="$RUNTIME_DIR/backend.pid"
LOG_DIR="$RUNTIME_DIR/logs"

# Detect backend from saved platform info or live detection
detect_backend() {
  if [ -f "$RUNTIME_DIR/backend" ]; then
    cat "$RUNTIME_DIR/backend"
  elif [ "$(uname -s)" = "Darwin" ]; then
    echo "omlx"
  elif command -v nvidia-smi &>/dev/null && nvidia-smi &>/dev/null; then
    echo "vllm"
  else
    echo "llama-cpp"
  fi
}

BACKEND="$(detect_backend)"

ensure_dirs() {
  mkdir -p "$RUNTIME_DIR" "$LOG_DIR"
}

activate_venv() {
  if [ ! -d "$VENV" ]; then
    echo "ERROR: venv not found at $VENV"
    echo "Run: convergio-llm.sh setup"
    exit 1
  fi
  source "$VENV/bin/activate"
}

is_running() {
  local pidfile="$1"
  [ -f "$pidfile" ] && kill -0 "$(cat "$pidfile")" 2>/dev/null
}

start_backend() {
  local model_dir="$1"
  if is_running "$BACKEND_PID"; then
    echo "Backend ($BACKEND) already running (PID $(cat "$BACKEND_PID"))"
    return
  fi
  if [ ! -d "$model_dir" ]; then
    echo "ERROR: model dir not found: $model_dir"
    exit 1
  fi
  activate_venv
  echo "Starting $BACKEND on :$BACKEND_PORT — model: $(basename "$model_dir")"

  case "$BACKEND" in
    omlx)
      nohup omlx serve \
        --model-dir "$model_dir" \
        --port "$BACKEND_PORT" \
        > "$LOG_DIR/backend.log" 2>&1 &
      ;;
    vllm)
      nohup python3 -m vllm.entrypoints.openai.api_server \
        --model "$model_dir" \
        --port "$BACKEND_PORT" \
        --host 0.0.0.0 \
        --trust-remote-code \
        > "$LOG_DIR/backend.log" 2>&1 &
      ;;
    llama-cpp)
      nohup python3 -m llama_cpp.server \
        --model "$model_dir"/*.gguf \
        --port "$BACKEND_PORT" \
        --host 0.0.0.0 \
        > "$LOG_DIR/backend.log" 2>&1 &
      ;;
    *)
      echo "ERROR: unknown backend: $BACKEND"
      exit 1
      ;;
  esac

  echo $! > "$BACKEND_PID"
  echo "$BACKEND started (PID $!)"
}

start_litellm() {
  if is_running "$LITELLM_PID"; then
    echo "LiteLLM already running (PID $(cat "$LITELLM_PID"))"
    return
  fi
  if [ ! -f "$LITELLM_CONFIG" ]; then
    echo "ERROR: LiteLLM config not found: $LITELLM_CONFIG"
    exit 1
  fi
  activate_venv
  echo "Starting LiteLLM proxy on :$LITELLM_PORT"
  nohup litellm \
    --config "$LITELLM_CONFIG" \
    --port "$LITELLM_PORT" \
    --host 0.0.0.0 \
    > "$LOG_DIR/litellm.log" 2>&1 &
  echo $! > "$LITELLM_PID"
  echo "LiteLLM started (PID $!)"
}

stop_service() {
  local name="$1" pidfile="$2"
  if is_running "$pidfile"; then
    local pid
    pid=$(cat "$pidfile")
    kill "$pid"
    rm -f "$pidfile"
    echo "$name stopped (PID $pid)"
  else
    [ -f "$pidfile" ] && rm -f "$pidfile"
    echo "$name not running"
  fi
}

check_service() {
  local name="$1" url="$2"
  if curl -s --max-time 2 "$url" > /dev/null 2>&1; then
    echo "  $name: UP ($url)"
  else
    echo "  $name: DOWN ($url)"
  fi
}

CMD="${1:-status}"

case "$CMD" in
  start)
    ensure_dirs
    MODEL_DIR="${2:-}"
    if [ -n "$MODEL_DIR" ]; then
      start_backend "$MODEL_DIR"
    else
      echo "No model dir — skipping backend"
      echo "  Usage: convergio-llm.sh start ~/GitHub/LocalModels/<model>"
    fi
    start_litellm
    echo ""
    echo "Use 'claude-local' for local models, 'claude' for cloud."
    ;;

  stop)
    stop_service "$BACKEND" "$BACKEND_PID"
    stop_service "LiteLLM" "$LITELLM_PID"
    ;;

  restart)
    "$0" stop
    sleep 1
    "$0" start "${2:-}"
    ;;

  status)
    echo "=== Convergio LLM Status ==="
    echo "  Backend: $BACKEND"
    check_service "$BACKEND" "http://localhost:$BACKEND_PORT/v1/models"
    check_service "LiteLLM" "http://localhost:$LITELLM_PORT/health"
    if is_running "$BACKEND_PID"; then
      echo "  $BACKEND PID: $(cat "$BACKEND_PID")"
    fi
    if is_running "$LITELLM_PID"; then
      echo "  LiteLLM PID: $(cat "$LITELLM_PID")"
    fi
    echo ""
    echo "Config: $LITELLM_CONFIG"
    echo "Logs:   $LOG_DIR/"
    ;;

  test)
    activate_venv
    echo "Testing LiteLLM proxy..."
    curl -s "http://localhost:$LITELLM_PORT/v1/chat/completions" \
      -H "Authorization: Bearer ${LITELLM_MASTER_KEY:-sk-local}" \
      -H "Content-Type: application/json" \
      -d '{"model":"claude-sonnet","messages":[{"role":"user","content":"Say hello in 5 words"}],"max_tokens":20}' \
      | python3 -m json.tool
    ;;

  models)
    echo "=== Local Models ($BACKEND) ==="
    curl -s "http://localhost:$BACKEND_PORT/v1/models" 2>/dev/null \
      | python3 -c "import sys,json; [print(f'  {m[\"id\"]}') for m in json.load(sys.stdin).get('data',[])]" \
      2>/dev/null || echo "  ($BACKEND not running)"
    echo ""
    echo "=== Proxy Models (LiteLLM) ==="
    curl -s "http://localhost:$LITELLM_PORT/v1/models" \
      -H "Authorization: Bearer ${LITELLM_MASTER_KEY:-sk-local}" 2>/dev/null \
      | python3 -c "import sys,json; [print(f'  {m[\"id\"]}') for m in json.load(sys.stdin).get('data',[])]" \
      2>/dev/null || echo "  (LiteLLM not running)"
    ;;

  logs)
    SERVICE="${2:-litellm}"
    case "$SERVICE" in
      backend|omlx|vllm|llama*) LOGFILE="$LOG_DIR/backend.log" ;;
      *) LOGFILE="$LOG_DIR/${SERVICE}.log" ;;
    esac
    if [ -f "$LOGFILE" ]; then
      tail -50 "$LOGFILE"
    else
      echo "No logs at $LOGFILE"
    fi
    ;;

  setup)
    echo "Running setup..."
    bash "$SCRIPT_DIR/convergio-llm-setup.sh"
    ;;

  *)
    echo "convergio-llm.sh — Local LLM server management"
    echo ""
    echo "Usage: convergio-llm.sh <command> [args]"
    echo ""
    echo "Commands:"
    echo "  start [model-dir]   Start backend + LiteLLM proxy"
    echo "  stop                Stop all services"
    echo "  restart [model-dir] Restart all services"
    echo "  status              Show service status"
    echo "  test                Send test request via proxy"
    echo "  models              List available models"
    echo "  logs [backend|litellm] Show recent logs"
    echo "  setup               Install/update backend + LiteLLM"
    echo ""
    echo "Backend: $BACKEND (detected)"
    ;;
esac
