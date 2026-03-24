# Lean Coordinator Protocol

Compaction mid-wave = state loss. Coordinator: dispatch + DB + checkpoint ONLY.

## Budget (NON-NEGOTIABLE)

ALLOWED: launch executor (~200 tok), read agent summary (~500), update DB (~300)
FORBIDDEN: read project files (→ Thor), read `/private/tmp/` transcripts (→ use summary), grep codebase (→ executor)

## Rules

1. NEVER read project files during execution. Executor reads+writes. Thor validates. You dispatch.
2. After task: (a) `cvg checkpoint save` (b) update DB (c) next task or Thor. Nothing more.
3. Batch DB: `cvg checkpoint save` (one call), not multiple queries.
4. Parallel: ALL independent tasks in ONE message.
5. Max 4 tasks/wave. Planner splits larger waves.

## Checkpoint

After every task | Before >2 parallel tasks | PreCompact hook automatic

## Post-Compaction

`cvg checkpoint restore` → `cvg plan execution-tree` → resume. Trust executor+Thor results.
