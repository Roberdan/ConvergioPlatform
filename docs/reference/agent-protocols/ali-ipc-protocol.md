---
name: ali-ipc-protocol
description: IPC protocol for Ali orchestrator — re-dispatch on failure, output passing, standard message schema.
type: reference
---

# Ali IPC Protocol

## Re-Dispatch on Failure (C3)

```
Agent fails task T1-01 (attempt 1/3)
  → Mark task in_progress, log failure reason
  → Re-dispatch SAME agent (might be transient)

Agent fails T1-01 (attempt 2/3)
  → Query catalog for ALTERNATIVE agent with same skill
  → Re-dispatch to alternative agent

Agent fails T1-01 (attempt 3/3)
  → ESCALATE to user: "T1-01 failed 3 times. Agents tried: [list]. Last error: [msg]"
  → Set task status = blocked
```

## Output Passing Between Agents (C4, E2)

When agent A produces output needed by agent B:

```bash
# Agent A stores output in shared context
curl -X POST $DAEMON_URL/api/ipc/context -d '{
  "key": "fiona_market_analysis",
  "value": "{\"path\":\"docs/market-analysis.md\",\"summary\":\"TAM $2B, 3 competitors, mobile-first\"}",
  "set_by": "fiona"
}'

# Ali tells agent B where to find it
convergio-bus.sh send ali matteo "Input ready: read docs/market-analysis.md (context key: fiona_market_analysis)"
```

## Standard IPC Message Schema (E3)

ALL agent messages MUST follow this JSON schema:

```json
{"type": "DONE|BLOCKED|PROGRESS", "task_id": "T1-01", "agent": "fiona",
 "summary": "Market analysis complete", "artifacts": ["docs/market-analysis.md"],
 "next_action": "ready_for_validation"}
```

### Message Types

| Type | Meaning | Action |
|---|---|---|
| `DONE` | Task complete, artifacts listed | Mark done, dispatch next |
| `BLOCKED` | Can't proceed, needs intervention | Analyze, re-assign or escalate |
| `PROGRESS` | Status update, percentage in summary | Log, no action required |
