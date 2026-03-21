# Execute — Task Routing & Tracking

## DB Schema (CRITICAL — do NOT guess column names)

Tasks table PK is `id` (INTEGER), NOT `db_id`. Full schema: `@reference/operational/plan-db-schema.md`

Key columns: `id` (PK), `plan_id`, `wave_id_fk`, `task_id` (human string like T1-01),
`title`, `status`, `description`, `model`, `executor_agent`, `validator_agent`,
`output_type`, `test_criteria`, `effort_level`, `notes`, `started_at`, `completed_at`.

_Why: Plan 677 — executor queried `SELECT db_id` which doesn't exist. Column is `id`._

## plan-db.sh Commands (ONLY use these — do NOT invent commands)

```
# View plan/tasks:
execution-tree <plan_id>          # Tree view with statuses
show <plan_id>                    # Alias for execution-tree
tasks <plan_id>                   # Alias for execution-tree
json <plan_id>                    # Full plan as JSON
status [project_id]               # Quick status
task-detail <plan_id> <task_id>   # Single task JSON (e.g. task-detail 680 W1-01)

# Task status updates:
update-task <task_db_id> <status> [notes]   # pending|in_progress|submitted|done|blocked
update-wave <wave_db_id> <status>           # pending|in_progress|done|blocked

# Validation:
drift-check <plan_id>             # JSON — check branch_behind, NOT exit code
evaluate-wave <wave_db_id>        # Check wave preconditions
validate-wave <wave_db_id> [by]   # Thor validates all submitted tasks in wave
check-readiness <plan_id>         # Full readiness check

# Lifecycle:
start <plan_id>                   # Begin execution
complete <plan_id>                # Mark done
cancel <plan_id> [reason]         # Cancel

# Context:
get-context <plan_id>             # Full plan+tasks JSON for executor prompt (PREFERRED)
get-worktree <plan_id>            # Worktree path
```

Commands that DO NOT EXIST: `list-tasks`, `get-tasks`, `show-tasks`, `task-list`, `task-info`.
Use `execution-tree` (or `show`), `task-detail`, `get-context`, or direct SQL instead.
_Why: Plan 677 — executor called `list-tasks` which doesn't exist, causing error cascade._

## Routing

Read per-task from DB:
```sql
SELECT id, task_id, title, status, model, executor_agent, output_type, validator_agent, test_criteria, description
FROM tasks WHERE plan_id = {plan_id} AND wave_id_fk = {wave_db_id} AND status = 'pending'
ORDER BY task_id;
```

| Column | Use |
|---|---|
| `id` | DB primary key (pass to copilot-worker.sh and plan-db-safe.sh) |
| `task_id` | Human ID (T1-01, T2a-01, etc) — for display/logging |
| `model` | Model to use (claude-sonnet-4.6, gpt-5, etc) |
| `executor_agent` | `task-executor` (claude) or `task-executor-copilot` (copilot) |
| `output_type` | `pr` (code), `document`, `analysis`, `design`, `legal_opinion` |
| `validator_agent` | Who validates: `thor`, `doc-validator`, `strategy-validator`, etc |
| `test_criteria` | JSON with `verify[]` array — commands Thor runs to validate |

## Dispatch

### Claude tasks (`executor_agent = task-executor`)
Use `Task(subagent_type="task-executor")` with full prompt including task description,
worktree path, verify commands, and constraints.

### Copilot tasks (`executor_agent = task-executor-copilot`)
Use the copilot worker script — it handles prompt generation, launch, monitoring, and cleanup:
```bash
# Launch copilot worker for a task (pass the DB task ID, not task_id string)
bash claude-config/scripts/copilot-worker.sh {db_task_id} --model gpt-5
```
The worker script:
1. Generates prompt via `copilot-task-prompt.sh` (reads task from DB, loads context)
2. Launches `gh copilot` with the prompt in the correct worktree
3. Monitors completion and tracks agent lifecycle in DB
4. On success: marks task `submitted` automatically
5. On failure: logs error, leaves task `in_progress` for retry

To get the DB task ID from task_id string:
```sql
SELECT id FROM tasks WHERE plan_id = {plan_id} AND task_id = '{task_id}';
```

Copilot reference: `@reference/operational/copilot-alignment.md`
Copilot scripts: `claude-config/scripts/copilot-worker.sh`, `copilot-task-prompt.sh`

### Every dispatch prompt MUST include:
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
NEVER mark done directly — Thor does that via `bash claude-config/scripts/plan-db.sh validate-wave {wave_db_id}`.

To get task_db_id from task_id string:
```bash
sqlite3 "$DASHBOARD_DB" "SELECT id FROM tasks WHERE plan_id = 677 AND task_id = 'T1-01';"
```

To get wave_db_id:
```bash
sqlite3 "$DASHBOARD_DB" "SELECT id FROM waves WHERE plan_id = 677 AND wave_id = 'W1';"
```

## Per-Task Mechanical Gates (before submitted)

| Gate | Command |
|---|---|
| Files exist | `test -f` for each artifact created |
| Verify array | Run ALL commands from `test_criteria.verify[]` — ALL must exit 0 |
| Line limits | Every new/modified file ≤250 lines |
| Type check | Rust: `cargo check` · TS: `npx tsc --noEmit` · Bash: `bash -n` |
