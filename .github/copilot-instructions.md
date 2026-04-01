# ConvergioPlatform — Copilot Instructions

Unified AI orchestration: Rust daemon + kernel/Jarvis (Qwen 7B) + mesh + agents + Telegram + Siri.

## Identity (NON-NEGOTIABLE)

On session start:
```bash
cvg agent start "copilot-$(hostname -s)-$$"
```
On exit: `cvg agent complete "copilot-$(hostname -s)-$$"`
Unregistered agents are invisible in `cvg who agents`.

## Language (NON-NEGOTIABLE)

Code/comments/docs: English | Conversation: Italian or English

## DB Access (NON-NEGOTIABLE)

NEVER `sqlite3` directly (git hook blocks). Use `cvg` CLI or `curl http://localhost:8420/api/...`

## Worktree Discipline (NON-NEGOTIABLE)

- NEVER create branches. Always `git worktree add --detach <path> HEAD`
- NEVER commit on main (git pre-commit hook blocks it)
- All work in worktrees, merge via PR only
- NEVER edit files directly in the main checkout

## File Limits (NON-NEGOTIABLE)

- Max **250 lines per file** (git pre-commit hook blocks oversized files)
- Split into submodules before committing

## Commits (NON-NEGOTIABLE)

- Conventional format enforced by git commit-msg hook
- Format: `type(scope): message`
- Types: feat, fix, docs, chore, refactor, test, perf, ci, style, build

## Security (NON-NEGOTIABLE)

- No secrets in code (git pre-commit scans for API keys, tokens, passwords)
- No `sqlite3` in shell scripts (pre-commit blocks)
- All auth via `CONVERGIO_AUTH_TOKEN` env var

## Evidence Before Done

- Status flow: `pending -> in_progress -> submitted -> done`
- Before `submitted`: POST `/api/plan-db/task/evidence` with test_pass
- Before `done`: POST `/api/validation/record` with verdict=pass
- Executors CANNOT set status=done (only Thor/validator)

## Git Hooks (enforced for BOTH Claude and Copilot)

| Hook | What | Blocks |
|---|---|---|
| pre-commit: MainGuard | No commits on main in main checkout | exit 1 |
| pre-commit: FileSizeGuard | No file >250 lines (.rs .ts .js .sh) | exit 1 |
| pre-commit: SecretScan | No API keys, tokens, passwords | exit 1 |
| pre-commit: SqliteBlock | No `sqlite3` in .sh/.py files | exit 1 |
| commit-msg: CommitLint | Conventional commit format required | exit 1 |

## Architecture

| Layer | Path | Lang |
|---|---|---|
| Daemon | `daemon/` | Rust |
| Web+Desktop | `convergio-web/` | Next.js+Tauri |
| Evolution | `evolution/` | TypeScript |
| Scripts | `scripts/` | Bash |
| Data | `data/dashboard.db` | SQLite WAL |

## Model Routing

| Phase | Model | Agent |
|---|---|---|
| Planning | claude-opus-4.6 | @planner |
| Review | claude-sonnet-4.6 | plan-reviewer |
| Execution | gpt-5.3-codex | @execute |
| Validation | claude-opus-4.6 | @validate |

## Commands

| Command | Purpose |
|---|---|
| `cd daemon && cargo build --release --features kernel` | Build |
| `cd daemon && cargo check --features kernel` | Type check |
| `cd daemon && cargo test --features kernel --lib -- --test-threads=1` | Tests |
| `./daemon/start.sh serve` | Run daemon |
| `cvg plan show <id>` | Plan details |
| `cvg plan list` | List plans |
| `cvg task update <id> <status>` | Update task |
| `cvg task create <plan_id> <wave_id> <task_id> <title>` | Create task |
| `cvg org list` | List orgs |
| `cvg org show <id>` | Org details |
| `scripts/mesh/mesh-heartbeat.sh` | Mesh health |

## Key Paths

`data/dashboard.db` (env: `DASHBOARD_DB`) | `~/.claude/config/peers.conf` (mesh) | `~/.convergio/config.toml` (runtime config)

## Conventions

Max 250 lines/file | English only | Comments: WHY not WHAT, <5% | Mesh: Tailscale+HMAC | Deploy: rsync binary, NEVER recompile on target

## Quality Principles (NON-NEGOTIABLE)

- Zero tech debt: touch file = own ALL issues
- Root cause only: no band-aids, escalate after 2 attempts
- Never hide problems: stop, surface, discuss
- 3 cascading fixes for same issue = STOP, propose rebuild

## Verification (NON-NEGOTIABLE)

| Claim | Evidence |
|---|---|
| "It builds" | Build output |
| "Tests pass" | Test output |
| "It works" | Execution demo |

TDD mandatory. Plan done = ALL PRs merged, worktrees clean, branches deleted.

## Scope Management

One session = one plan. Max 5 checkpoints. Scope drift after 3 = split.

## Mesh (active nodes)

| Node | Role | IP |
|---|---|---|
| M5 Max | coordinator | 100.89.245.79 |
| M1 Pro | worker/kernel | 100.106.173.118 |
| Omarchy | worker (Linux) | 100.127.138.62 |
