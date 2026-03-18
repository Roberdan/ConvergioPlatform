# Convergio LLM

Local inference on any machine. Platform-specific backend + LiteLLM proxy. Claude Code and VS Code use the proxy.

## Architecture

```
claude-local / Continue.dev / agents → LiteLLM (:4000) → Backend (:8321) | Cloud APIs
```

| Platform | Backend | Accelerator |
|---|---|---|
| macOS (Apple Silicon) | oMLX (MLX) | Metal GPU, unified memory |
| Linux (NVIDIA) | vLLM | CUDA |
| Linux (no GPU) | llama.cpp (server) | CPU AVX2/AVX512 |
| Windows | WSL2 + vLLM or llama.cpp | CUDA via WSL2 |

LiteLLM proxy is the same on all platforms — only the backend changes.

## Files (in repo)

| Path | What |
|---|---|
| `scripts/llm/convergio-llm.sh` | CLI: start/stop/status/test/models/logs |
| `scripts/llm/convergio-llm-setup.sh` | Bootstrap: detect OS, install backend + LiteLLM |
| `scripts/llm/convergio-llm-download.sh` | Download models (format per platform) |
| `scripts/llm/setup-llm-symlinks.sh` | Wire symlinks (idempotent) |
| `config/llm/litellm.yaml` | Proxy routing (local + cloud) |
| `config/llm/models.yaml` | Model catalog per platform |
| `config/llm/continue-config.json` | Continue.dev config |

## External (not in repo, created by setup)

| Path | What |
|---|---|
| `~/llm-local/` | Python venv (backend + LiteLLM) |
| `~/GitHub/LocalModels/` | Downloaded models |
| `~/.convergio-llm/` | PIDs + logs |
| `~/bin/convergio-llm*.sh` | Symlinks to scripts/llm/ |
| `~/.continue/config.json` | Symlink to config/llm/ |

## Commands (all platforms)

| Command | What |
|---|---|
| `convergio-llm.sh setup` | Install backend + LiteLLM |
| `convergio-llm.sh start ~/GitHub/LocalModels/<m>` | Start backend + proxy |
| `convergio-llm.sh stop` | Stop all |
| `convergio-llm.sh status` | Service status |
| `convergio-llm.sh models` | List available models |
| `convergio-llm.sh test` | Test proxy |
| `convergio-llm.sh logs [backend\|litellm]` | View logs |
| `convergio-llm-download.sh <hf_repo> <name>` | Download model |

## Setup per Platform

### macOS (Apple Silicon)

Requires: Homebrew, Python 3.10+

```bash
scripts/llm/setup-llm-symlinks.sh        # symlinks
convergio-llm.sh setup                    # installs oMLX + LiteLLM
convergio-llm-download.sh mlx-community/Qwen2.5-Coder-32B-Instruct-4bit qwen2.5-coder-32b
convergio-llm.sh start ~/GitHub/LocalModels/qwen2.5-coder-32b
claude-local
```

### Linux (NVIDIA GPU)

Requires: Python 3.10+, CUDA 12+, NVIDIA driver 535+

```bash
scripts/llm/setup-llm-symlinks.sh        # symlinks
convergio-llm.sh setup                    # detects NVIDIA → installs vLLM
convergio-llm-download.sh Qwen/Qwen2.5-Coder-32B-Instruct-AWQ qwen2.5-coder-32b
convergio-llm.sh start ~/GitHub/LocalModels/qwen2.5-coder-32b
claude-local
```

### Linux (no GPU / CPU only)

Requires: Python 3.10+, cmake, gcc

```bash
scripts/llm/setup-llm-symlinks.sh        # symlinks
convergio-llm.sh setup                    # detects no GPU → installs llama.cpp
convergio-llm-download.sh bartowski/Qwen2.5-Coder-7B-Instruct-GGUF qwen2.5-coder-7b
convergio-llm.sh start ~/GitHub/LocalModels/qwen2.5-coder-7b
claude-local
```

### Windows

**Option A — WSL2** (recommended):
```powershell
wsl --install -d Ubuntu-24.04
# Inside WSL2: follow Linux instructions above
```

**Option B — LM Studio** (native):
1. Install LM Studio from lmstudio.ai
2. Download model in UI, start local server (:1234)
3. Edit `config/llm/litellm.yaml`: change `api_base` to `localhost:1234`
4. `pip install litellm[proxy]` + `litellm --config config/llm/litellm.yaml --port 4000`
5. `set ANTHROPIC_BASE_URL=http://localhost:4000` + `claude`

## Integration

| Tool | Cloud | Local |
|---|---|---|
| Claude Code | `claude` | `claude-local` |
| Continue.dev | Select cloud model | Select local model |
| Copilot Chat | `@ConvergioLLM` agent | N/A |
| Any script | default | `ANTHROPIC_BASE_URL=http://localhost:4000` |

## Ports

4000 = LiteLLM proxy | 8321 = Backend (oMLX/vLLM/llama.cpp) | 1234 = LM Studio (Win native)

## Model Format per Platform

| Platform | Format | HF repo pattern |
|---|---|---|
| macOS | MLX 4-bit | `mlx-community/*-4bit` |
| Linux NVIDIA | AWQ/GPTQ | `*-AWQ` or `*-GPTQ-Int4` |
| Linux CPU | GGUF | `*-GGUF` (Q4_K_M) |
| Windows WSL2 | Same as Linux | Same as Linux |
| Windows native | GGUF (LM Studio) | `*-GGUF` |

## Troubleshooting

| Problem | Fix |
|---|---|
| `venv not found` | `convergio-llm.sh setup` |
| Backend won't start | Check model dir + `convergio-llm.sh logs backend` |
| LiteLLM won't start | Check litellm.yaml + `convergio-llm.sh logs litellm` |
| `claude-local` hangs | Both services must be UP |
| Port conflict | `lsof -i :4000` (mac/linux) or `netstat -ano \| findstr 4000` (win) |
| CUDA out of memory | Smaller quant or smaller model |
| MLX not found | macOS Apple Silicon only |
| vLLM import error | Check CUDA: `nvidia-smi` |
