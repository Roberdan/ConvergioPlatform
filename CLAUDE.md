# ConvergioPlatform

Unified AI orchestration: Rust daemon + kernel/Jarvis (Qwen 7B) + mesh + agents + Telegram + Siri.

**Capabilities**: @reference/operational/convergio-capabilities.md (MUST READ for non-trivial tasks)

**Identity**: Principal Software Engineer | Sonnet 4.6 (coordinator) · Opus 4.6 (planning) · Haiku 4.5 (utility)
**Style**: Concise, action-first, no emojis | Datetime: DD Mese YYYY, HH:MM CET
**Shell**: zsh. `Read` tool over Bash. NEVER pipe to `tail`/`head`/`grep`/`cat` — hooks block.

## Session Identity (NON-NEGOTIABLE)

On session start, register with the daemon using a meaningful name:
```bash
curl -sf -X POST http://localhost:8420/api/ipc/agents/register \
  -H 'Content-Type: application/json' \
  -d '{"agent_id":"copilot-<hostname>-<pid>","host":"<hostname>","agent_type":"copilot"}'
```
Use `cvg agent start "copilot-$(hostname -s)-$$"` for plan work (include `--task-id`).
On exit: `cvg agent complete "copilot-$(hostname -s)-$$"` or POST to `/api/ipc/agents/deregister`.
Unregistered sessions are invisible in `cvg who agents` and platform dashboards.

Check registered sessions: `GET http://localhost:8420/api/ipc/agents/list`

## Language (NON-NEGOTIABLE)

Code/comments/docs: English | Conversation: Italian or English | Override: explicit user request only

## DB Access (NON-NEGOTIABLE)

NEVER `sqlite3` directly (hook-blocked). Use `cvg` CLI or `curl http://localhost:8420/api/...`

## Rules

@rules/hard-enforcement.md
@rules/best-practices.md

## Governance

@CONSTITUTION.md
@AgenticManifesto.md
@LEGAL_NOTICE.md

## Model Routing

| Phase | Model | Agent |
|---|---|---|
| Triage | opus-4.6 | /solve |
| Planning | opus-4.6-1m | @planner |
| Review | sonnet-4.6 | plan-reviewer |
| Execution | gpt-5.3-codex | @execute |
| Validation | opus-4.6 | @validate |
| Coordinator | sonnet-4.6 | default |

> Full registry: `reference/operational/model-routing-spec.md`

## Subagent Discipline

Before fix agents: `git log --oneline | grep -i "BUG\|fix"` — never re-fix fixed bugs.
After agent: `SubagentStop` hook auto-commits. Verify `git log` before cherry-pick.
Cherry-picks: delegate to agent. Auth failures: `/login` then re-launch.

## Commands

| Command | Purpose |
|---|---|
| `cd daemon && cargo build --release` | Build |
| `cd daemon && cargo check` | Type check |
| `cd daemon && cargo test` | Tests |
| `./daemon/start.sh` | Run daemon |
| `cd evolution && npx vitest run` | Evolution tests |
| `cvg plan show <id>` | Plan details |
| `cvg project create\|list\|show <id>` | Project ops |
| `scripts/mesh/mesh-heartbeat.sh` | Mesh health |

**Resilience**: `cvg reap` | `cvg checkpoint save|restore` | `cvg kernel start|stop|status`

## Architecture

| Layer | Path | Lang |
|---|---|---|
| Daemon | `daemon/` | Rust |
| Web+Desktop | `convergio-web/` | Next.js+Tauri |
| Evolution | `evolution/` | TypeScript |
| Scripts | `scripts/` | Bash |
| Data | `data/dashboard.db` | SQLite WAL |
| Integrations | `integrations/` | TS |

## Key Paths

`data/dashboard.db` (env: `DASHBOARD_DB`) | `~/.claude/scripts/*.sh` → `claude-config/scripts/` | `~/.claude/config/peers.conf` (mesh)

## Conventions

Max 250 lines/file | English only | Comments: WHY not WHAT, <5% | Mesh: Tailscale+HMAC-SHA256

## Validators

| Output | Validator |
|---|---|
| code | thor (10 gates) |
| document | doc-validator |
| analysis | plan-reviewer |
| design | design-validator |
| legal | compliance-validator |

## Tools

Priority: LSP → Glob/Grep/Read/Edit → Subagents → Bash (git/npm only)

@reference/operational/core-tools.md

## CodeGraph

`.codegraph/` exists → use codegraph_search/callers/callees/impact. Absent → `codegraph init -i`.

## Memory

`~/.claude/projects/{slug}/memory/`. `/memory` to inspect.

## AI Agents

@.github/agents/Convergio.agent.md
@.github/agents/ConvergioLLM.agent.md
