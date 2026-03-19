# API Reference — UI Endpoints

## Evolution CRUD

| Method | Endpoint | Description |
|---|---|---|
| GET | `/api/evolution/proposals` | List all proposals (sorted by `created_at` DESC) |
| POST | `/api/evolution/proposals/:id/approve` | Approve a pending proposal |
| POST | `/api/evolution/proposals/:id/reject` | Reject a pending proposal |
| GET | `/api/evolution/experiments` | List experiments with joined proposal data |
| GET | `/api/evolution/roi` | Aggregate ROI: success rate, rollbacks, proposals by status |
| GET | `/api/evolution/audit/:id` | Audit trail for a specific proposal |

### Approve/Reject Body

```json
{ "reason": "string (optional)", "actor": "string (optional, default: dashboard)" }
```

### Response

```json
{ "ok": true, "id": 1, "status": "approved" }
```

Error (400): `{ "error": "proposal not found or not in pending status" }`

## Delegate Cancel

| Method | Endpoint | Description |
|---|---|---|
| POST | `/api/mesh/delegate/:id/cancel` | Cancel an active delegation by ID |

Returns `{ "ok": true }` if cancelled, 404 if delegation not found.

Delegation IDs follow format: `del-{plan_id}-{target}-{timestamp_ms}`.

## WebSocket: Brain Push Events (`/ws/brain`)

All events use envelope: `{ "kind": "brain_event", "event_type": "<type>", "payload": {...} }`

| Event Type | Payload | Trigger |
|---|---|---|
| `agent_update` | `{ "agents": [{ name, host, agent_type, pid, metadata, registered_at, last_seen }] }` | Agent register/unregister |
| `task_update` | `{ "task_id": int, "status": string, "plan_id": int }` | Task status change |
| `session_update` | `{ "sessions": [{ agent_id, type, description, status, metadata, started_at, tokens_total, cost_usd, model }] }` | Session register/unregister/heartbeat |

## SSE: Chat Stream (`/api/chat/stream/:sid`)

Claude API events forwarded as SSE. Key event types:

| SSE Event | Data Shape | Description |
|---|---|---|
| `content_block_delta` | `{ "type": "content_block_delta", "index": int, "delta": { "type": "text_delta", "text": string } }` | Incremental text token from LLM |
| `message_start` | `{ "type": "message_start", "message": {...} }` | Stream begins |
| `message_delta` | `{ "type": "message_delta", "delta": { "stop_reason": string } }` | Stream ends |
| `error` | `{ "type": "error", "error": { "message": string } }` | Stream error |

## SSE: Delegate Stages (`/api/mesh/delegate`)

Query params: `plan_id`, `target`, `cli` (optional).

| SSE Event | Data Shape | Description |
|---|---|---|
| `stage` | `{ "type": "stage", "stage": string, "peer": string, "detail": string }` | Delegation progress |
| `delegate_complete` | `{ "type": "delegate_complete", "peer": string }` | Delegation finished |

### Delegate Stage Values

| Stage | Detail |
|---|---|
| `connecting` | Resolving peer, SSH handshake |
| `cloning` | `git fetch + checkout` |
| `spawning` | Launching CLI agent |
| `running` | Agent executing |

## Delegation Protocol

1. Client sends GET `/api/mesh/delegate?plan_id=X&target=worker-1`
2. Server resolves peer via `peers.conf`, opens SSH tunnel
3. SSE stages stream: `connecting` -> `cloning` -> `spawning` -> `running`
4. On completion: `delegate_complete` event, brain `task_update` broadcast
5. Client may cancel via POST `/api/mesh/delegate/:id/cancel` at any point
6. Cancelled delegations emit final `stage` with detail "cancelled"
