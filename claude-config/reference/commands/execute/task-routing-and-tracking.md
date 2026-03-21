# Execute — Task Routing & Tracking

## Routing

Read per-task from DB:
```sql
SELECT task_id, model, executor_agent, output_type, validator_agent, test_criteria
FROM tasks WHERE plan_id = {plan_id} AND wave_id_fk = {wave_db_id} AND status = 'pending'
ORDER BY task_id;
```

| Field | Use |
|---|---|
| `model` | Model to use (claude-sonnet-4.6, gpt-5, etc) |
| `executor_agent` | `task-executor` (claude) or `task-executor-copilot` (copilot) |
| `output_type` | `pr` (code), `document`, `analysis`, `design`, `legal_opinion` |
| `validator_agent` | Who validates: `thor`, `doc-validator`, `strategy-validator`, etc |
| `test_criteria` | JSON with `verify[]` array — commands Thor runs to validate |

## Dispatch

For `task-executor` (claude): use `Task(subagent_type="task-executor")` or `/execute` delegation.
For `task-executor-copilot`: use Copilot CLI via `convergio` CLI or direct `gh copilot`.

Every dispatch prompt MUST include:
1. Task description (from `title` or `description`)
2. Worktree path (from wave `worktree_path`)
3. Verify commands (from `test_criteria.verify[]`)
4. Constraint: max 250 lines per file
5. Script paths: `bash claude-config/scripts/plan-db-safe.sh update-task {id} submitted`

## Status Tracking

```
pending → in_progress (executor starts)
in_progress → submitted (executor finishes, mechanical gates pass)
submitted → done (ONLY Thor/validator via validate-wave)
```

Mark in_progress: `bash claude-config/scripts/plan-db-safe.sh update-task {task_db_id} in_progress`
Mark submitted: `bash claude-config/scripts/plan-db-safe.sh update-task {task_db_id} submitted "summary"`
NEVER mark done directly — Thor does that via validate-wave.

## Per-Task Mechanical Gates (before submitted)

| Gate | Command |
|---|---|
| Files exist | `test -f` for each artifact created |
| Verify array | Run ALL commands from `test_criteria.verify[]` — ALL must exit 0 |
| Line limits | Every new/modified file ≤250 lines |
| Type check | Rust: `cargo check` · TS: `npx tsc --noEmit` · Bash: `bash -n` |
