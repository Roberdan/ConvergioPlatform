# IPC Agent API Reference

Base URL: `http://localhost:8420` | All responses: JSON with `"ok": true` | POST body: `Content-Type: application/json`

## 1. Agent Lifecycle

### POST /api/ipc/agents/register

Register or re-register an agent. Uses `INSERT OR REPLACE` — calling again updates the entry.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| agent_id | string | yes | -- | Unique agent name |
| host | string | yes | -- | Hostname |
| pid | integer | no | null | Process ID |
| worktree | string | no | null | Git worktree path |
| branch | string | no | null | Current branch |
| current_task | string | no | null | Current task ID |

```bash
curl -X POST http://localhost:8420/api/ipc/agents/register \
  -H 'Content-Type: application/json' \
  -d '{"agent_id":"planner","host":"mac-m5","pid":1234,"current_task":"T1"}'
```
**Response:** `{"ok": true, "agent_id": "planner"}` | **Error 500:** DB write failed

**WS event:** `{"type":"agent_registered","agent_id":"...","host":"..."}`

### POST /api/ipc/agents/unregister

Remove an agent by `(agent_id, host)` primary key.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| agent_id | string | yes | Agent name |
| host | string | yes | Hostname |

```bash
curl -X POST http://localhost:8420/api/ipc/agents/unregister \
  -H 'Content-Type: application/json' \
  -d '{"agent_id":"planner","host":"mac-m5"}'
```
**Response:** `{"ok": true}` | **Error 500:** DB delete failed

**WS event:** `{"type":"agent_unregistered","agent_id":"...","host":"..."}`

### POST /api/ipc/agents/heartbeat

Update `last_heartbeat` and optionally `current_task` for a registered agent.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| agent_id | string | yes | Agent name |
| host | string | yes | Hostname |
| current_task | string | no | Current task ID (null clears it) |

```bash
curl -X POST http://localhost:8420/api/ipc/agents/heartbeat \
  -H 'Content-Type: application/json' \
  -d '{"agent_id":"planner","host":"mac-m5","current_task":"T3"}'
```
**Response:** `{"ok": true}` | **Error 500:** DB update failed

### GET /api/ipc/agents

List all registered agents, ordered by most recent heartbeat.

```bash
curl http://localhost:8420/api/ipc/agents
```
**Response:**
```json
{
  "ok": true,
  "agents": [{
    "agent_id": "planner", "host": "mac-m5", "status": "active", "pid": 1234,
    "worktree": "/path/to/worktree", "branch": "plan-668-w1", "current_task": "T1",
    "last_heartbeat": "2026-03-19 10:00:00", "registered_at": "2026-03-19 09:50:00"
  }]
}
```

## 2. Messaging

### POST /api/ipc/send

Send a message to a channel. Auto-creates the channel if it does not exist.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| sender_name | string | yes | -- | Sender identifier |
| content | string | yes | -- | Message body |
| channel | string | no | "general" | Target channel |

```bash
curl -X POST http://localhost:8420/api/ipc/send \
  -H 'Content-Type: application/json' \
  -d '{"sender_name":"executor","content":"T3 done","channel":"wave-1"}'
```
**Response:** `{"ok": true}` | **Error 500:** DB insert failed

**WS event:** `{"type":"ipc_message","channel":"wave-1","sender":"executor","content":"T3 done"}`

### GET /api/ipc/messages

Retrieve messages, newest first.

| Query Param | Type | Default | Description |
|-------------|------|---------|-------------|
| channel | string | (all) | Filter by channel |
| limit | integer | 50 | Max messages |

```bash
curl 'http://localhost:8420/api/ipc/messages?channel=wave-1&limit=10'
```
**Response:**
```json
{
  "ok": true,
  "messages": [{"id": 42, "channel": "wave-1", "sender": "executor", "content": "T3 done", "created_at": "2026-03-19 10:05:00"}]
}
```

### GET /api/ipc/channels

List all known channels.

```bash
curl http://localhost:8420/api/ipc/channels
```
**Response:**
```json
{
  "ok": true,
  "channels": [
    {"name": "general", "description": null, "created_at": "2026-03-19 09:00:00"},
    {"name": "wave-1", "description": null, "created_at": "2026-03-19 09:30:00"}
  ]
}
```

## 3. Coordination

### GET /api/ipc/context

Retrieve shared key-value context entries.

```bash
curl http://localhost:8420/api/ipc/context
```
**Response:**
```json
{
  "ok": true,
  "context": [{"key": "current_wave", "value": "1", "updated_by": "coordinator", "updated_at": "2026-03-19 09:55:00"}]
}
```

### GET /api/ipc/locks

List all active file locks.

```bash
curl http://localhost:8420/api/ipc/locks
```
**Response:**
```json
{
  "ok": true,
  "locks": [{"file_pattern": "daemon/src/server/api_ipc.rs", "agent": "executor", "host": "mac-m5", "pid": 5678, "locked_at": "2026-03-19 10:01:00"}]
}
```

### GET /api/ipc/worktrees

List registered agent worktrees.

```bash
curl http://localhost:8420/api/ipc/worktrees
```
**Response:**
```json
{
  "ok": true,
  "worktrees": [{"agent": "executor", "host": "mac-m5", "branch": "plan-668-w1", "path": "/Users/dev/project-plan-668-w1", "registered_at": "2026-03-19 09:50:00"}]
}
```

### GET /api/ipc/conflicts

Detect file-lock conflicts (files locked by more than one agent).

```bash
curl http://localhost:8420/api/ipc/conflicts
```
**Response:**
```json
{"ok": true, "conflicts": [{"file_pattern": "dashboard/app.js", "agents": "executor,validator", "agent_count": 2}]}
```

## 4. Intelligence

### GET /api/ipc/skills

List all skills from the agent skill pool.

```bash
curl http://localhost:8420/api/ipc/skills
```
**Response:** `{"skills": [{"name": "planner", "agent": "coordinator", "description": "..."}]}`

### GET /api/ipc/models

List registered models and their capabilities.

```bash
curl http://localhost:8420/api/ipc/models
```
**Response:**
```json
{
  "models": [{"id": "claude-opus-4.6", "provider": "anthropic", "context_window": 1000000}],
  "capabilities": [{"model_id": "claude-opus-4.6", "capability": "planning"}]
}
```

### GET /api/ipc/budget

Get budget status for all subscriptions, including threshold alerts.

```bash
curl http://localhost:8420/api/ipc/budget
```
**Response:**
```json
{
  "budgets": [{
    "subscription": "anthropic-pro", "provider": "anthropic", "plan": "max",
    "budget_usd": 100.0, "status": {"spent": 42.5, "remaining": 57.5}, "alert": null
  }]
}
```

### GET /api/ipc/auth-status

Check token sync health and list managed tokens.

```bash
curl http://localhost:8420/api/ipc/auth-status
```
**Response:**
```json
{
  "health": {"synced": true, "last_check": "2026-03-19 10:00:00"},
  "tokens": [{"name": "claude", "status": "valid", "expires": null}]
}
```

### GET /api/ipc/route-history

Last 20 model-routing log entries (newest first).

```bash
curl http://localhost:8420/api/ipc/route-history
```
**Response:**
```json
{
  "history": [{
    "subscription": "anthropic-pro", "date": "2026-03-19", "tokens_in": 5000,
    "tokens_out": 1200, "cost": 0.03, "model": "claude-opus-4.6", "task": "T5"
  }]
}
```

## 5. System

### GET /api/ipc/status

Aggregate IPC system counts.

```bash
curl http://localhost:8420/api/ipc/status
```
**Response:** `{"ok": true, "agents_active": 3, "locks_active": 1, "messages_total": 47, "conflicts": 0}`

### GET /api/ipc/metrics

Platform-wide IPC metrics.

```bash
curl http://localhost:8420/api/ipc/metrics
```
**Response:**
```json
{
  "ipc_message_rate_1d": 120, "agent_count": 4, "model_count": 6,
  "avg_route_latency_ms": 0, "budget_usage": 42.5, "skill_requests_active": 1
}
```

### GET /api/ipc/logs

Retrieve recent IPC log entries from the in-memory ring buffer (max 1000 stored).

| Query Param | Type | Default | Description |
|-------------|------|---------|-------------|
| limit | integer | 100 | Max entries (capped at 1000) |

```bash
curl 'http://localhost:8420/api/ipc/logs?limit=20'
```
**Response:**
```json
{
  "logs": [{"timestamp": "1710842400", "level": "info", "module": "ipc", "message": "Agent planner registered"}],
  "count": 1
}
```

## 6. Common Patterns

### Pattern 1: Register, Send, Receive

Agent-to-agent messaging via a shared channel.

```bash
# 1. Register the sender
curl -X POST http://localhost:8420/api/ipc/agents/register \
  -H 'Content-Type: application/json' \
  -d '{"agent_id":"executor","host":"mac-m5","pid":5678,"branch":"plan-668-w1"}'

# 2. Send a message to the coordination channel
curl -X POST http://localhost:8420/api/ipc/send \
  -H 'Content-Type: application/json' \
  -d '{"sender_name":"executor","content":"T3 complete, merging","channel":"coord"}'

# 3. Another agent reads the channel
curl 'http://localhost:8420/api/ipc/messages?channel=coord&limit=5'
```

### Pattern 2: Register, Sync Skills, Route Task

Agent capability discovery and model routing lookup.

```bash
# 1. Register the coordinator agent
curl -X POST http://localhost:8420/api/ipc/agents/register \
  -H 'Content-Type: application/json' \
  -d '{"agent_id":"coordinator","host":"mac-m5","pid":9999}'

# 2. Discover available skills across all agents
curl http://localhost:8420/api/ipc/skills

# 3. Check model availability and budget before routing
curl http://localhost:8420/api/ipc/models
curl http://localhost:8420/api/ipc/budget

# 4. After task execution, review routing history
curl http://localhost:8420/api/ipc/route-history
```
