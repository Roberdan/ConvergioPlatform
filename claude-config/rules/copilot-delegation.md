# Copilot Delegation (NON-NEGOTIABLE)

## How to Delegate

| Need | Command |
|---|---|
| Single task | `copilot-worker.sh <db_task_id> --model claude-opus-4.6` |
| Full plan | `copilot-plan-runner.sh <plan_id>` |
| Task prompt | `copilot-task-prompt.sh <db_task_id> [role]` |

## Scripts (claude-config/scripts/)

| Script | Purpose |
|---|---|
| `copilot-worker.sh` | Execute single task: retries, timeout, agent tracking, TDD prompt, mesh events |
| `copilot-task-prompt.sh` | Generate context-rich prompt: task, wave, prior outputs, PR feedback, worktree |
| `copilot-plan-runner.sh` | Auto-restart loop until plan 100% complete |
| `copilot-bridge.sh` | Mesh delegation bridge (scripts/platform/) |

## In Plan Specs

Set `executor_agent: copilot` and `model: claude-opus-4.6` for delegated tasks. The `/execute` skill routes automatically.

## FORBIDDEN

NEVER delegate via GitHub Issues or `@copilot` assignee. Convergio scripts handle agent tracking, TDD, retries, plan DB updates. GitHub Issues bypass the orchestration system.
