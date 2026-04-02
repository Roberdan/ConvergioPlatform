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

## Worktree Discipline (NON-NEGOTIABLE)
- NEVER create branches. Always `git worktree add --detach <path> HEAD`
- Branch creation is blocked by hook. Only worktree detached HEAD is allowed.
- Branches are created ONLY by the workspace release pipeline at merge time.
- Deleting branches (`git branch -D`) is allowed for cleanup.

## Scope Management

- One session = one plan or one clearly defined scope
- After 3 checkpoints, evaluate if scope has drifted from original intent
- If scope changed significantly, suggest opening a new session
- Max 5 checkpoints per session — beyond this, split is mandatory
- Mega-sessions (80+ turns) degrade context quality and increase fragility

## Rules

@rules/hard-enforcement.md

> Best practices: `@rules/best-practices.md` (on-demand, not hook-enforced)

## Governance

@CONSTITUTION.md

> Manifesto: `AgenticManifesto.md` | Legal: `LEGAL_NOTICE.md` (on-demand, not operational)

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
| `cd daemon && cargo build --release --features kernel` | Build |
| `cd daemon && cargo check --features kernel,crsqlite` | Type check (all features) |
| `cd daemon && cargo test --features kernel --lib -- --test-threads=1` | Tests |
| `./daemon/start.sh` | Run daemon |
| `cd evolution && npx vitest run` | Evolution tests |
| `cvg plan show <id>` | Plan details |
| `cvg org list` | List orgs with status/CEO |
| `cvg org show <id>` | Show full org detail (members/services/telemetry) |
| `cvg bus org --human` | Render terminal org hierarchy tree |
| `cvg bus watch <agent>` | Watch direct messages for an agent via SSE |
| `cvg project create\|list\|show <id>` | Project ops |
| `cvg plan close <id>` | Close plan with mesh broadcast |
| `cvg cleanup` | Remove stale worktrees/branches |
| `cvg delegation start <id> --peer <peer>` | Delegate plan |
| `scripts/platform/record-evidence.sh <task_id> test_pass "<cmd>" 0` | Record test evidence |

## Task Lifecycle (NON-NEGOTIABLE)

Status flow: `pending → in_progress → submitted → done`

Before `submitted`: POST `/api/plan-db/task/evidence` with `evidence_type=test_pass` (TestGate)
Before `done`: POST `/api/validation/record` with `verdict=pass` (ValidatorGate)

Skipping gates = 400 error. No exceptions.

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
NEVER pipe Bash to `tail`/`head`/`grep`/`cat` — hook blocks. Use Read/Grep/Glob.
CI: `ci-summary.sh`/`ci-digest.sh` over raw `npm`/`gh` commands. PR ops: `cvg workspace release`.
Plans: `Skill(skill="planner")` not EnterPlanMode. Execute: `Skill(skill="execute")`.
CodeGraph: `.codegraph/` exists → use codegraph_search. Absent → `codegraph init -i`.

> Full tool/routing reference: `@reference/operational/core-tools.md` (on-demand)
> Agent specs: `.github/agents/` (auto-registered, fetch on-demand via Agent tool)

## On-Demand Context

Skills, agents, and references are lazy-loaded. The daemon dispatches on demand.
Only hard-enforcement rules and Constitution NN articles are always-on.
Everything else: fetch when the task requires it, not upfront.
