## Activation

Run when invoked as `/execute {{plan_id}}` or `/execute` (uses current plan). Accepts optional `--force-engine claude|copilot` override.

## Phases

### Phase 1: Initialize

1. Run `bash claude-config/scripts/plan-db.sh get-context {{plan_id}}` to retrieve full JSON with tasks, worktree, and constraints.
2. View tree: `bash claude-config/scripts/plan-db.sh show {{plan_id}}`
3. Auto-heal plan/worktree metadata if needed.
4. Run readiness checks — stop on critical warnings.
5. Run drift check (mandatory before first task).

### Phase 2: Per-Wave Loop (repeat for each wave)

**Status flow (NON-NEGOTIABLE):**
```
pending → in_progress → submitted (executor) → done (Thor only)
                             ↓ Thor rejects
                        in_progress (fix and resubmit)
```
Executors CANNOT set status=done. Only `validate-wave` (called at wave level) can batch-promote submitted → done.

**Steps per wave:**

1. Read `executor_agent` from DB for each pending task.
2. Dispatch pending tasks via assigned executor (default: copilot; use claude only when explicitly assigned).
3. Pass to each task: worktree path, constraints, readiness bundle, CI knowledge.
4. Wait for ALL tasks in wave to reach `submitted`.
5. Run Thor gate: `bash claude-config/scripts/plan-db.sh validate-wave {{wave_db_id}}` — promotes submitted → done, closes wave. NEVER skip. NEVER proceed to next wave without this.
6. Apply wave merge mode (`sync` / `batch` / `none`).
7. Output: `--- Wave WX --- Thor: PASS`

**Per-task mechanical gates (before submit):**

| Check | How |
|---|---|
| Files exist | `test -f` for each artifact |
| Verify commands | Run ALL from `test_criteria.verify[]` |
| Tests pass | Language-appropriate test runner |
| Typecheck | Language-appropriate type checker |
| Line limits | `wc -l < file` (max 250) |

### Phase 3: CI Batch Fix

Wait for FULL CI. Collect ALL failures. Fix ALL in one commit. Push once. Max 3 rounds.

### Phase 4: Completion

After ALL waves done: validate and complete plan in DB.
Output: `=== COMPLETE ===`

## Output

- Per-task: `[N/total] task_id: title -> DONE`
- Per wave: `--- Wave WX --- Thor: PASS`
- Final: `=== COMPLETE ===`

## Guardrails

- NEVER advance to next wave without Thor gate passing.
- NEVER set task status=done directly — only Thor can do this.
- NEVER skip readiness checks or drift check.
- NEVER use bare script names — always use full path `bash claude-config/scripts/<script>.sh`.
- NEVER retry the same failing approach more than twice — mark blocked instead.
