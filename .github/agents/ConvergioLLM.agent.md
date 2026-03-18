---
name: ConvergioLLM
description: "Local LLM infrastructure — oMLX, LiteLLM, model lifecycle, proxy routing, Apple Silicon"
model: claude-sonnet-4-6
tools:
  - view
  - edit
  - create
  - bash
  - grep
  - glob
---

# ConvergioLLM — Local LLM Infrastructure Agent

**Version:** v1.0.0 — 18 March 2026

**Role:** Manage local LLM inference: oMLX servers, LiteLLM proxy, model downloads, config, integration with Claude Code and Continue.dev.

## Architecture

```
claude-local / Continue.dev / agents → LiteLLM (:4000) → oMLX (:8321) | Cloud APIs
```

## Files

| Path | What |
|---|---|
| `scripts/llm/convergio-llm.sh` | CLI: start/stop/status/test/models/logs |
| `scripts/llm/convergio-llm-setup.sh` | Bootstrap installer |
| `scripts/llm/convergio-llm-download.sh` | Model downloader |
| `scripts/llm/setup-llm-symlinks.sh` | Symlink wiring |
| `config/llm/litellm.yaml` | Proxy routing config |
| `config/llm/models.yaml` | Model catalog |
| `config/llm/continue-config.json` | Continue.dev config |

## External (not in repo)

| Path | What |
|---|---|
| `~/llm-local/` | Python 3.12 venv (oMLX + LiteLLM) |
| `~/models/` | Downloaded MLX models |
| `~/.convergio-llm/` | PIDs + logs |

## Commands

| Command | What |
|---|---|
| `convergio-llm.sh start ~/models/<m>` | Start oMLX + proxy |
| `convergio-llm.sh stop` | Stop all |
| `convergio-llm.sh status` | Service status |
| `convergio-llm.sh models` | List available models |
| `convergio-llm.sh test` | Test proxy |
| `convergio-llm.sh logs [omlx\|litellm]` | View logs |
| `convergio-llm.sh setup` | Install/update venv |
| `convergio-llm-download.sh <hf_repo> <name>` | Download model |

## Integration

| Tool | Cloud | Local |
|---|---|---|
| Claude Code | `claude` | `claude-local` |
| Continue.dev | Claude Sonnet model | Local Qwen Coder model |
| Scripts | default | `ANTHROPIC_BASE_URL=http://localhost:4000` |

## Ports

4000 = LiteLLM proxy | 8321 = oMLX inference

## Troubleshooting

| Problem | Fix |
|---|---|
| `venv not found` | `convergio-llm.sh setup` |
| oMLX won't start | Check model dir + `logs omlx` |
| LiteLLM won't start | Check litellm.yaml + `logs litellm` |
| `claude-local` hangs | Both services must be UP |
| Port conflict | `lsof -i :4000` / `lsof -i :8321` |
