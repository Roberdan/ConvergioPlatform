<!-- v1.1.0 -->
# ConvergioPlatform

Unified control plane: Rust daemon (107 modules) + dashboard + evolution engine.

## Commands

| Command | Purpose |
|---|---|
| `cd daemon && cargo build --release` | Build daemon |
| `cd daemon && cargo check` | Type check (~5s) |
| `cd daemon && cargo test` | Daemon tests |
| `cd daemon && cargo run -- tui` | Launch TUI (plan/task/agent ops) |
| `./daemon/start.sh` | Run daemon (auto release/debug/build) |
| `cd dashboard && ./start.sh` | Run Control Room (reads DASHBOARD_DB) |
| `cd evolution && npx tsc --noEmit` | Type check evolution |
| `cd evolution && npx vitest run` | Evolution tests |
| `plan-db.sh status convergio` | Plan status |
| `scripts/mesh/mesh-heartbeat.sh` | Check mesh nodes |

## Architecture

| Layer | Path | Lang | Modules |
|---|---|---|---|
| Daemon | `daemon/` | Rust | mesh(40) server(32) ipc(15) db(7) hooks(3) tui(3) |
| Dashboard | `dashboard/` | JS (Maranello DS) | app, KPI, mesh, chat, brain, IPC (served by daemon) |
| Evolution | `evolution/` | TypeScript | core/types, adapters (claude, maranello, dashboard) |
| Scripts | `scripts/` | Bash | mesh(12), platform(5) |
| Data | `data/dashboard.db` | SQLite WAL | plans, tasks, waves, KB, heartbeats |

## Key Paths

| Path | What |
|---|---|
| `data/dashboard.db` | Platform DB (env: `DASHBOARD_DB`) |
| `~/.claude/data/dashboard.db` | Symlink to above |
| `~/.claude/scripts/*.sh` | Symlinks to `claude-config/scripts/*.sh` (via bootstrap) |
| `~/.claude/scripts/lib/*.sh` | Symlinks to `claude-config/scripts/lib/*.sh` |
| `~/.claude/config/peers.conf` | Mesh config (per-machine) |
| `daemon/Cargo.toml` | Rust deps (axum, rusqlite, tokio, ssh2, ratatui) |

## Conventions

- Max 250 lines/file — split if exceeds
- English only
- Rust: `cargo fmt` + `cargo clippy`
- JS: vanilla, Maranello DS for UI
- TS: strict, no `any`, named exports
- Comments: WHY not WHAT, <5% density
- Evolution: standalone core, thin adapters, PR-only governance
- Mesh: Tailscale + HMAC-SHA256

## AI Agents

| Agent | Role |
|---|---|
| Convergio | Platform control plane expert |
| ConvergioLLM | Local LLM infrastructure manager |

@.github/agents/Convergio.agent.md
@.github/agents/ConvergioLLM.agent.md
