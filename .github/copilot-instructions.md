# ConvergioPlatform -- Copilot Instructions

Unified control plane: Rust daemon (107 modules) + dashboard + evolution engine.

## Architecture

| Layer | Path | Lang | Purpose |
|---|---|---|---|
| Daemon | `daemon/` | Rust | Mesh P2P, HTTP/WS/SSE API, TUI, IPC, hooks, SQLite DB |
| Dashboard | `dashboard/` | Python+JS | Control Room web UI (Maranello DS) |
| Evolution | `evolution/` | TypeScript | Self-improving optimization engine (core + adapters) |
| Scripts | `scripts/` | Bash | Mesh ops (12), platform tooling (5) |
| Data | `data/dashboard.db` | SQLite WAL | Plans, tasks, waves, KB, heartbeats |

Stack: axum, rusqlite WAL, tokio, ssh2, ratatui, serde, hmac+sha2+aes-gcm, reqwest, sysinfo, tracing.

## Agent Naming

Every agent has a unique name. Cross-platform agents share the same plan-db and worktree discipline.

| Name | Type | Default Model | Primary Skill |
|---|---|---|---|
| planner | claude | claude-opus-4.6 | Plan creation (spec.json + DB registration) |
| executor | copilot | gpt-5.3-codex | TDD task execution (one task at a time) |
| reviewer (thor) | claude | claude-opus-4.6 | Quality validation (9 gates, per-wave) |
| explorer | claude | claude-haiku-4.5 | Codebase exploration and analysis |
| prompt | claude | claude-opus-4.6 | Requirements extraction (F-xx features) |
| researcher | claude | claude-haiku-4.5 | Web research and context gathering |
| copilot-worker | copilot | gpt-5.3-codex | General coding tasks |

## Workflow Routing

| Trigger | Command | NOT |
|---|---|---|
| Multi-step work (3+ tasks) | `@planner` | Manual planning, EnterPlanMode |
| Execute plan tasks | `@execute {id}` | Direct file editing |
| Validate wave | `@validate` | Self-declaring done |
| Single isolated fix | Direct edit | Creating unnecessary plan |

EnterPlanMode bypasses DB registration and is a VIOLATION. Always use `@planner` for plans.

Workflow sequence: `@prompt` (requirements) -> `@planner` (spec + DB) -> review -> `@execute {id}` (TDD) -> `@validate` (Thor per-wave) -> merge -> done.

## IPC Registration

Agents MUST register on start:

```bash
scripts/platform/agent-bridge.sh --register --name <name> --type <type>
```

API endpoint: `POST http://localhost:8420/api/ipc/agents/register`

Registration payload:

```json
{
  "name": "copilot-worker",
  "type": "copilot",
  "model": "gpt-5.3-codex",
  "capabilities": ["code", "test", "review"]
}
```

Without registration, agents cannot participate in coordinated workflows or appear in platform dashboards.

## Code Standards

| Lang | Standard |
|---|---|
| Rust | `cargo fmt` + `cargo clippy`, strict warnings |
| TS/JS | ESLint+Prettier, semicolons, single quotes, 100 char lines, `const` > `let`, `interface` > `type` |
| Python | Black 88, Google docstrings, type hints, pytest+fixtures |
| Bash | `set -euo pipefail`, quote all vars, `local`, `trap cleanup EXIT` |
| CSS | Modules/BEM, `rem`/`px` borders, mobile-first, max 3 nesting levels |
| Config | 2-space indent |

| Rule | Detail |
|---|---|
| Max file size | 250 lines -- split if exceeds |
| Comments | WHY not WHAT, less than 5% of lines |
| Commits | Conventional format, 1 subject line |
| PRs | Summary + Test plan sections |
| REST API | Plural nouns, kebab-case, max 3 levels, `/api/v1/` prefix |
| Error format | `{error:{code,message,details?,requestId,timestamp}}` |
| Coverage | 80% business logic, 100% critical paths |
| Fail-loud | Empty unexpected data -> `console.warn` + visible UI; silent `return null` = BUG |
| Zero debt | Touch ANY line -> own ALL issues; no TODO/FIXME/stubs/deferred |
| Tests | Colocated `.test.ts`, AAA pattern, real data shapes, no `Test Studio` placeholders |
| Security | Parameterized SQL, CSP, TLS 1.2+, RBAC server-side, no secrets in code |
| A11y | WCAG 2.1 AA: 4.5:1 contrast, keyboard nav, screen readers, 200% resize |

## Hook Enforcement

These hooks apply to Copilot CLI sessions. All are portable across Claude Code and Copilot.

| Hook | Event | Trigger | Action |
|---|---|---|---|
| guard-plan-mode | PreToolUse | EnterPlanMode | Block -- must use @planner |
| enforce-plan-db-safe | PreToolUse | `plan-db.sh done` | Block -- must use plan-db-safe.sh |
| enforce-plan-edit | PreToolUse | Edit plan files | Block unless task-executor |
| worktree-guard | PreToolUse | git on main | Warn/Block -- use worktrees |
| session-file-lock | PreToolUse | Edit/Write | Lock file to prevent conflicts |
| prefer-ci-summary | PreToolUse | Raw npm/gh commands | Block -- use digest scripts |
| warn-bash-antipatterns | PreToolUse | Unsafe Bash patterns | Warn/Block |
| enforce-line-limit | PostToolUse | File > 250 lines | Warn -- split the file |

Non-portable hooks (Claude Code only, no Copilot event): secret-scanner, env-vault-guard, auto-format, inject-agent-context, preserve-context, model-registry-refresh, version-check.

## Commands

| Command | Purpose |
|---|---|
| `cd daemon && cargo build --release` | Build daemon |
| `cd daemon && cargo check` | Type check (~5s) |
| `cd daemon && cargo test` | Run daemon tests |
| `./daemon/start.sh` | Run daemon (auto release/debug/build) |
| `cd dashboard && ./start.sh` | Run Control Room (reads DASHBOARD_DB) |
| `cd evolution && npx tsc --noEmit` | Type check evolution engine |
| `cd evolution && npx vitest run` | Run evolution tests |
| `plan-db.sh status convergio` | Check plan status |
| `plan-db.sh execution-tree {plan_id}` | View plan execution tree |
| `scripts/mesh/mesh-heartbeat.sh` | Check mesh node health |
| `scripts/mesh/mesh-provision-node.sh <peer>` | Provision new mesh node |
| `scripts/mesh/mesh-sync-all.sh` | Sync all mesh nodes |
| `scripts/platform/agent-bridge.sh` | Agent IPC registration bridge |

## Key Paths

| Path | What |
|---|---|
| `daemon/` | Rust daemon: mesh(40), server(32), ipc(15), db(7), hooks(3), tui(3) |
| `daemon/Cargo.toml` | Rust deps (axum, rusqlite, tokio, ssh2, ratatui) |
| `dashboard/` | Python api_server + vanilla JS + Maranello DS |
| `dashboard/api_server.py` | HTTP server, daemon proxy |
| `evolution/` | TypeScript core/types + adapters (claude, maranello, dashboard) |
| `scripts/mesh/` | 12 mesh operation scripts |
| `scripts/platform/` | Platform tooling (bootstrap, agent-bridge) |
| `data/dashboard.db` | Platform DB -- env: `DASHBOARD_DB` |
| `~/.claude/data/dashboard.db` | Symlink to platform DB |
| `~/.claude/config/peers.conf` | Mesh config (per-machine) |
| `.github/agents/` | Agent definitions (Convergio, ConvergioLLM) |
| `config/llm/` | LiteLLM proxy + model catalog config |
