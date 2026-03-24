<!-- v1.3.0 -->
<!-- Copyright (c) 2026 Roberto D'Angelo. MPL-2.0. -->
# ConvergioPlatform

Unified control plane: Rust daemon (107 modules) + dashboard + evolution engine.

## DB Access (NON-NEGOTIABLE)

NEVER use `sqlite3` directly. Hook-blocked. Use `cvg` CLI or daemon API:

| Instead of | Use |
|---|---|
| `sqlite3 dashboard.db "SELECT..."` | `cvg plan list` / `cvg plan show <id>` / `cvg project plans <id>` |
| `sqlite3 ... "UPDATE plans..."` | `cvg plan start <id>` / `cvg task update <id> done` |
| `sqlite3 ... "INSERT INTO..."` | `cvg plan create` / `cvg plan import` |
| Raw DB queries | `curl http://localhost:8420/api/...` |

## Agent Identity (NON-NEGOTIABLE)

```bash
cvg agent start "<type>-$(hostname -s)-$$"    # on start (type: claude, copilot, executor)
cvg agent complete "<type>-$(hostname -s)-$$"  # before /exit
```

## Governance (NON-NEGOTIABLE)

@CONSTITUTION.md
@AgenticManifesto.md
@LEGAL_NOTICE.md

## Commands

| Command | Purpose |
|---|---|
| `cd daemon && cargo build --release` | Build daemon |
| `cd daemon && cargo check` | Type check (~5s) |
| `cd daemon && cargo test` | Daemon tests |
| `cd daemon && cargo run -- tui` | Launch TUI |
| `./daemon/start.sh` | Run daemon |
| `cd dashboard && ./start.sh` | Run Control Room |
| `cd evolution && npx tsc --noEmit` | Type check evolution |
| `cd evolution && npx vitest run` | Evolution tests |
| `cvg plan status convergio` | Plan status |
| `cvg project create\|list\|show <id>` | Project ops |
| `cvg audit --project <id>` | Project audit |
| `scripts/mesh/mesh-heartbeat.sh` | Mesh health |
| `bash scripts/platform/convergio-openclaw-skills.sh` | Generate OpenClaw skills |

## Architecture

| Layer | Path | Lang | Modules |
|---|---|---|---|
| Daemon | `daemon/` | Rust | mesh(40) server(32) ipc(15) db(7) hooks(3) tui(3) |
| Dashboard | `dashboard/` | JS (Maranello DS) | app, KPI, mesh, chat, brain, IPC |
| Evolution | `evolution/` | TypeScript | core/types, adapters |
| Scripts | `scripts/` | Bash | mesh(12), platform(5) |
| Data | `data/dashboard.db` | SQLite WAL | plans, tasks, waves, KB, heartbeats |
| Integrations | `integrations/` | TS | OpenClaw bridge |
| Workspace | `daemon/src/workspace/` | Rust | core, events, git, waves, merge, quality, validation, release, deliverables |

## Key Paths

`data/dashboard.db` (env: `DASHBOARD_DB`) | `~/.claude/data/dashboard.db` (symlink) | `~/.claude/scripts/*.sh` → `claude-config/scripts/` | `~/.claude/config/peers.conf` (mesh) | `config/openclaw.yaml` | `integrations/openclaw-bridge/`

## Conventions

Max 250 lines/file | English only | Rust: fmt+clippy | JS: vanilla+Maranello DS | TS: strict, no `any` | Comments: WHY not WHAT, <5% | Evolution: standalone core, thin adapters | Mesh: Tailscale+HMAC-SHA256

## AI Agents

@.github/agents/Convergio.agent.md
@.github/agents/ConvergioLLM.agent.md
