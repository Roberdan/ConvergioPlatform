# Hard Enforcement (HOOK-ENFORCED)

These rules are enforced by daemon hooks. Violations are blocked automatically.

| # | Rule | Hook | Block |
|---|---|---|---|
| 1 | No secrets in code | `SecretScan` | Detects API keys, tokens, passwords |
| 2 | No `sqlite3` direct access | `SqliteBlock` | Use `cvg` CLI or daemon API |
| 3 | Agent identity required | `AgentIdentity` | `cvg agent start/complete` on session boundaries |
| 4 | `cargo check` on .rs edits | `RustCheck` | Exit != 0 blocks commit |
| 5 | Evidence before done | `EvidenceGate` | Build output, test output, or execution demo |
| 6 | Max 250 lines/file | `FileSizeGuard` | `wc -l` > 250 on changed files |
| 7 | New .rs file = wire mod.rs | `RustModWiring` | Same commit, `cargo check` pass |
| 8 | Fail-loud (no silent fallback) | `FailLoud` | Empty data → `console.warn` + visible UI |
| 9 | Conventional commits | `CommitLint` | `type(scope): message` format |
| 10 | Test before done | `TestGate` | Task cannot reach `submitted` without passing tests |

## Workflow (HOOK-ENFORCED)

`/solve` → `/planner` (Opus) → review (Sonnet) → DB → `/execute` (Codex) → thor (Opus) → merge → done

Skipping steps = BLOCKED. After every task: `cvg checkpoint save` → update DB.

## Status Flow

`pending → in_progress → submitted (executor) → done (ONLY Thor)`

Executors CANNOT set status=done. Only `cvg plan validate` promotes submitted → done.

## Cascading Fix Threshold

3 consecutive fixes for same issue where each introduces new problem → STOP. Explain root cause, propose rebuild. Band-aid chains = REJECTED.

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

| Need | Command |
|---|---|
| Single task | `copilot-worker.sh <db_task_id> --model claude-opus-4.6` |
| Full plan | `copilot-plan-runner.sh <plan_id>` |
| Task prompt | `copilot-task-prompt.sh <db_task_id> [role]` |

NEVER delegate via GitHub Issues. Convergio scripts handle orchestration.

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

NEVER remove: quality gates, Thor validation, pre-commit hooks, verify steps, security rules, worktree discipline, routing, docs requirements, learning markers.
