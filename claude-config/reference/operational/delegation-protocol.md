# Delegation Protocol

## Worker Lifecycle (NON-NEGOTIABLE)

Every delegated agent MUST follow this lifecycle:

```
START → REGISTER → WORK → COMPLETE → EXIT
```

### 1. START — First 3 commands

```bash
# Register this agent session with coordinator
cvg agent start "claude-$(hostname -s)" --task-id $FIRST_TASK_DB_ID

# Verify daemon is reachable and plan exists
curl -sf http://localhost:8420/api/health || exit 1
cvg delegation status $PLAN_ID
```

### 2. WORK — Per wave

```bash
# Mark task started
cvg task update $TASK_DB_ID in_progress

# ... do the work ...

# Mark task submitted (NOT done — only Thor sets done)
cvg task update $TASK_DB_ID submitted --summary "what was done"

# After all tasks in wave submitted:
cvg plan validate $PLAN_ID

# Commit code
git add -A && git commit -m "feat(plan-$PLAN_ID): $WAVE_ID summary"
```

### 3. COMPLETE — When all waves done

```bash
# Push code
git push origin $(git branch --show-current)

# Complete agent session
cvg agent complete "claude-$(hostname -s)" --summary "W4-W12 done, 216 tests added"

# Final status report
cvg delegation status $PLAN_ID
```

### 4. EXIT

```bash
/exit
```

## Delegation Prompt Template

When delegating to a worker, use this EXACT template:

```
You are agent "claude-{hostname}" executing Plan {plan_id} on {peer_name}.
Branch: {branch}. Waves {from_wave}-{to_wave} are your scope.

FIRST: Run these setup commands:
  cvg agent start "claude-{hostname}" --task-id {first_task_db_id}
  cvg delegation status {plan_id}

FOR EACH TASK:
  cvg task update {db_id} in_progress
  [do the work — TDD, verify, cargo check/test]
  cvg task update {db_id} submitted --summary "..."

AFTER EACH WAVE:
  cvg plan validate {plan_id}
  git add -A && git commit -m "feat(plan-{plan_id}): {wave} summary"

WHEN ALL WAVES DONE:
  git push origin {branch}
  cvg agent complete "claude-{hostname}"
  cvg delegation status {plan_id}
  /exit

EXACT cvg CLI commands (do NOT guess):
  cvg task update <db_id> <status>     — pending/in_progress/submitted
  cvg plan validate <plan_id>          — Thor validates submitted→done
  cvg plan tree <plan_id>              — see wave/task status
  cvg agent start <name>               — register this session
  cvg agent complete <name>            — mark session done
  cvg delegation status <plan_id>      — check plan progress

Do NOT use: plan-db.sh, validate-wave, validate-task. These are deprecated.
Be fully autonomous. Commit after each wave. Do NOT ask for confirmation.
```
