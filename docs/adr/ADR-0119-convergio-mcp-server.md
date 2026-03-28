# Convergio MCP Server -- Design Document

**Author**: plan-reviewer agent (research/design task)
**Date**: 27 Marzo 2026
**Status**: DRAFT
**Daemon version**: 18.4.0

---

## 1. Problem Statement

Convergio Platform has a Rust daemon on `localhost:8420` with 250+ API endpoints covering plans, agents, mesh, kernel, metrics, notifications, and more. It has an MCP **client** (`daemon/src/capabilities/mcp.rs` -- `McpConnector`) that connects to external MCP servers via stdio transport. However, the daemon does not expose itself as an MCP server.

Any MCP-compatible LLM client (Claude Code, GitHub Copilot, local Mistral/Qwen via kernel, Cursor, Windsurf) must currently use raw HTTP calls to interact with the daemon. An MCP server would provide a standard tool-discovery and invocation interface, enabling any LLM to discover and use Convergio capabilities without custom integration code.

---

## 2. Architecture Decision

### Q1: Where should the MCP server code live?

**Decision**: `daemon/src/mcp_server/` as a new module within the daemon crate.

**Rationale**: The MCP server is a thin JSON-RPC/stdio adapter over the daemon's HTTP API. It does not need direct access to `ServerState`, the DB pool, or internal structs. It calls `localhost:8420` just like the existing kernel tools (`daemon/src/kernel/tools.rs`) already do. This keeps the MCP server decoupled from daemon internals while living in the same crate for build convenience.

```
daemon/src/mcp_server/
    mod.rs          -- McpServer struct, stdio loop, JSON-RPC dispatch
    protocol.rs     -- JSON-RPC types (request, response, error, notification)
    tools.rs        -- Tool registry: definitions + input schemas
    handlers.rs     -- Tool handler implementations (HTTP calls to daemon)
    security.rs     -- Ring-based access control for MCP callers
```

### Q2: Separate binary or part of the daemon?

**Decision**: Separate binary, same crate.

```toml
# daemon/Cargo.toml
[[bin]]
name = "convergio-mcp-server"
path = "src/mcp_server/main.rs"
```

**Rationale**:
- MCP servers use **stdio transport**: the client spawns the server process, communicates over stdin/stdout. This is fundamentally incompatible with being part of the long-running daemon process.
- The daemon runs on port 8420 and serves HTTP/WS. The MCP server is a short-lived (or session-lived) process spawned by each LLM client.
- Sharing the crate means shared types, shared `reqwest` HTTP client code, and a single `cargo build` produces both binaries.
- The existing `McpConnector` (client) already models this pattern: it spawns a child process and talks over stdio.

**Lifecycle**:
```
Claude Code                    convergio-mcp-server           Daemon (:8420)
    |                                |                              |
    |--- spawn process ------------->|                              |
    |--- initialize (JSON-RPC) ----->|                              |
    |<-- capabilities response ------|                              |
    |--- tools/list (JSON-RPC) ----->|                              |
    |<-- tool definitions -----------|                              |
    |--- tools/call "list_plans" --->|--- GET /api/plan-db/list --->|
    |<-- result ---------------------|<-- JSON response ------------|
    |--- tools/call "notify" ------->|--- POST /api/notify -------->|
    |<-- result ---------------------|<-- JSON response ------------|
```

### Q3: How do we secure it?

**Decision**: Ring-based access control, inheriting the existing capability system.

The MCP server authenticates callers using the same ring model (`daemon/src/capabilities/ring.rs`):

| Ring | Caller | Tools Available |
|------|--------|-----------------|
| 0 (Core) | Daemon itself, kernel engine | All tools including `restart_node`, `checkpoint_save` |
| 1 (Trusted) | Claude Code, registered Copilot agents | Plans (read/write), agents, mesh, metrics, kernel, notify |
| 2 (Community) | Community skills, external agents | Plans (read), agents (read), mesh (read), metrics (read) |
| 3 (Sandboxed) | Unknown/unregistered callers | Plans (read), agents (read) only |

**Implementation**:

1. **Environment variable authentication**: The MCP server reads `CONVERGIO_MCP_RING` (default: 3) from its environment. The spawning client sets this based on trust level. Example Claude Code config:
   ```json
   {
     "mcpServers": {
       "convergio": {
         "command": "convergio-mcp-server",
         "env": { "CONVERGIO_MCP_RING": "1" }
       }
     }
   }
   ```

2. **Daemon-side token**: If `CONVERGIO_API_TOKEN` is set, the MCP server includes it in all HTTP requests to the daemon. This leverages the existing auth middleware (`daemon/src/server/middleware.rs` -- `AUTH_TOKEN`).

3. **Tool filtering**: `tools/list` only returns tools the caller's ring can access. `tools/call` rejects calls to tools above the caller's ring level.

4. **No network exposure**: The MCP server communicates with the daemon only via `localhost:8420`. It never listens on a network port itself (stdio only). The attack surface is limited to whoever can spawn the binary.

### Q4: How does the local kernel connect to it?

**Decision**: The kernel does NOT connect to the MCP server. The MCP server connects to the kernel (via the daemon API).

The kernel (`daemon/src/kernel/engine.rs`) already has its own tool system (`daemon/src/kernel/tools.rs`) that calls the daemon HTTP API. The kernel is an **internal component** of the daemon. The MCP server is an **external interface** for LLM clients.

However, the kernel can be an MCP **client** if needed, using the existing `McpConnector`:

```
Kernel (Mistral/Qwen via AppleFmBridge)
    |
    +--> kernel/tools.rs (direct HTTP to daemon) -- current path, keep it
    |
    +--> McpConnector (spawn convergio-mcp-server) -- optional, for MCP-native models
```

For v1, the kernel continues using its direct HTTP tool calls. If a future local model natively supports MCP tool use (function calling via MCP protocol), the kernel can spawn `convergio-mcp-server` as a child process using `McpConnector`.

### Q5: Minimal viable set of tools for v1?

**Decision**: 14 tools across 6 domains. Selected based on what LLM agents actually need during plan execution.

---

## 3. Tool Definitions (v1)

### 3.1 Plans Domain (Ring 2: read, Ring 1: write)

| Tool | Ring | Method | Daemon Endpoint | Description |
|------|------|--------|-----------------|-------------|
| `cvg_list_plans` | 2 | GET | `/api/plan-db/list` | List all plans with id, name, status, progress |
| `cvg_get_plan` | 2 | GET | `/api/plan-db/json/{plan_id}` | Full plan detail with tasks and waves |
| `cvg_update_task` | 1 | POST | `/api/plan-db/task/update` | Update task status (done/blocked/in_progress) |
| `cvg_checkpoint_save` | 1 | POST | `/api/plan-db/checkpoint/save` | Save plan checkpoint |

### 3.2 Agents Domain (Ring 2: read)

| Tool | Ring | Method | Daemon Endpoint | Description |
|------|------|--------|-----------------|-------------|
| `cvg_list_agents` | 2 | GET | `/api/ipc/agents` | List registered agents with status |
| `cvg_agent_start` | 1 | POST | `/api/plan-db/agent/start` | Register agent as active |
| `cvg_agent_complete` | 1 | POST | `/api/plan-db/agent/complete` | Mark agent as completed |

### 3.3 Mesh Domain (Ring 2: read)

| Tool | Ring | Method | Daemon Endpoint | Description |
|------|------|--------|-----------------|-------------|
| `cvg_mesh_status` | 2 | GET | `/api/mesh` | Peer topology and mesh state |
| `cvg_node_readiness` | 2 | GET | `/api/node/readiness` | Health checks for node |

### 3.4 Metrics Domain (Ring 2: read)

| Tool | Ring | Method | Daemon Endpoint | Description |
|------|------|--------|-----------------|-------------|
| `cvg_cost_summary` | 2 | GET | `/api/metrics/cost` | Spending overview across plans |

### 3.5 Kernel Domain (Ring 2: read, Ring 0: action)

| Tool | Ring | Method | Daemon Endpoint | Description |
|------|------|--------|-----------------|-------------|
| `cvg_kernel_status` | 2 | GET | `/api/kernel/status` | Models loaded, uptime, active node |
| `cvg_kernel_ask` | 1 | POST | `/api/kernel/ask` | Ask the local LLM a question |

### 3.6 Actions Domain (Ring 0-1: write)

| Tool | Ring | Method | Daemon Endpoint | Description |
|------|------|--------|-----------------|-------------|
| `cvg_notify` | 1 | POST | `/api/notify` | Send notification (Telegram/ntfy) |
| `cvg_restart_node` | 0 | POST | `/api/node/recover` | Trigger node recovery |

---

## 4. JSON-RPC Protocol Implementation

The MCP server implements the [Model Context Protocol](https://spec.modelcontextprotocol.io/) specification:

### 4.1 Initialization

```json
// Client -> Server
{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
  "protocolVersion": "2024-11-05",
  "capabilities": {},
  "clientInfo": {"name": "claude-code", "version": "1.0.0"}
}}

// Server -> Client
{"jsonrpc": "2.0", "id": 1, "result": {
  "protocolVersion": "2024-11-05",
  "capabilities": {"tools": {}},
  "serverInfo": {"name": "convergio-mcp-server", "version": "18.4.0"}
}}
```

### 4.2 Tool Discovery

```json
// Client -> Server
{"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}

// Server -> Client (filtered by caller ring)
{"jsonrpc": "2.0", "id": 2, "result": {
  "tools": [
    {
      "name": "cvg_list_plans",
      "description": "List all plans with id, name, status, tasks_done, tasks_total.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "status_filter": {
            "type": "string",
            "description": "Filter by status: todo, doing, done, cancelled",
            "enum": ["todo", "doing", "done", "cancelled"]
          }
        }
      }
    },
    {
      "name": "cvg_get_plan",
      "description": "Get full plan JSON with tasks, waves, and progress. Use plan_id from cvg_list_plans.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "plan_id": {"type": "integer", "description": "Plan ID"}
        },
        "required": ["plan_id"]
      }
    },
    {
      "name": "cvg_update_task",
      "description": "Update task status. Valid transitions: pending->in_progress, in_progress->submitted, submitted->done.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "task_id": {"type": "integer", "description": "Task ID"},
          "status": {
            "type": "string",
            "enum": ["in_progress", "submitted", "done", "blocked"]
          },
          "summary": {"type": "string", "description": "Completion summary (required for done)"}
        },
        "required": ["task_id", "status"]
      }
    }
  ]
}}
```

### 4.3 Tool Invocation

```json
// Client -> Server
{"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {
  "name": "cvg_list_plans",
  "arguments": {"status_filter": "doing"}
}}

// Server -> Client
{"jsonrpc": "2.0", "id": 3, "result": {
  "content": [
    {
      "type": "text",
      "text": "[{\"id\":712,\"name\":\"MCP Server Implementation\",\"status\":\"doing\",\"tasks_done\":3,\"tasks_total\":8}]"
    }
  ]
}}
```

### 4.4 Error Handling

```json
// Ring violation
{"jsonrpc": "2.0", "id": 4, "error": {
  "code": -32001,
  "message": "Ring violation: caller ring 2 cannot access tool cvg_restart_node (ring 0)"
}}

// Daemon unreachable
{"jsonrpc": "2.0", "id": 5, "error": {
  "code": -32002,
  "message": "Daemon unreachable at localhost:8420. Is the daemon running?"
}}

// Invalid input
{"jsonrpc": "2.0", "id": 6, "error": {
  "code": -32602,
  "message": "Invalid params: plan_id is required"
}}
```

Custom error codes:
| Code | Meaning |
|------|---------|
| -32001 | Ring violation (permission denied) |
| -32002 | Daemon unreachable |
| -32003 | Daemon returned error |
| -32600 | Invalid JSON-RPC request |
| -32601 | Method not found |
| -32602 | Invalid params |

---

## 5. Module Design

### 5.1 `mcp_server/main.rs` -- Entry point

```rust
// Separate binary entry point for convergio-mcp-server.
// Reads stdin line-by-line, dispatches JSON-RPC, writes to stdout.

use convergio_mcp_server::McpServer;

fn main() {
    let ring = std::env::var("CONVERGIO_MCP_RING")
        .ok()
        .and_then(|v| v.parse::<u8>().ok())
        .unwrap_or(3);
    let daemon_url = std::env::var("CONVERGIO_DAEMON_URL")
        .unwrap_or_else(|_| "http://localhost:8420".to_string());
    let token = std::env::var("CONVERGIO_API_TOKEN").ok();

    let server = McpServer::new(ring, &daemon_url, token.as_deref());
    server.run_stdio(); // blocking stdin/stdout loop
}
```

### 5.2 `mcp_server/mod.rs` -- Server core

```rust
pub struct McpServer {
    ring: Ring,
    daemon_url: String,
    api_token: Option<String>,
    tools: ToolRegistry,
}

impl McpServer {
    pub fn new(ring_level: u8, daemon_url: &str, token: Option<&str>) -> Self;

    /// Blocking stdio loop: read JSON-RPC from stdin, write responses to stdout.
    pub fn run_stdio(&self) {
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let reader = BufReader::new(stdin.lock());

        for line in reader.lines() {
            let line = match line { Ok(l) => l, Err(_) => break };
            if line.trim().is_empty() { continue; }

            let response = self.handle_request(&line);
            let mut out = stdout.lock();
            writeln!(out, "{}", response).ok();
            out.flush().ok();
        }
    }

    fn handle_request(&self, raw: &str) -> String;
    fn handle_initialize(&self, id: u64) -> JsonRpcResponse;
    fn handle_tools_list(&self, id: u64) -> JsonRpcResponse;
    fn handle_tools_call(&self, id: u64, params: Value) -> JsonRpcResponse;
}
```

### 5.3 `mcp_server/tools.rs` -- Tool registry

```rust
pub struct ToolRegistry {
    tools: Vec<McpTool>,
}

pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub ring: Ring,
    pub handler: fn(&str, Option<&str>, &Value) -> Result<Value, McpError>,
    // handler args: (daemon_url, api_token, arguments)
}

impl ToolRegistry {
    /// Build the full v1 tool set.
    pub fn v1() -> Self;

    /// Return tools visible to a given ring level.
    pub fn tools_for_ring(&self, ring: Ring) -> Vec<&McpTool>;

    /// Find a tool by name, checking ring access.
    pub fn get(&self, name: &str, ring: Ring) -> Result<&McpTool, McpError>;
}
```

### 5.4 `mcp_server/handlers.rs` -- HTTP bridge

Each handler function follows the same pattern: build URL, call daemon, transform response.

```rust
// Reuses the pattern from daemon/src/kernel/tools.rs

pub fn list_plans(daemon_url: &str, token: Option<&str>, args: &Value)
    -> Result<Value, McpError>
{
    let url = format!("{daemon_url}/api/plan-db/list");
    let body = http_get(&url, token)?;
    let plans = body.get("plans").cloned().unwrap_or(json!([]));
    // Optional: filter by status_filter arg
    Ok(plans)
}

pub fn update_task(daemon_url: &str, token: Option<&str>, args: &Value)
    -> Result<Value, McpError>
{
    let task_id = args.get("task_id")
        .and_then(|v| v.as_i64())
        .ok_or(McpError::InvalidParams("task_id required"))?;
    let status = args.get("status")
        .and_then(|v| v.as_str())
        .ok_or(McpError::InvalidParams("status required"))?;

    let url = format!("{daemon_url}/api/plan-db/task/update");
    let payload = json!({
        "task_id": task_id,
        "status": status,
        "summary": args.get("summary").and_then(|v| v.as_str()).unwrap_or("")
    });
    http_post(&url, token, &payload)
}
```

### 5.5 `mcp_server/security.rs` -- Ring enforcement

```rust
use crate::capabilities::ring::Ring;

/// Validate that caller ring can access tool ring.
pub fn check_ring_access(caller: Ring, tool: Ring) -> Result<(), McpError> {
    if caller.can_access(tool) {
        Ok(())
    } else {
        Err(McpError::RingViolation {
            caller: caller.as_u8(),
            tool: tool.as_u8(),
        })
    }
}
```

---

## 6. Client Configuration

### 6.1 Claude Code (`~/.claude/mcp.json`)

```json
{
  "mcpServers": {
    "convergio": {
      "command": "/Users/Roberdan/GitHub/ConvergioPlatform/daemon/target/release/convergio-mcp-server",
      "env": {
        "CONVERGIO_MCP_RING": "1",
        "CONVERGIO_DAEMON_URL": "http://localhost:8420"
      }
    }
  }
}
```

### 6.2 GitHub Copilot (`.github/copilot-mcp.json`)

```json
{
  "servers": {
    "convergio": {
      "type": "stdio",
      "command": "convergio-mcp-server",
      "env": {
        "CONVERGIO_MCP_RING": "1"
      }
    }
  }
}
```

### 6.3 Local Kernel (programmatic, via McpConnector)

```rust
// In kernel engine, if model supports MCP natively:
let mut mcp = McpConnector::new("convergio-mcp-server", &["--ring", "0"]);
mcp.connect()?;
let tools = mcp.list_tools()?;
// Inject tool descriptions into model prompt...
let result = mcp.invoke("cvg_list_plans", json!({}))?;
```

### 6.4 Any MCP-compatible client

```bash
# Manual stdio session (for testing)
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.1"}}}' | CONVERGIO_MCP_RING=2 convergio-mcp-server
```

---

## 7. File Structure

```
daemon/src/mcp_server/
    main.rs             -- Binary entry point (separate [[bin]])
    mod.rs              -- McpServer struct, stdio loop, JSON-RPC dispatch
    protocol.rs         -- JsonRpcRequest, JsonRpcResponse, McpError types
    tools.rs            -- ToolRegistry, McpTool definitions with input schemas
    handlers.rs         -- Tool implementations (HTTP calls to daemon API)
    security.rs         -- Ring-based access enforcement
    tests.rs            -- Unit tests (tool registry, ring checks, handler mocking)
```

**Lines budget** (250 lines/file max per project convention):

| File | Est. Lines | Notes |
|------|-----------|-------|
| main.rs | ~25 | Entry point only |
| mod.rs | ~120 | Stdio loop + JSON-RPC dispatch |
| protocol.rs | ~80 | Types + serialization |
| tools.rs | ~150 | 14 tool definitions with schemas |
| handlers.rs | ~200 | 14 handler functions + HTTP helpers |
| security.rs | ~40 | Ring check + error |
| tests.rs | ~200 | Coverage for all tools |

Total: ~815 lines across 7 files.

---

## 8. Dependencies

No new crate dependencies needed. The MCP server uses:

| Dependency | Already in Cargo.toml | Used For |
|------------|----------------------|----------|
| serde, serde_json | Yes | JSON-RPC serialization |
| reqwest (blocking) | Yes | HTTP calls to daemon |
| tracing | Yes | Structured logging (to stderr, not stdout) |

**Critical**: stdout is reserved for JSON-RPC protocol. All logging goes to stderr via `tracing` with `tracing_subscriber` writing to stderr.

---

## 9. Build and Install

```bash
# Build both binaries
cd daemon && cargo build --release

# Binaries produced:
#   target/release/convergio-platform-daemon     (existing)
#   target/release/convergio-mcp-server           (new)

# Optional: symlink to PATH
ln -sf $(pwd)/target/release/convergio-mcp-server /usr/local/bin/convergio-mcp-server
```

Add to `daemon/start.sh` health check:
```bash
# Verify MCP server binary exists
if [[ ! -f "$DAEMON_DIR/target/release/convergio-mcp-server" ]]; then
    echo "WARN: convergio-mcp-server not built. Run: cargo build --release"
fi
```

---

## 10. Testing Strategy

### 10.1 Unit Tests (`mcp_server/tests.rs`)

- Tool registry returns correct tools per ring level
- Ring 3 cannot see Ring 0/1 tools
- Ring 1 can see Ring 1/2/3 tools
- JSON-RPC parsing handles malformed input
- Handler functions return correct error on daemon unreachable
- Input validation rejects missing required params

### 10.2 Integration Tests

- Spawn MCP server process, send initialize + tools/list + tools/call via stdin/stdout
- Verify JSON-RPC response format matches MCP spec
- Verify tool invocation reaches daemon (requires running daemon)
- Test ring filtering end-to-end

### 10.3 Manual Testing

```bash
# Start daemon
./daemon/start.sh

# In another terminal, interactive MCP session:
CONVERGIO_MCP_RING=1 convergio-mcp-server <<'EOF'
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.1"}}}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"cvg_list_plans","arguments":{}}}
EOF
```

---

## 11. v2 Roadmap (Out of Scope for v1)

| Feature | Rationale | Complexity |
|---------|-----------|------------|
| **Resources** (MCP resources) | Expose plan specs, ADRs as readable resources | Medium |
| **Prompts** (MCP prompts) | Pre-built prompts for common workflows | Low |
| **SSE transport** | For long-running operations (delegation, mesh sync) | High |
| **Streaming responses** | For `cvg_kernel_ask` with token streaming | Medium |
| **Dynamic tool registration** | Skills register themselves as MCP tools | High |
| **Multi-daemon** | Connect to remote daemons via mesh | High |
| **Notifications** | Server-initiated events (task completed, mesh change) | Medium |

---

## 12. Relationship to Existing Code

```
daemon/src/
    capabilities/
        mcp.rs              <-- MCP CLIENT (connects TO external servers)
        ring.rs             <-- Ring enum (reused by MCP server security)
        types.rs            <-- ToolSchema, CapabilityError (reused)
        permissions.rs      <-- PermissionManager (pattern reference)
    kernel/
        tools.rs            <-- Existing HTTP-based tool calls (same pattern as handlers.rs)
        engine.rs           <-- KernelEngine (potential MCP client consumer)
    server/
        api_capabilities.rs <-- Capability REST API (registers MCP tools as capabilities)
        middleware.rs        <-- Auth middleware (token validation reused)
    mcp_server/             <-- NEW: MCP SERVER (exposes daemon AS an MCP server)
        main.rs
        mod.rs
        protocol.rs
        tools.rs
        handlers.rs
        security.rs
        tests.rs
```

The key distinction:
- `capabilities/mcp.rs` = Convergio **consumes** external MCP servers
- `mcp_server/` = Convergio **exposes itself** as an MCP server

These are complementary. An LLM client using the MCP server can invoke tools that themselves use capabilities backed by external MCP servers (via `McpConnector`). The daemon is both MCP client and MCP server.

---

## 13. Risk Assessment

| Risk | Mitigation |
|------|-----------|
| stdout pollution breaks JSON-RPC | All logging to stderr. No `println!` in MCP server code. Clippy lint + test assertion. |
| Daemon not running when MCP server starts | Health check on initialize. Clear error: "Daemon unreachable at localhost:8420". Retry with backoff on first tool call. |
| Ring escalation via env var tampering | Document that CONVERGIO_MCP_RING is set by the spawning client. In production, the daemon token provides the real auth boundary. |
| Blocking I/O in stdio loop | Acceptable for v1. Each MCP session is one-client-one-process. Async upgrade in v2 if needed. |
| Tool schema drift from daemon API | Handlers test against live daemon in integration tests. Schema definitions are the source of truth for LLMs. |

---

## 14. Implementation Plan (Estimated Effort)

| Wave | Tasks | Effort |
|------|-------|--------|
| W1 | protocol.rs + mod.rs (stdio loop + JSON-RPC) + security.rs | 2 |
| W2 | tools.rs (14 definitions) + handlers.rs (HTTP bridge) | 3 |
| W3 | main.rs + Cargo.toml [[bin]] + tests.rs | 2 |
| W4 | Client configs (Claude Code, Copilot) + integration test + docs | 1 |

Total: ~4 tasks, effort 8, single wave feasible for one executor.
