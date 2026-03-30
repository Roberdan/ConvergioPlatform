# Hard Enforcement (HOOK-ENFORCED)

Enforcement status reflects what hooks actually do as of 30 Marzo 2026.

| # | Rule | Hook | Status | Behavior |
|---|---|---|---|---|
| 1 | No secrets in code | `SecretScan` | BLOCK | pre-tool-guard + secret-scanner.sh on git commit |
| 2 | No `sqlite3` direct access | `SqliteBlock` | BLOCK | pre-tool-guard: blocks `sqlite3 ` in Bash |
| 3 | Agent identity required | `AgentIdentity` | WARNING | SubagentStart + SubagentStop warn if CONVERGIO_AGENT_NAME unset |
| 4 | `cargo check` on .rs edits | `RustCheck` | BLOCK | PostToolUse/Edit: post-edit-rust-check.sh, exits non-zero |
| 5 | Evidence before done | `EvidenceGate` | WARNING | SubagentStop: warns if "done/completed" claimed without test/curl evidence |
| 6 | Max 250 lines/file | `FileSizeGuard` | BLOCK | PostToolUse/Write+Edit: exits 2 if wc -l > 250 |
| 7 | New .rs file = wire mod.rs | `RustModWiring` | BLOCK | PostToolUse: check-rust-wiring.sh on daemon/src/*.rs |
| 8 | Fail-loud (no silent fallback) | `FailLoud` | WARNING | PreToolUse/Edit: warns on unwrap_or_default() and let _ = in .rs files |
| 9 | Conventional commits | `CommitLint` | BLOCK | PreToolUse/Bash: blocks git commit -m with non-conventional message |
| 10 | Test before done | `TestGate` | WARNING | PreToolUse/Bash: warns on Rust commit if /tmp/.convergio-test-ran absent |

**Why WARNING vs BLOCK**: EvidenceGate, AgentIdentity, FailLoud, TestGate are WARNING to avoid
breaking existing flows. They surface problems without halting valid work. Escalate to BLOCK
after a confirmed incident per gate.

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

NEVER remove: quality gates, Thor validation, pre-commit hooks, verify steps, security rules,
worktree discipline, routing, docs requirements, learning markers.
