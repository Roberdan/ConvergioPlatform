# Plan Database Schema Reference

**Database Location**: `$HOME/.claude/data/dashboard.db`

## Core Tables

### plans

| Column | Type | Key | Notes |
|--------|------|-----|-------|
| id | INTEGER | PK | AUTO |
| project_id | TEXT | FK→projects | NOT NULL |
| name | TEXT | | NOT NULL, UNIQUE(project_id, name) |
| status | TEXT | | CHECK('todo','doing','done','cancelled') |
| tasks_total, tasks_done | INTEGER | | Counter-maintained |
| worktree_path | TEXT | | Plan-level worktree |
| source_file, markdown_path, markdown_dir | TEXT | | Plan source refs |
| parallel_mode | TEXT | | 'standard' default |
| created_at, started_at, completed_at, validated_at | DATETIME | | Lifecycle |
| validated_by, execution_host, description, human_summary | TEXT | | Metadata |
| lines_added, lines_removed | INTEGER | | Stats |
| cancelled_at, cancelled_reason | TEXT | | Cancellation |

### waves

| Column | Type | Key | Notes |
|--------|------|-----|-------|
| id | INTEGER | PK | AUTO |
| plan_id | INTEGER | FK→plans | |
| wave_id | TEXT | | Human-readable ID |
| status | TEXT | | CHECK('pending','in_progress','done','blocked','merging','cancelled') |
| tasks_done, tasks_total | INTEGER | | Counter-maintained |
| position | INTEGER | | Sort order |
| worktree_path, branch_name | TEXT | | Wave-level isolation |
| pr_number, pr_url | TEXT | | PR tracking |
| precondition | TEXT | | JSON preconditions |
| merge_mode | TEXT | | 'sync' default |

**Trigger**: `wave_auto_complete` — wave→'merging' when tasks_done = tasks_total

### tasks

| Column | Type | Key | Notes |
|--------|------|-----|-------|
| id | INTEGER | PK | AUTO |
| plan_id | INTEGER | FK→plans | |
| wave_id_fk | INTEGER | FK→waves(id) | **INTEGER FK, not TEXT wave_id** |
| wave_id | TEXT | | Human-readable, NOT the FK |
| task_id | TEXT | | Human-readable ID |
| status | TEXT | | CHECK('pending','in_progress','submitted','done','blocked','skipped','cancelled') |
| priority | TEXT | | CHECK('P0','P1','P2','P3') |
| type | TEXT | | CHECK('bug','feature','fix','refactor','test','config','documentation','chore','doc') |
| effort_level | INTEGER | | CHECK(1,2,3) — INTEGER not string |
| model | TEXT | | Default 'claude-haiku-4.5' |
| validated_at, validated_by | TEXT | | Thor validation |
| validation_report | TEXT | | Thor report JSON |
| output_data | TEXT | | Inter-wave data passing |
| executor_host, executor_agent, executor_session_id | TEXT | | Execution tracking |
| executor_status | TEXT | | CHECK('idle','running','paused','completed','failed') |
| test_criteria, description, notes | TEXT | | Task details |
| privacy_required | BOOLEAN | | Default 0 |

**Triggers**:
- `enforce_thor_done` — BLOCKS status→done unless OLD.status='submitted' AND validated_by IN ('thor','thor-quality-assurance-guardian','thor-per-wave','forced-admin')
- `task_done_counter` — Increments waves.tasks_done and plans.tasks_done on status→done
- `task_undone_counter` — Decrements counters on done→other status

## Critical Notes

- **wave_id_fk** is INTEGER FK to waves(id) — do NOT confuse with TEXT wave_id
- Use `plan-db-safe.sh update-task <id> done` — never `plan-db.sh update-task done` directly
- Triggers maintain counters automatically — manual updates cause drift
- Full CLI reference: `plan-db.sh --help` or `plan-db.sh` with no args

## Indexes

- `idx_plans_project` ON plans(project_id, status)
- `idx_waves_plan` ON waves(plan_id, position)
- `idx_tasks_plan_status` ON tasks(plan_id, status, wave_id_fk)
- `idx_tasks_executor_active` ON tasks(executor_status) WHERE executor_status IN ('running', 'paused')
