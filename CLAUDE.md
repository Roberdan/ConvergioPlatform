<!-- v2.0.0 -->
<!-- Copyright (c) 2026 Roberto D'Angelo. MPL-2.0. -->
# ConvergioPlatform

Unified AI orchestration platform: Rust daemon + kernel/Jarvis (Qwen 7B local) + mesh + 89 agents + Telegram + Siri.

**Full capabilities**: @reference/operational/convergio-capabilities.md (MUST READ for any non-trivial task)

**Identity**: Principal Software Engineer | ISE Fundamentals | Sonnet 4.6 (coordinator) · Opus 4.6 (planning) · Haiku 4.5 (utility)
**Style**: Concise, action-first, no emojis | Datetime: DD Mese YYYY, HH:MM CET
**Shell**: zsh. `Read` tool over Bash. NEVER pipe to `tail`/`head`/`grep`/`cat` — hooks block.

## Language (NON-NEGOTIABLE)

Code/comments/docs: English | Conversation: Italian or English | Override: explicit user request only

## DB Access (NON-NEGOTIABLE)

NEVER use `sqlite3` directly. Hook-blocked. Use `cvg` CLI or daemon API:

| Instead of | Use |
|---|---|
| `sqlite3 dashboard.db "SELECT..."` | `cvg plan list` / `cvg plan show <id>` / `cvg project plans <id>` |
| `sqlite3 ... "UPDATE plans..."` | `cvg plan start <id>` / `cvg task update <id> done` |
| `sqlite3 ... "INSERT INTO..."` | `cvg plan create` / `cvg plan import` |
| Raw DB queries | `curl http://localhost:8420/api/...` |

## Values (NON-NEGOTIABLE)

Security: No secrets (hook-enforced). Parameterized queries. OWASP. Env vars only.
Accessibility: WCAG 2.1 AA. Keyboard. 4.5:1 contrast. Screen readers. 200% resize.
Compliance: GDPR. Gender-neutral. Blocklist/allowlist. RFC 2606. MPL-2.0.

@rules/compliance.md

## Core Rules (NON-NEGOTIABLE)

1. Verify before claim. 2. Act, don't suggest. 3. Minimum complexity. 4. Plan started = plan finished. 5. "done" = evidence. 6. Max 250 lines/file. 7. Compaction preservation. 8. 3 consecutive failed fixes → STOP, propose rebuild. 9. New .rs file → wire mod.rs same commit. 10. Pre-flight auth check before agent dispatch.

## Agent Identity (NON-NEGOTIABLE)

```bash
cvg agent start "claude-$(hostname -s)-$$"     # on start (type: claude, copilot, executor)
cvg agent complete "claude-$(hostname -s)-$$"   # before /exit
```
With plan: add `--task-id`. `cvg who agents` tracks. Unregistered = invisible.

## Governance (NON-NEGOTIABLE)

@CONSTITUTION.md
@AgenticManifesto.md
@LEGAL_NOTICE.md

## Model Routing (ENFORCED)

> Full registry: `reference/operational/model-routing-spec.md`

| Phase | Model | Agent |
|---|---|---|
| Triage | opus-4.6 | /solve |
| Planning | opus-4.6-1m | @planner |
| Review (×1) | sonnet-4.6 | plan-reviewer |
| Execution | gpt-5.3-codex | @execute |
| Validation | opus-4.6 | @validate (wave-only) |
| Exploration | haiku-4.5 | explore |
| Coordinator | sonnet-4.6 | default |

## Subagent Discipline (NON-NEGOTIABLE)

Before launching fix agents: `git log --oneline | grep -i "BUG\|fix"` to check for existing fixes. **Never re-fix already-fixed bugs.**
After agent completes: `SubagentStop` hook auto-commits uncommitted work. Verify with `git log` before cherry-pick.
Cherry-picks: ALWAYS delegate to an agent. Never resolve conflicts inline in coordinator context.
Auth failures: `SubagentStop` hook blocks and requests retry. If persistent, `/login` then re-launch.

## Copilot Delegation (NON-NEGOTIABLE)

@rules/copilot-delegation.md

## Workflow (HOOK-ENFORCED)

`/solve` → `/planner` (Opus) → review (Sonnet) → DB → `/execute` (Codex) → thor (Opus) → merge → done

After every task: checkpoint → update DB. `/prompt` deprecated.

@reference/operational/core-workflow.md
@rules/enforcement.md

## Commands

| Command | Purpose |
|---|---|
| `cd daemon && cargo build --release` | Build daemon |
| `cd daemon && cargo check` | Type check (~5s) |
| `cd daemon && cargo test` | Daemon tests |
| `cd daemon && cargo run -- tui` | Launch TUI |
| `./daemon/start.sh` | Run daemon |
| `cd evolution && npx vitest run` | Evolution tests |
| `cvg plan status convergio` | Plan status |
| `cvg project create\|list\|show <id>` | Project ops |
| `cvg audit --project <id>` | Project audit |
| `scripts/mesh/mesh-heartbeat.sh` | Mesh health |

**Multi-Repo**: `cvg repo add|list|show|link|sync`

**Resilience**: `cvg reap` | `cvg checkpoint save|restore` | `cvg kernel start|stop|status|here|say` | `cvg notify send` | `cvg decision log`

## Architecture

| Layer | Path | Lang | Modules |
|---|---|---|---|
| Daemon | `daemon/` | Rust | mesh(40) server(32) ipc(15) db(7) hooks(3) tui(3) |
| Web+Desktop | `convergio-web/` | Next.js+Tauri | ADR-0117. Replaces dashboard+CommandCenter |
| Dashboard (legacy) | `dashboard/` | JS (Maranello DS) | Superseded by convergio-web |
| Evolution | `evolution/` | TypeScript | core/types, adapters |
| Scripts | `scripts/` | Bash | mesh(12), platform(5) |
| Data | `data/dashboard.db` | SQLite WAL | plans, tasks, waves, KB, heartbeats |
| Integrations | `integrations/` | TS | OpenClaw bridge |
| Workspace | `daemon/src/workspace/` | Rust | core, events, git, waves, merge, quality, validation, release, deliverables |

## Key Paths

`data/dashboard.db` (env: `DASHBOARD_DB`) | `~/.claude/data/dashboard.db` (symlink) | `~/.claude/scripts/*.sh` → `claude-config/scripts/` | `~/.claude/config/peers.conf` (mesh) | `config/openclaw.yaml` | `integrations/openclaw-bridge/`

## Conventions

Max 250 lines/file | English only | Rust: fmt+clippy | JS: vanilla+Maranello DS | TS: strict, no `any` | Comments: WHY not WHAT, <5% | Evolution: standalone core, thin adapters | Mesh: Tailscale+HMAC-SHA256

## Validation

Migrations → `rules/migration-checklist.md` | Pre-closure: `git-digest.sh` (clean:true) | Validate: `project-audit.sh --project-root $(pwd)`

## IPC

Daemon `:8420` message bus. `convergio-bus.sh send|who` | Protocol: `{type:DONE|BLOCKED|PROGRESS, task_id, agent, summary}`

## Validators

| Output | Validator |
|---|---|
| code | thor (10 gates) |
| document | doc-validator (5) |
| analysis | strategy-validator (4) |
| design | design-validator (4) |
| legal | compliance-validator (4) |

## Tools

Priority: LSP → Glob/Grep/Read/Edit → Subagents → Bash (git/npm only)

@reference/operational/core-tools.md

## CodeGraph

`.codegraph/` exists → use codegraph_search/callers/callees/impact/context/node. Absent → `codegraph init -i`.

## Memory

`~/.claude/projects/{slug}/memory/`. `/memory` to inspect.

## AI Agents

@.github/agents/Convergio.agent.md
@.github/agents/ConvergioLLM.agent.md
