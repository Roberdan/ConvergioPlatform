# Hard Enforcement (HOOK-ENFORCED)

Enforcement status as of 01 Aprile 2026.

## Git Hooks (work for BOTH Claude Code and Copilot CLI)

| # | Rule | Hook | File | Blocks |
|---|---|---|---|---|
| G1 | No commits on main | `MainGuard` | `.git/hooks/pre-commit` | commit on main in main checkout |
| G2 | Max 250 lines/file | `FileSizeGuard` | `.git/hooks/pre-commit` | commit with .rs/.ts/.js/.sh >250 lines |
| G3 | No secrets in code | `SecretScan` | `.git/hooks/pre-commit` | commit with API keys, tokens, passwords |
| G4 | No sqlite3 direct | `SqliteBlock` | `.git/hooks/pre-commit` | commit with `sqlite3` in .sh/.py |
| G5 | Conventional commits | `CommitLint` | `.git/hooks/commit-msg` | non-conventional commit messages |

## Claude Code Hooks (Claude only — Copilot does NOT see these)

| # | Rule | Hook | Status | Behavior |
|---|---|---|---|---|
| C1 | No secrets in code | `SecretScan` | BLOCK | pre-tool-guard on Bash |
| C2 | No `sqlite3` direct | `SqliteBlock` | BLOCK | pre-tool-guard on Bash |
| C3 | Agent identity required | `AgentIdentity` | WARNING | SubagentStart warns if unset |
| C4 | `cargo check` on .rs edits | `RustCheck` | BLOCK | PostToolUse/Edit |
| C5 | Evidence before done | `EvidenceGate` | WARNING | SubagentStop |
| C6 | Max 250 lines/file | `FileSizeGuard` | BLOCK | PostToolUse/Write+Edit |
| C7 | New .rs = wire mod.rs | `RustModWiring` | BLOCK | PostToolUse |
| C8 | Fail-loud | `FailLoud` | WARNING | PreToolUse/Edit |
| C9 | No writes on main | `MainGuard` | BLOCK | PreToolUse/Edit+Write |
| C10 | Main dirty check | `MainDirtyCheck` | WARNING | SubagentStart |

## Daemon-Side (work for ALL clients)

| # | Rule | Hook | Behavior |
|---|---|---|---|
| D1 | Orphan task reset | `TaskReaper` | reaper resets in_progress tasks of dead agents |
| D2 | Daemon CWD guard | `DaemonCwdGuard` | start.sh blocks boot from worktree |
| D3 | Auth auto-provision | `AuthGuard` | start.sh generates token if missing |
| D4 | Evidence gate isolation | `EvidenceIsolation` | cargo test uses separate CARGO_TARGET_DIR |
| D5 | Main dirty reaper | `MainDirtyReaper` | daemon notifies if main has >5 dirty files |

**Coverage matrix**: G1-G5 protect against Copilot incidents. C1-C10 add real-time Claude protection. D1-D5 are server-side safety nets.

**Why WARNING vs BLOCK**: EvidenceGate, AgentIdentity, FailLoud are WARNING to avoid
breaking flows. Escalate to BLOCK after a confirmed incident per gate.

## Workflow (HOOK-ENFORCED)

`/solve` → `/planner` (Opus) → review (Sonnet) → DB → `/execute` (Codex) → thor (Opus) → merge → done

Skipping steps = BLOCKED. After every task: `cvg checkpoint save` → update DB.

## Status Flow

`pending → in_progress → submitted (executor) → done (ONLY Thor)`

Executors CANNOT set status=done. Only `cvg plan validate` promotes submitted → done.

## Cascading Fix Threshold

3 consecutive fixes for same issue where each introduces new problem → STOP. Explain root
cause, propose rebuild. Band-aid chains = REJECTED.

## Merge Quality Gate

| Gate | Command | Block if |
|---|---|---|
| Clean tree | `git status --short` | Modified/untracked |
| No contamination | `git diff --name-only` vs task files | Outside scope |
| Type-check | `npx tsc --noEmit` / `cargo check` | Exit != 0 |
| Tests | `pytest` / `vitest` / `cargo test` | Exit != 0 |
| Lint | `ruff check` / `eslint` / `clippy` | Errors |
| Version | VERSION.md = Cargo.toml/package.json | Mismatch |
| CHANGELOG | Latest entry = current version | Stale |

## Copilot Delegation

Copilot delegation is handled natively by the Convergio daemon:

| Need | How |
|---|---|
| Execute plan | `/execute {plan_id}` — auto-delegates mechanical tasks to Copilot |
| Launch Copilot session | `cvg copilot <name>` — registers with daemon, spawns `gh copilot` |
| Manual delegation | Claude agents read TASK.md header and call `gh copilot --model claude-opus-4-6` |

The daemon spawner launches Claude, which delegates sub-tasks to Copilot per TASK.md instructions.
Process scanner auto-discovers running Copilot PIDs and registers them in ipc_agents.

NEVER delegate via GitHub Issues or external scripts. The daemon handles all orchestration.

## UI Integration Rules

- UI-touching agents MUST read `claude-config/reference/operational/ds-integration-playbook.md`
  before recommending or implementing Convergio Design System integration.
- For imperative DS widgets in React, use `useRef + useEffect + dynamic import`
  with explicit cleanup on unmount.
- Do NOT wrap DS ref containers in `AnimatePresence`; use early-return loading
  states instead.
- Use exact DS API signatures from the generated `.d.ts` types, not inferred
  prop names from docs, memory, or examples.

## Compaction Preservation

NEVER remove: quality gates, Thor validation, pre-commit hooks, verify steps, security rules,
worktree discipline, routing, docs requirements, learning markers.
