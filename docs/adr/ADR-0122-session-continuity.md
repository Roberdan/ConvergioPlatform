# ADR-0122: Recursive Session Continuity

**Status**: Accepted
**Date**: 28 Marzo 2026

## Context

Long-running plans exceed a single session's context window. Compaction loses state. Delegated agents on remote nodes lack context and produce unvalidated output.

## Decision

Implement recursive session continuity with three layers:

1. **copilot-plan-runner.sh** — Loop that spawns fresh CLI sessions (`claude` or `copilot`) until plan is 100% complete. Resets stuck tasks between iterations.
2. **PreCompact hook** — Safety net. Before compaction: checkpoint plan state, spawn runner as new tab in `Convergio` tmux session.
3. **Memory checkpoint** — `project_plan{id}_execution_state.md` written to claude memory with task status, key decisions, resume commands.

## Architecture

```
Session N (coordinator or executor)
  ↓ context degrades or compaction imminent
  ↓ PreCompact hook fires:
      1. cvg checkpoint save <plan_id>
      2. tmux new-window -t Convergio -n plan-<id> 'copilot-plan-runner.sh <id>'

Session N+1 (fresh copilot)
  ↓ reads memory → cvg plan show <id> → finds pending tasks
  ↓ /execute <id> → works on next tasks
  ↓ exits when done or context limit
  ↓ runner loop relaunches

Session N+2 ...
  ↓ repeats until plan_done() returns true
```

## Configuration

| Node | Claude | Copilot | Hook |
|------|--------|---------|------|
| M5Max | ~/.claude/settings.json PreCompact | ~/.copilot/config.json preCompact | precompact-db-audit.sh + preserve-context.sh |
| M1 Pro | ~/.claude/settings.json PreCompact | ~/.copilot/config.json preCompact | precompact-db-audit.sh + preserve-context.sh |

Note: `track-precompact.sh` was removed — telemetry is now handled by the daemon. PreCompact hooks are `precompact-db-audit.sh` and `preserve-context.sh`.

Both CLIs supported. Runner auto-detects available CLI.

## Tmux Convention

All plan continuations run as tabs in the `Convergio` tmux session: `tmux new-window -t Convergio -n plan-<id>`.

## Consequences

- Plans can span unlimited sessions without state loss
- Each session starts fresh with full context window
- Coordinator does triage+planning, executors do implementation
- No single session needs to complete an entire plan
