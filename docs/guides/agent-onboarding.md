# Agent Onboarding Guide

Quick-start for AI agents integrating with ConvergioPlatform IPC.

## 1. Prerequisites

| What | Check | Fix |
|------|-------|-----|
| Daemon running | `curl -s localhost:8420/api/ipc/status` | `./daemon/start.sh` |
| DASHBOARD_DB set | `echo $DASHBOARD_DB` | `export DASHBOARD_DB=/path/to/data/dashboard.db` |
| setup.sh completed | `test -L ~/.claude/CLAUDE.md` | `./setup.sh` |
| Scripts on PATH | `which agent-bridge.sh` | `export PATH="$HOME/.claude/scripts:$PATH"` |

## 2. Agent Registration

**Via script (recommended):**

```bash
scripts/platform/agent-bridge.sh --register --name planner --type claude
```

**Via HTTP API:**

```bash
curl -X POST http://localhost:8420/api/ipc/agents/register \
  -H 'Content-Type: application/json' \
  -d '{"agent_id":"planner","host":"'"$(hostname)"'","pid":$$}'
```

Optional fields: `worktree`, `branch`, `current_task`.

**Via CLI:**

```bash
claude-core ipc register --name planner --agent-type claude
```

**Verify registration:**

```bash
curl -s localhost:8420/api/ipc/agents | jq '.agents[] | select(.agent_id=="planner")'
```

**Unregister:**

```bash
curl -X POST http://localhost:8420/api/ipc/agents/unregister \
  -H 'Content-Type: application/json' \
  -d '{"agent_id":"planner","host":"'"$(hostname)"'"}'
```

## 3. Skill Discovery

**List all skills:**

```bash
curl -s localhost:8420/api/ipc/skills | jq '.skills'
```

**Request a skill from another agent:**

```bash
claude-core ipc-intel request-skill planning --payload '{"task":"Create migration plan"}'
```

**Register your own skills:**

```bash
claude-core ipc-intel request-skill coding \
  --payload '{"source":"manual","agent":"copilot","confidence":"0.90"}'
```

**Worked example -- planner requests executor:**

```bash
# 1. Planner checks who can execute
curl -s localhost:8420/api/ipc/skills | jq '.skills[] | select(.skill=="execute")'

# 2. Planner sends task to executor
curl -X POST localhost:8420/api/ipc/send \
  -H 'Content-Type: application/json' \
  -d '{"sender_name":"planner","channel":"execute","content":"Execute T3: create setup.sh"}'

# 3. Executor picks up message
curl -s 'localhost:8420/api/ipc/messages?channel=execute&limit=1'
```

## 4. Messaging

**Send point-to-point:**

```bash
curl -X POST localhost:8420/api/ipc/send \
  -H 'Content-Type: application/json' \
  -d '{"sender_name":"planner","channel":"direct:executor","content":"T3 is ready"}'
```

**Broadcast on channel:**

```bash
curl -X POST localhost:8420/api/ipc/send \
  -H 'Content-Type: application/json' \
  -d '{"sender_name":"planner","channel":"planning","content":"W2 starting"}'
```

**Receive messages:**

```bash
curl -s 'localhost:8420/api/ipc/messages?channel=planning&limit=10' | jq '.messages'
```

Channels are auto-created on first message (no explicit create needed).

## 5. Lifecycle Management

| Event | Action | Command |
|-------|--------|---------|
| Agent start | Register | `agent-bridge.sh --register --name $NAME --type $TYPE` |
| Every 60s | Heartbeat | `agent-heartbeat.sh --name $NAME --task $CURRENT_TASK` |
| Before compaction | Checkpoint | `agent-bridge.sh --checkpoint --name $NAME --key context --value "$STATE"` |
| Agent stop | Unregister | `agent-bridge.sh --unregister --name $NAME` |

**Heartbeat via HTTP API:**

```bash
curl -X POST localhost:8420/api/ipc/agents/heartbeat \
  -H 'Content-Type: application/json' \
  -d '{"agent_id":"planner","host":"'"$(hostname)"'","current_task":"T5"}'
```

**Store/retrieve context (survives compaction):**

```bash
# Store
curl -X POST localhost:8420/api/ipc/context \
  -H 'Content-Type: application/json' \
  -d '{"key":"planner-state","value":"W2 T5 in_progress","updated_by":"planner"}'

# Retrieve
curl -s localhost:8420/api/ipc/context | jq '.context'
```

## 6. Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| "daemon not reachable" | Daemon not running | `./daemon/start.sh` |
| Agent not in /agents | Used /send instead of /register | `POST /api/ipc/agents/register` |
| Stale heartbeat | Heartbeat script not running | `agent-heartbeat.sh --name $NAME` |
| Skills empty | Skills not synced | `scripts/platform/agent-skills-sync.sh` |
| Permission denied | Scripts not executable | `chmod +x scripts/platform/*.sh` |
| DB locked | Concurrent writes | `dashboard-db-repair.sh` |

**Quick health check:**

```bash
curl -s localhost:8420/api/ipc/status | jq '.'
curl -s localhost:8420/api/ipc/agents | jq '.agents | length'
curl -s localhost:8420/api/ipc/skills | jq '.skills | length'
```

## 7. Agent Name Registry

| Name | Type | Default Model | Primary Skills | Registered By |
|------|------|--------------|----------------|---------------|
| planner | claude | opus | planning, review | SubagentStart hook |
| executor | copilot | codex | coding, testing, tdd | SubagentStart hook |
| reviewer (thor) | claude | opus | validation, quality | SubagentStart hook |
| explorer | claude | haiku | search, codebase-nav | SubagentStart hook |
| prompt | claude | opus | requirements, f-xx | SubagentStart hook |
| researcher | claude | haiku | web-search, analysis | SubagentStart hook |
| copilot-worker | copilot | codex | coding, refactoring | copilot-bridge.sh |
| coordinator | claude | sonnet | dispatch, checkpoint | Session start |

## API Quick Reference

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/ipc/status` | GET | Daemon IPC status |
| `/api/ipc/agents` | GET | List registered agents |
| `/api/ipc/agents/register` | POST | Register agent |
| `/api/ipc/agents/unregister` | POST | Unregister agent |
| `/api/ipc/agents/heartbeat` | POST | Update heartbeat |
| `/api/ipc/send` | POST | Send message |
| `/api/ipc/messages` | GET | Read messages (?channel=&limit=) |
| `/api/ipc/channels` | GET | List channels |
| `/api/ipc/context` | GET/POST | Shared context store |
| `/api/ipc/skills` | GET | List skill pool |
| `/api/ipc/locks` | GET | File lock status |
| `/api/ipc/budget` | GET | Token budget info |
| `/api/ipc/models` | GET | Available models |
