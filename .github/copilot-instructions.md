# ConvergioPlatform

Unified control plane: Rust daemon + dashboard + evolution engine + local LLM.

## Structure

| Layer | Path | Lang |
|---|---|---|
| Daemon | `daemon/` | Rust — mesh P2P, API, TUI, IPC, DB |
| Dashboard | `dashboard/` | Python+JS — Control Room |
| Evolution | `evolution/` | TypeScript — self-improving engine |
| Scripts | `scripts/` | Bash — mesh(12), platform(6), llm(4) |
| Config | `config/` | Platform + LLM config |
| Data | `data/` | SQLite WAL |

## Agents

| Agent | File | Role |
|---|---|---|
| @Convergio | `.github/agents/Convergio.agent.md` | Platform (daemon, mesh, dashboard) |
| @ConvergioLLM | `.github/agents/ConvergioLLM.agent.md` | Local LLM (oMLX, LiteLLM, models) |

## Local LLM

| Command | What |
|---|---|
| `convergio-llm.sh start ~/models/<m>` | Start local inference |
| `convergio-llm.sh stop` | Stop |
| `convergio-llm.sh status` | Check |
| `claude-local` | Claude Code with local models |
| `claude` | Claude Code with cloud API |

Config: `config/llm/litellm.yaml` | Models: `config/llm/models.yaml` | Docs: `docs/llm/README.md`

## Conventions

Max 250 lines/file | English only | Rust: fmt+clippy | JS: vanilla+Maranello DS | TS: strict | Bash: `set -euo pipefail` | Comments: WHY not WHAT | Mesh: Tailscale+HMAC-SHA256
