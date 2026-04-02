# Ali Orchestrator — Daemon Background Service

## What you're building

A Rust background service (`tokio::spawn`) that runs inside the Convergio daemon and orchestrates plan execution by listening to the IPC message bus and reacting to events. **Event-driven, zero polling.**

Ali is the "chief of staff" — it watches what happens, checks dependencies, picks the right machine and model, delegates work, and follows up. Pure Rust logic, no LLM calls.

## Where it plugs in

The daemon starts in `daemon/src/mesh/daemon/service.rs` function `run_service()`. At line 42, an `IpcEngine` is created:

```rust
let ipc_engine = std::sync::Arc::new(crate::ipc::IpcEngine::new(config.db_path.clone()));
```

This engine has a `tokio::Notify` inside (`ipc_engine.notify`) that fires every time a message is sent. The `receive_wait()` method blocks (zero-cost) until a message arrives.

After the IPC socket server is spawned (line 49-53), **add Ali's spawn here**. Ali needs:
- The `Arc<IpcEngine>` (to send/receive messages)
- The `config.db_path` (to open SQLite connections for plan queries)

The HTTP server runs separately in `daemon/src/server/mod.rs` and shares the same database. Both Ali and the HTTP server can read/write to the same SQLite WAL database concurrently.

## The IPC bus (already working, use as-is)

Read `daemon/src/ipc/engine/messaging.rs`. Key methods on `IpcEngine`:

```rust
// Send to specific agent (stores in ipc_messages table, wakes Notify)
fn send_message(&self, from, to, content, msg_type, priority) -> Result<IpcResponse>

// Broadcast to channel (to_agent=NULL, channel=name)
fn broadcast(&self, from, content, msg_type, channel) -> Result<IpcResponse>

// Block until message arrives on channel — ZERO COST WAIT via tokio::Notify
async fn receive_wait(&self, agent, from_filter, channel_filter, limit, timeout_secs) -> Result<IpcResponse>
```

`receive_wait` returns `IpcResponse::MessageList { messages }` where each message has: `id, from_agent, to_agent, channel, content, msg_type, created_at`.

The `content` field is a JSON string. Ali should parse it to determine event type and payload.

## The plan hierarchy (already working, use as-is)

Read `daemon/src/db/plan_hierarchy.rs`:

```rust
// Are all plans in depends_on column done/cancelled?
fn dependencies_met(conn: &Connection, plan_id: i64) -> Result<bool>

// Aggregate status from child plans
fn master_rollup(conn: &Connection, master_id: i64) -> Result<(done, total, status)>

// Full tree for a project
fn project_plan_tree(conn: &Connection, project_id: &str) -> Result<ProjectTree>
```

Plans have these columns: `id, status, parent_plan_id, depends_on TEXT, execution_mode TEXT`.

## The IPC router (already working, use as-is)

Read `daemon/src/ipc/router/dispatch.rs`:

```rust
// Analyze task description → TaskType + complexity
fn analyze_task(description: &str) -> TaskAnalysis

// Pick best model/provider/host from registry
fn route_task(conn: &Connection, description: &str) -> Result<Option<RouteDecision>>
```

## Mesh delegation (already working, use as-is)

The HTTP API on localhost:8420:
- `POST /api/mesh/exec` — run command on peer `{peer, command, args, timeout_secs}`
- `POST /api/mesh/delegate` — assign plan to peer `{plan_id, peer}`
- `GET /api/mesh/status` — returns `{peers: [{peer_name, is_online, capabilities, ...}]}`

## What to implement

### File structure

```
daemon/src/orchestrator/
  mod.rs        — pub fn spawn_ali(engine: Arc<IpcEngine>, db_path: PathBuf)
  reactor.rs    — Event loop: receive_wait → parse → dispatch to handler
  handlers.rs   — One handler per event type
  actions.rs    — Reusable actions: find_peer, delegate, check_deps, emit
```

Each file MUST be under 250 lines.

### mod.rs (~30 lines)

```rust
mod reactor;
mod handlers;
mod actions;

use crate::ipc::IpcEngine;
use std::path::PathBuf;
use std::sync::Arc;

const ALI_AGENT: &str = "ali-orchestrator";
const CHANNEL: &str = "#orchestration";

pub fn spawn_ali(engine: Arc<IpcEngine>, db_path: PathBuf) {
    tokio::spawn(async move {
        tracing::info!("ali-orchestrator: starting");
        // Create channel if not exists
        let _ = engine.channel_create(CHANNEL, Some("Plan orchestration events"), ALI_AGENT);
        // Register as agent
        let _ = engine.register(ALI_AGENT, "orchestrator", 0, &crate::ipc::IpcEngine::hostname(), None);
        // Run reactor
        reactor::run(engine, db_path).await;
    });
}
```

### reactor.rs (~80 lines)

The core loop. **MUST use receive_wait, NOT sleep/poll.**

```rust
pub async fn run(engine: Arc<IpcEngine>, db_path: PathBuf) {
    loop {
        // Block until message on #orchestration channel
        let resp = engine.receive_wait(
            super::ALI_AGENT,
            None,              // any sender
            Some(super::CHANNEL),
            10,                // up to 10 messages per batch
            300,               // 5 min timeout, then re-loop (keepalive)
        ).await;

        match resp {
            Ok(IpcResponse::MessageList { messages }) => {
                for msg in messages {
                    if let Err(e) = handle_message(&engine, &db_path, &msg).await {
                        tracing::error!("ali: handler error: {e}");
                        // Emit error event so it's visible
                        let _ = engine.broadcast(
                            super::ALI_AGENT,
                            &format!(r#"{{"type":"error","detail":"{}"}}"#, e),
                            "error",
                            Some(super::CHANNEL),
                        );
                    }
                }
            }
            Ok(_) => {} // empty list on timeout, just re-loop
            Err(e) => {
                tracing::error!("ali: receive_wait error: {e}, retrying in 5s");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    }
}
```

Parse `msg.content` as JSON. Extract `type` field. Route to handlers.

### handlers.rs (~150 lines)

| msg.content.type | Handler | What it does |
|---|---|---|
| `plan_started` | `on_plan_started(plan_id)` | Check dependencies_met(). If blocked → update status, emit blocked. If ok → call actions::delegate_plan() |
| `task_done` | `on_task_done(task_id, plan_id)` | Update counters. If wave complete → emit wave_done |
| `wave_done` | `on_wave_done(wave_id, plan_id)` | Emit wave_needs_validation (Thor will pick this up later; for now just log) |
| `wave_validated` | `on_wave_validated(wave_id, plan_id)` | If last wave → emit plan_done. Else → next wave ready |
| `plan_done` | `on_plan_done(plan_id)` | master_rollup(). Find sibling plans with deps met → emit plan_ready for each |
| `plan_ready` | `on_plan_ready(plan_id)` | Same as plan_started: delegate |
| `delegation_failed` | `on_delegation_failed(plan_id, peer, reason)` | Retry once on different peer. If no peers → emit need_human |

### actions.rs (~120 lines)

Reusable functions:

```rust
// Find best available peer from mesh status
pub async fn find_available_peer(db_path: &Path) -> Option<String>

// Delegate plan to peer: POST /api/mesh/delegate + emit event
pub async fn delegate_plan(engine: &IpcEngine, db_path: &Path, plan_id: i64) -> Result<()>

// Check and emit ready plans after a plan completes
pub fn check_unblocked_plans(engine: &IpcEngine, conn: &Connection, master_id: i64) -> Result<()>

// Emit structured event to #orchestration
pub fn emit(engine: &IpcEngine, event_type: &str, payload: &serde_json::Value) -> Result<()>
```

`find_available_peer` calls `GET http://localhost:8420/api/mesh/status` and picks the first online peer with capabilities matching the task. Use `reqwest` (already a dependency).

`delegate_plan` calls `POST http://localhost:8420/api/mesh/delegate` with `{plan_id, peer}`.

## How to wire it into the daemon

In `daemon/src/mesh/daemon/service.rs`, after line 53 (after IPC socket spawn), add:

```rust
// Spawn Ali orchestrator
let ali_engine = ipc_engine.clone();
let ali_db = config.db_path.clone();
crate::orchestrator::spawn_ali(ali_engine, ali_db);
```

In `daemon/src/main.rs`, add `mod orchestrator;` to the module list.

Make sure `daemon/src/orchestrator/mod.rs` is a proper module with `mod reactor; mod handlers; mod actions;`.

## How other components send events to Ali

Any component that changes plan/task/wave status should broadcast to #orchestration. For now, this means modifying **two existing API handlers**:

1. `api_plan_db_lifecycle/handlers.rs` — when plan status changes to "doing", broadcast `{"type":"plan_started","plan_id":N}`
2. The task update handler — when task status changes to "done", broadcast `{"type":"task_done","task_id":"T1-01","plan_id":N}`

Search for where `UPDATE plans SET status` and `UPDATE tasks SET status` happen. Add one `engine.broadcast()` call after each successful update. The engine is not in ServerState currently — you'll need to add `pub ipc_engine: Option<Arc<IpcEngine>>` to ServerState, initialized in the server startup.

## Testing

Write tests in each file. Use in-memory SQLite. Test:
1. `dependencies_met` returns true → handler emits delegate event
2. `dependencies_met` returns false → handler emits blocked event
3. Task done → wave complete detection
4. Plan done → sibling unblocking
5. No available peer → need_human event

## What NOT to do

- Do NOT add tokio::time::interval or sleep-based polling. The bus is event-driven.
- Do NOT call Claude/GPT from Ali. Ali is pure Rust logic.
- Do NOT modify the IPC engine code. Use it as-is.
- Do NOT use raw SQL for plan queries — use `plan_hierarchy.rs` functions.
- Do NOT hardcode peer names or model names.
- Do NOT create files over 250 lines.
- Do NOT skip tests.

## Verification

After implementation:
1. `cargo check` passes
2. `cargo test orchestrator` — all tests green
3. Start daemon: the log shows "ali-orchestrator: starting"
4. Send test event: `cvg bus send ali-orchestrator '{"type":"plan_started","plan_id":719}' event`
5. Ali should check dependencies, find a peer, and attempt delegation (will fail gracefully if no peer — that's ok)
6. Check #orchestration channel: `cvg bus read ali-orchestrator --channel '#orchestration'`
