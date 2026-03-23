# Convergio × OpenClaw Bridge — Execution Plan

**Target executor**: GitHub Copilot (Opus model)
**Project root**: `/Users/Roberdan/GitHub/ConvergioPlatform`
**Date**: 22 March 2026

## Context

**Convergio** is a platform with 40+ specialized AI agents (code review, security, strategy, legal, etc.) orchestrated by Ali (Chief of Staff) via a Rust daemon on `:8420` with IPC, mesh P2P, and a dashboard.

**OpenClaw** is the most popular open-source AI agent (250K+ GitHub stars) that operates via messaging apps (WhatsApp, Telegram, Slack, 30+ channels). It uses a Gateway + SKILL.md skill system + Lobster workflow engine.

**Goal**: Create a bridge so Convergio agents are accessible via OpenClaw messaging channels. OpenClaw plugin → HTTP → Convergio daemon API.

## Architecture

```
User (WhatsApp/Telegram/Slack)
  → OpenClaw Gateway (:18789)
    → convergio-bridge plugin
      → HTTP POST http://localhost:8420/api/openclaw/invoke
        → Convergio Daemon
          → Ali Orchestrator → Specialized Agent
          → IPC response
        ← JSON response
      ← formatted message
    ← platform delivery
  ← reply to user
```

## Key Files Reference

| What | Path |
|---|---|
| Agent definitions | `claude-config/agents/**/*.md` (YAML frontmatter + markdown) |
| Daemon server modules | `daemon/src/server/` |
| Route builder | `daemon/src/server/routes/mod.rs` (uses `.merge()` pattern) |
| Server state | `daemon/src/server/state.rs` (axum shared state) |
| IPC skills | `daemon/src/ipc/skills/` |
| Existing API pattern | `daemon/src/server/api_ipc/` (directory module to follow) |
| Platform scripts | `scripts/platform/` |
| Config dir | `config/` |
| Existing tests | `tests/` |
| Existing ADRs | `docs/adr/` (9 ADRs present) |

---

## Phase 1: Architecture Documents

### Task 1.1 — Architecture Decision Record

**Create** `docs/adr/adr-openclaw-bridge.md`

Write an ADR documenting the Convergio × OpenClaw integration architecture:

1. **Decision**: Use an OpenClaw TypeScript plugin that bridges to Convergio daemon HTTP API, rather than embedding Convergio logic directly in OpenClaw or vice versa
2. **Format mapping** — how Convergio agent definitions map to OpenClaw SKILL.md:
   - `name` → `metadata.openclaw.skillKey` (lowercase, hyphens)
   - `description` → `description`
   - `model` → noted in markdown body
   - `tools` → `metadata.openclaw.requires.bins` (conceptual mapping only — tools execute server-side)
   - `maturity` (stable/preview) → `version` (1.0.0/0.9.0)
3. **Session mapping**: OpenClaw session keys (`agent:<id>:<provider>:<chat>`) → Convergio IPC channels + shared context store at `/api/ipc/context`
4. **Auth bridge**: OpenClaw channel security (DM pairing, allowlists) → Convergio RBAC + privacy-aware dispatch (sensitive data → local agents only per `privacy.sensitive_agents` config)
5. **Orchestration**: Ali multi-agent dispatch within OpenClaw's session model. For complex requests, Ali receives the message, assembles a virtual team, dispatches to specialists via IPC, aggregates responses. OpenClaw sees a single session; Convergio handles the fan-out internally.
6. **Risk analysis**: API coupling (mitigated by HTTP boundary), OpenClaw format stability (mitigated by version pinning), data privacy (mitigated by agent allowlist/denylist + sensitive agent routing)

Constraints: max 250 lines. English only. Format: `## Status`, `## Context`, `## Decision`, `## Consequences`.

### Task 1.2 — Plugin Design Spec

**Create** `docs/design/openclaw-plugin-spec.md` (create `docs/design/` directory first)

Document the plugin contract:

1. **Plugin structure**:
   ```
   integrations/openclaw-bridge/
     openclaw.plugin.json    # OpenClaw plugin manifest
     package.json            # Node deps
     tsconfig.json           # TypeScript strict, ESM
     src/
       index.ts              # Plugin entry: register(api)
       client.ts             # ConvergioDaemonClient HTTP client
       types.ts              # Shared interfaces
       orchestrator.ts       # Ali routing logic
       session-bridge.ts     # Session ↔ IPC mapping
   ```

2. **Daemon API endpoints** (new, under `/api/openclaw/`):

   | Method | Path | Purpose |
   |---|---|---|
   | GET | `/api/openclaw/agents` | List agents in SKILL.md-compatible format (cached, 60s TTL) |
   | POST | `/api/openclaw/invoke` | Invoke agent with message + session context. Returns JSON `{request_id, status, response}`. Timeout: 120s. Error format: `{error: {code, message}}` |
   | GET | `/api/openclaw/session/:key` | Map OpenClaw session key to IPC channel. Creates channel if absent. |
   | POST | `/api/openclaw/webhook` | Receive OpenClaw gateway events (message_received, session_started, session_ended). Routes to IPC. |

3. **Data flow diagram** (text): message in → gateway → plugin `register(api)` → `api.registerTool('convergio-invoke')` → tool called by model → `ConvergioDaemonClient.invokeAgent()` → HTTP POST to daemon → IPC skill request → agent executes → IPC response → HTTP response → tool result → model formats → gateway delivers

4. **Error handling**: agent timeout (504 + retry), model failure (502 + fallback message), RBAC denial (403 + "agent unavailable"), daemon unreachable (503 + "service offline")

5. **Configuration** (in `openclaw.json`):
   ```json
   {
     "convergio": {
       "daemon_url": "http://localhost:8420",
       "auth_token": "${CONVERGIO_AUTH_TOKEN}",
       "default_agent": "ali-orchestrator",
       "timeout_seconds": 120,
       "direct_routing_threshold": 0.8
     }
   }
   ```

Max 250 lines. English only.

---

## Phase 2a: Plugin Core (parallel with Phase 2b)

### Task 2a.1 — Plugin Package Setup

**Create** directory `integrations/openclaw-bridge/` with these files:

**`package.json`**:
- name: `@convergio/openclaw-bridge`
- version: `0.1.0`
- type: `module`
- main: `dist/index.js`
- scripts: `build` (tsc), `test` (vitest or node --test)
- dependencies: none (use native fetch)
- devDependencies: `typescript`, `@types/node`

**`tsconfig.json`**: strict, ESM, target ES2022, outDir `dist/`, rootDir `src/`

**`openclaw.plugin.json`**:
```json
{
  "id": "convergio-bridge",
  "name": "Convergio Bridge",
  "version": "0.1.0",
  "description": "Access 40+ Convergio AI agents via OpenClaw messaging",
  "main": "dist/index.js"
}
```

**`src/types.ts`** — shared interfaces:
```typescript
export interface ConvergioAgent {
  name: string;
  description: string;
  model: string;
  tools: string[];
  maturity: 'stable' | 'preview';
  category: string;
}

export interface InvokeRequest {
  agent_id: string;
  message: string;
  session_key?: string;
  channel_context?: Record<string, string>;
}

export interface InvokeResponse {
  request_id: string;
  status: 'completed' | 'pending' | 'error';
  response?: string;
  agent: string;
  duration_ms?: number;
}

export interface ConvergioError {
  error: {
    code: string;
    message: string;
  };
}

export interface OpenClawConfig {
  daemon_url: string;
  auth_token?: string;
  default_agent: string;
  timeout_seconds: number;
  direct_routing_threshold: number;
}
```

Each file max 250 lines. No stubs — real, compilable TypeScript.

**Verify**: `cd integrations/openclaw-bridge && npx tsc --noEmit`

### Task 2a.2 — Daemon HTTP Client

**Create** `integrations/openclaw-bridge/src/client.ts`:

`ConvergioDaemonClient` class:
- `constructor(config: OpenClawConfig)` — stores daemon URL, auth token, timeout
- `async listAgents(): Promise<ConvergioAgent[]>` — GET `/api/openclaw/agents`, parse JSON, return typed array. Cache for 60s (in-memory Map with TTL).
- `async invokeAgent(req: InvokeRequest): Promise<InvokeResponse>` — POST `/api/openclaw/invoke` with JSON body. Use `AbortSignal.timeout(config.timeout_seconds * 1000)`. Handle HTTP errors: 403 → throw 'RBAC denied', 504 → throw 'Agent timeout', 503 → throw 'Daemon unreachable'.
- `async getSessionMapping(openclawKey: string): Promise<{channel: string, created: boolean}>` — GET `/api/openclaw/session/${key}`
- `async sendWebhook(event: string, data: Record<string, unknown>): Promise<void>` — POST `/api/openclaw/webhook`
- `async health(): Promise<boolean>` — GET `/api/health`, return true if 200

All methods: use native `fetch()`. Add `Authorization: Bearer ${token}` header if token configured. Structured error handling with typed errors. No external dependencies.

Max 250 lines.

**Verify**: `cd integrations/openclaw-bridge && npx tsc --noEmit`

### Task 2a.3 — Plugin Entry Point

**Create** `integrations/openclaw-bridge/src/index.ts`:

```typescript
export function register(api: any) {
  const config = loadConfig(api);
  const client = new ConvergioDaemonClient(config);

  api.registerTool({
    name: 'convergio-invoke',
    description: 'Invoke a Convergio AI agent (code review, security audit, strategy, etc.)',
    schema: {
      type: 'object',
      properties: {
        agent: { type: 'string', description: 'Agent name (e.g., rex-code-reviewer, luca-security-expert, ali-orchestrator)' },
        message: { type: 'string', description: 'Message/task for the agent' },
      },
      required: ['message'],
    },
    execute: async (params: { agent?: string; message: string }) => {
      const agentId = params.agent || config.default_agent;
      const response = await client.invokeAgent({
        agent_id: agentId,
        message: params.message,
      });
      return response.response || `Agent ${agentId} processing (request: ${response.request_id})`;
    },
  });

  api.registerTool({
    name: 'convergio-agents',
    description: 'List available Convergio agents and their capabilities',
    schema: { type: 'object', properties: {} },
    execute: async () => {
      const agents = await client.listAgents();
      return agents.map(a => `${a.name}: ${a.description} [${a.maturity}]`).join('\n');
    },
  });
}

function loadConfig(api: any): OpenClawConfig {
  const raw = api.getConfig?.('convergio') || {};
  return {
    daemon_url: raw.daemon_url || 'http://localhost:8420',
    auth_token: raw.auth_token || process.env.CONVERGIO_AUTH_TOKEN,
    default_agent: raw.default_agent || 'ali-orchestrator',
    timeout_seconds: raw.timeout_seconds || 120,
    direct_routing_threshold: raw.direct_routing_threshold || 0.8,
  };
}
```

Import from `./client.ts` and `./types.ts`. Wire everything — no dead imports, no unregistered code.

Max 250 lines.

**Verify**: `cd integrations/openclaw-bridge && npx tsc --noEmit`

### Task 2a.4 — Daemon API Endpoints (Rust)

**Create** `daemon/src/server/api_openclaw.rs` (max 250 lines):

Implement 4 endpoints using existing axum patterns from `api_ipc/`:

1. **GET `/api/openclaw/agents`** — Read agent catalog from `claude-config/agents/**/*.md`. Parse YAML frontmatter (between `---` markers). Extract: name, description, model, tools, maturity. Return JSON array. **Cache results** in a `tokio::sync::RwLock<Option<(Instant, Vec<Agent>)>>` with 60s TTL.

2. **POST `/api/openclaw/invoke`** — Accept `InvokeRequest` JSON. Create IPC skill request via `create_skill_request()` from `ipc/skills/executor.rs`. If agent specified, assign directly; otherwise default to `ali-orchestrator`. Return `{request_id, status: "pending", agent}`. If `Accept: text/event-stream`, use SSE to stream progress.

3. **GET `/api/openclaw/session/:key`** — Map OpenClaw session key to IPC channel name (sanitize key, create channel via IPC if absent). Return `{channel, created}`.

4. **POST `/api/openclaw/webhook`** — Accept `{event, data}`. Route `message_received` events to IPC message bus via `POST /api/ipc/send`. Log all events.

**Auth middleware**: Read `OPENCLAW_AUTH_TOKEN` from env. If set, require `Authorization: Bearer <token>` on all endpoints. Return 401 if missing/invalid. If env not set, endpoints are open (dev mode).

Create a `pub fn router() -> axum::Router<ServerState>` function.

**CRITICAL wiring** — also modify these existing files:
- `daemon/src/server/mod.rs`: add `pub mod api_openclaw;`
- `daemon/src/server/routes/mod.rs`: add `use super::api_openclaw;` and `.merge(api_openclaw::router())` in `build_router_with_db()` function, following the existing `.merge()` pattern

**Verify**: `cd daemon && cargo check` (must pass with 0 errors)

---

## Phase 2b: Skill Generator (parallel with Phase 2a)

### Task 2b.1 — Skill Generator Script

**Create** `scripts/platform/convergio-openclaw-skills.sh`

Bash script that auto-generates OpenClaw SKILL.md files from Convergio agent definitions.

```bash
#!/usr/bin/env bash
set -euo pipefail

# Usage: convergio-openclaw-skills.sh [--output-dir DIR] [--daemon-url URL]
# Reads: claude-config/agents/**/*.md
# Writes: <output-dir>/<agent-name>/SKILL.md + index.json
```

Logic:
1. Find all `.md` files in `claude-config/agents/` (recursive)
2. For each file, extract YAML frontmatter (between `---` markers)
3. Parse fields: `name`, `description`, `version` (default "1.0.0"), `model`, `tools`, `maturity`
4. Handle missing fields gracefully: `tools` defaults to `[]`, `model` defaults to "claude-sonnet-4-6", `maturity` defaults to "stable"
5. Generate `<output-dir>/<name>/SKILL.md`:
   ```yaml
   ---
   name: <name>
   description: <description>
   version: <version>
   metadata:
     openclaw:
       requires:
         env:
           - CONVERGIO_AUTH_TOKEN
         bins:
           - curl
           - jq
       primaryEnv: CONVERGIO_AUTH_TOKEN
       emoji: "🤖"
   ---

   # <Name> (Convergio Agent)

   <description>

   To invoke this agent, use the convergio-invoke tool or run:

   curl -X POST "${DAEMON_URL:-http://localhost:8420}/api/openclaw/invoke" \
     -H "Content-Type: application/json" \
     -H "Authorization: Bearer $CONVERGIO_AUTH_TOKEN" \
     -d '{"agent_id":"<name>","message":"$USER_MESSAGE"}'
   ```
6. Generate `index.json` listing all skills (for ClawHub compatibility)
7. Support `--help` flag showing usage
8. Idempotent (overwrites existing files)

Constraints: `set -euo pipefail`, quote all vars, use `local`, `trap cleanup EXIT`. Max 250 lines.

**Verify**:
- `test -x scripts/platform/convergio-openclaw-skills.sh`
- `bash scripts/platform/convergio-openclaw-skills.sh --help 2>&1 | grep -qi usage`
- `wc -l scripts/platform/convergio-openclaw-skills.sh | awk '{if($1 > 250) exit 1}'`

### Task 2b.2 — Skill Generator Tests

**Create** `tests/openclaw/` directory with:

**`tests/fixtures/openclaw/test-agent.md`** — realistic test fixture:
```yaml
---
name: test-reviewer
description: Test code reviewer for integration testing
tools: ["Read", "Grep", "Glob", "Bash"]
model: claude-sonnet-4-6
version: "1.0.0"
maturity: stable
---

# Test Reviewer

You are a test agent for integration testing of the OpenClaw bridge.
Review code for quality and suggest improvements.
```

**`tests/openclaw/test-skill-generator.sh`**:
1. Create temp dir for output
2. Run `convergio-openclaw-skills.sh --output-dir $TMPDIR` against test fixture
3. Verify output `SKILL.md` exists
4. Verify YAML frontmatter contains `name: test-reviewer`
5. Verify `metadata.openclaw.requires.env` contains `CONVERGIO_AUTH_TOKEN`
6. Verify curl command in body contains `/api/openclaw/invoke`
7. Verify `index.json` exists and is valid JSON
8. Cleanup temp dir

**`tests/openclaw/test-skill-format.sh`**:
1. Run generator against full agent catalog
2. For each generated SKILL.md, verify:
   - `name` is lowercase with hyphens only (regex: `^[a-z][a-z0-9-]*$`)
   - `version` is valid semver (regex: `^[0-9]+\.[0-9]+\.[0-9]+$`)
   - `metadata.openclaw` section exists
   - `description` is non-empty
3. Report pass/fail count

All tests: `set -euo pipefail`. Real file I/O, no mocks.

**Verify**: `bash tests/openclaw/test-skill-generator.sh` (must pass)

---

## Phase 3: Orchestration Layer

> Depends on Phase 2a and 2b being complete.

### Task 3.1 — Orchestrator Module

**Create** `integrations/openclaw-bridge/src/orchestrator.ts` (max 250 lines):

`ConvergioOrchestrator` class:
- `constructor(client: ConvergioDaemonClient, config: OpenClawConfig)`
- `async routeMessage(message: string, sessionContext?: Record<string, string>): Promise<InvokeResponse>` — determines routing:
  - If message clearly maps to single agent (keywords: "review" → rex, "security" → luca, "deploy" → marco), route directly
  - If ambiguous or multi-domain → route to `ali-orchestrator`
  - Use `config.direct_routing_threshold` to decide confidence cutoff
- `private detectAgent(message: string): {agent: string; confidence: number} | null` — keyword matching against known agent catalog (fetched via `client.listAgents()`)
- `async handleMultiAgentResponse(responses: InvokeResponse[]): Promise<string>` — merge multiple agent responses into coherent reply

### Task 3.2 — Session Bridge Module

**Create** `integrations/openclaw-bridge/src/session-bridge.ts` (max 250 lines):

`SessionBridge` class:
- `constructor(client: ConvergioDaemonClient)`
- `async mapSession(openclawKey: string): Promise<string>` — calls `client.getSessionMapping()`, returns IPC channel name
- `async pushContext(openclawKey: string, message: string, role: 'user' | 'assistant'): Promise<void>` — stores conversation turn in Convergio IPC shared context via `POST /api/ipc/context` with key `openclaw:${openclawKey}:history`
- `async getHistory(openclawKey: string): Promise<Array<{role: string; content: string}>>` — retrieves conversation history from IPC shared context

### Task 3.3 — Wire Orchestrator into Plugin

**Update** `integrations/openclaw-bridge/src/index.ts`:

1. Import `ConvergioOrchestrator` and `SessionBridge`
2. In `register(api)`, create instances
3. Update `convergio-invoke` tool execute function to:
   - If `params.agent` specified → direct invoke
   - If no agent → use `orchestrator.routeMessage()` for smart routing
4. Add session context passing via `sessionBridge.pushContext()` after each invocation

**Verify**: `cd integrations/openclaw-bridge && npx tsc --noEmit`

### Task 3.4 — Lobster Workflow Templates

**Create** `integrations/openclaw-bridge/workflows/` with 3 files:

**`code-review.yaml`** (Lobster workflow):
```yaml
name: convergio-code-review
args:
  target:
    default: "."
steps:
  - id: invoke-reviewer
    run: >
      openclaw.invoke --tool convergio-invoke
      --args-json '{"agent":"rex-code-reviewer","message":"Review: ${target}"}'
  - id: format
    run: echo "$invoke_reviewer"
```

**`project-plan.yaml`** — invoke ali-orchestrator with approval gate
**`security-audit.yaml`** — invoke luca-security-expert

Each max 50 lines. Real Lobster syntax.

### Task 3.5 — OpenClaw Configuration

**Create** `config/openclaw.yaml`:
```yaml
enabled: true
gateway_url: "http://localhost:18789"
skill_output_dir: "integrations/openclaw-bridge/skills"
agent_allowlist: []  # empty = all agents
agent_denylist: []   # agents never exposed
privacy:
  sensitive_agents:
    - elena-legal-compliance-expert
    - dr-enzo-healthcare-compliance-manager
  require_auth: true
  audit_log: true
session:
  idle_timeout_minutes: 30
  max_history_turns: 50
orchestration:
  default_agent: ali-orchestrator
  direct_routing_threshold: 0.8
```

**Update** `daemon/src/server/api_openclaw.rs` to load this config at startup via `serde_yaml`. Add `OpenClawConfig` to `ServerState` in `state.rs`. Guard all `/api/openclaw/*` endpoints with `if !config.enabled { return 404 }`.

**Verify**: `cd daemon && cargo check`

---

## Phase 4: Testing & Documentation

### Task 4.0 — Rust Unit Tests

**Create** `daemon/src/server/api_openclaw_tests.rs` (or `#[cfg(test)] mod tests` in api_openclaw.rs):

Follow the existing companion test file pattern (see api_ideas tests).

Tests:
1. `test_agents_endpoint` — mock agent catalog dir with 2 test .md files, GET /api/openclaw/agents, verify JSON array with 2 entries, verify fields (name, description, model)
2. `test_agents_cache` — call twice within 60s, verify second call uses cache
3. `test_invoke_endpoint` — POST with valid {agent_id, message}, verify 200 + request_id
4. `test_invoke_missing_agent` — POST with empty agent_id, verify defaults to ali-orchestrator
5. `test_session_mapping` — GET /api/openclaw/session/test-key, verify channel returned, call again verify same channel
6. `test_auth_required` — set OPENCLAW_AUTH_TOKEN env, call without Bearer, verify 401. With token, verify 200.
7. `test_auth_open` — unset OPENCLAW_AUTH_TOKEN, verify endpoints accessible without token (dev mode)

Use axum::test helpers. Real DB (test fixture), no mocks on internals.

**Verify**: `cd daemon && cargo test api_openclaw` (all pass)

### Task 4.1 — Integration Tests

**Create** `tests/openclaw/test-daemon-api.sh`:
1. Build and start daemon in background
2. Wait for health check (`curl http://localhost:8420/api/health`)
3. Test GET `/api/openclaw/agents` — verify 200 + JSON array
4. Test POST `/api/openclaw/invoke` — verify 200/202 + request_id returned
5. Test GET `/api/openclaw/session/test-key` — verify 200 + channel name
6. Test POST `/api/openclaw/webhook` — verify 200
7. Test auth: if `OPENCLAW_AUTH_TOKEN` set, verify 401 without token
8. Kill daemon, cleanup

**Create** `tests/openclaw/test-plugin-build.sh`:
1. `cd integrations/openclaw-bridge && npm install && npm run build`
2. Verify `dist/index.js` exists
3. Verify `openclaw.plugin.json` is valid JSON
4. Verify no TypeScript errors

**Create** `tests/openclaw/test-e2e-flow.sh`:
1. Start daemon
2. Simulate message flow: webhook → invoke → response
3. Verify IPC channel created for session
4. Verify agent invoked
5. Cleanup

All tests: `set -euo pipefail`, real daemon, real HTTP, no mocks.

**Verify**: `bash tests/openclaw/test-plugin-build.sh` (must pass)

### Task 4.2 — Documentation

1. **Update `CHANGELOG.md`**: Add entry under new version:
   ```
   ## [vX.Y.Z] - 2026-03-XX
   ### Added
   - OpenClaw bridge plugin: Convergio agents accessible via 30+ messaging platforms
   - Daemon API: /api/openclaw/* endpoints for agent invocation and session mapping
   - Skill generator: auto-generates OpenClaw SKILL.md from agent catalog
   - Lobster workflows: code-review, project-plan, security-audit templates
   ```

2. **Update `TROUBLESHOOTING.md`**: Add section:
   ```
   ## Problem: OpenClaw plugin cannot connect to Convergio daemon
   **Symptom**: `convergio-invoke` tool returns "service offline"
   **Cause**: Daemon not running or wrong URL
   **Fix**: Start daemon (`./daemon/start.sh`), verify `curl http://localhost:8420/api/health`

   ## Problem: No agents listed in OpenClaw
   **Symptom**: `convergio-agents` returns empty list
   **Cause**: Agent catalog path not found or YAML parse errors
   **Fix**: Verify `claude-config/agents/` exists with .md files

   ## Problem: SKILL.md generation fails
   **Symptom**: convergio-openclaw-skills.sh exits with error
   **Cause**: Agent .md files with malformed YAML frontmatter
   **Fix**: Run with `bash -x` to identify failing file, fix frontmatter
   ```

3. **Create** `integrations/openclaw-bridge/README.md`:
   - Quick start (3 steps: install deps, configure, start daemon)
   - Architecture diagram (text: user → OpenClaw → plugin → daemon → agent)
   - Configuration reference (openclaw.yaml all fields)
   - Available agents table (all 40+ with descriptions)
   - Lobster workflow examples
   - Troubleshooting section

4. **Update `VERSION.md`**: minor version bump

5. **Update `README.md`** (repo root) — add OpenClaw section:
   - What it is (bridge to 30+ messaging platforms)
   - Link to `integrations/openclaw-bridge/README.md`
   - Quick setup commands

6. **Update `CLAUDE.md`** — add to Commands table:
   - `convergio-openclaw-skills.sh` (generate SKILL.md files)
   - Add to Architecture table: `Integrations | integrations/ | TS | OpenClaw bridge plugin`
   - Add to Key Paths: `config/openclaw.yaml | OpenClaw bridge config`

7. **Update `.github/agents/Convergio.agent.md`**:
   - Add `api_openclaw` to API modules list (now 26 modules)
   - Add `/api/openclaw/*` endpoints to New Endpoint table:
     - `/api/openclaw/agents` GET — List agents for OpenClaw
     - `/api/openclaw/invoke` POST — Invoke agent with message
     - `/api/openclaw/session/:key` GET — Map session to IPC channel
     - `/api/openclaw/webhook` POST — Receive gateway events
   - Add OpenClaw troubleshooting entries

### Task 4.3 — Final Verification

Run this checklist:
1. `cd daemon && cargo check` — 0 errors
2. `cd daemon && cargo test` — all pass
3. `cd integrations/openclaw-bridge && npx tsc --noEmit` — 0 errors
4. `bash tests/openclaw/test-skill-generator.sh` — pass
5. `bash tests/openclaw/test-plugin-build.sh` — pass
6. `bash scripts/platform/convergio-openclaw-skills.sh --output-dir integrations/openclaw-bridge/skills` — generates SKILL.md files
7. Start daemon → `curl http://localhost:8420/api/openclaw/agents` — 200 + non-empty JSON
8. All files max 250 lines: `find integrations/openclaw-bridge/src -name '*.ts' -exec sh -c 'test $(wc -l < "$1") -le 250' _ {} \; -print`

---

## Dependency Graph

```
Phase 1 (docs)
  ├── Task 1.1 (ADR)
  └── Task 1.2 (plugin spec)
       │
       ├──────────────────────┐
Phase 2a (plugin)        Phase 2b (skills)
  ├── Task 2a.1 (setup)    ├── Task 2b.1 (generator)
  ├── Task 2a.2 (client)   └── Task 2b.2 (tests)
  ├── Task 2a.3 (entry)
  └── Task 2a.4 (rust API)
       │                      │
       └──────────┬───────────┘
                  │
Phase 3 (orchestration)
  ├── Task 3.1 (orchestrator)
  ├── Task 3.2 (session bridge)
  ├── Task 3.3 (wire to plugin)
  ├── Task 3.4 (lobster workflows)
  └── Task 3.5 (config + state wiring)
                  │
Phase 4 (closure)
  ├── Task 4.1 (integration tests)
  ├── Task 4.2 (docs)
  └── Task 4.3 (final verification)
```

## New Files Created (complete list)

| # | Path | Language | Purpose |
|---|---|---|---|
| 1 | `docs/adr/adr-openclaw-bridge.md` | Markdown | Architecture decision |
| 2 | `docs/design/openclaw-plugin-spec.md` | Markdown | Plugin design spec |
| 3 | `integrations/openclaw-bridge/package.json` | JSON | Node package |
| 4 | `integrations/openclaw-bridge/tsconfig.json` | JSON | TypeScript config |
| 5 | `integrations/openclaw-bridge/openclaw.plugin.json` | JSON | OpenClaw manifest |
| 6 | `integrations/openclaw-bridge/src/types.ts` | TypeScript | Shared interfaces |
| 7 | `integrations/openclaw-bridge/src/client.ts` | TypeScript | Daemon HTTP client |
| 8 | `integrations/openclaw-bridge/src/index.ts` | TypeScript | Plugin entry point |
| 9 | `integrations/openclaw-bridge/src/orchestrator.ts` | TypeScript | Ali routing logic |
| 10 | `integrations/openclaw-bridge/src/session-bridge.ts` | TypeScript | Session ↔ IPC mapping |
| 11 | `integrations/openclaw-bridge/workflows/code-review.yaml` | YAML | Lobster workflow |
| 12 | `integrations/openclaw-bridge/workflows/project-plan.yaml` | YAML | Lobster workflow |
| 13 | `integrations/openclaw-bridge/workflows/security-audit.yaml` | YAML | Lobster workflow |
| 14 | `integrations/openclaw-bridge/README.md` | Markdown | Plugin docs |
| 15 | `daemon/src/server/api_openclaw.rs` | Rust | 4 API endpoints |
| 15b | `daemon/src/server/api_openclaw_tests.rs` | Rust | Unit tests (7 test cases) |
| 16 | `config/openclaw.yaml` | YAML | Bridge configuration |
| 17 | `scripts/platform/convergio-openclaw-skills.sh` | Bash | Skill generator |
| 18 | `tests/fixtures/openclaw/test-agent.md` | Markdown | Test fixture |
| 19 | `tests/openclaw/test-skill-generator.sh` | Bash | Generator tests |
| 20 | `tests/openclaw/test-skill-format.sh` | Bash | Format validation |
| 21 | `tests/openclaw/test-daemon-api.sh` | Bash | API integration tests |
| 22 | `tests/openclaw/test-plugin-build.sh` | Bash | Plugin build test |
| 23 | `tests/openclaw/test-e2e-flow.sh` | Bash | E2E flow test |

## Existing Files Modified

| # | Path | Change |
|---|---|---|
| 1 | `daemon/src/server/mod.rs` | Add `pub mod api_openclaw;` |
| 2 | `daemon/src/server/routes/mod.rs` | Add `use super::api_openclaw;` + `.merge(api_openclaw::router())` |
| 3 | `daemon/src/server/state.rs` | Add `OpenClawConfig` to `ServerState` |
| 4 | `CHANGELOG.md` | Add OpenClaw bridge entry |
| 5 | `TROUBLESHOOTING.md` | Add OpenClaw troubleshooting section |
| 6 | `VERSION.md` | Minor version bump |
| 7 | `README.md` | Add OpenClaw bridge section with quick setup |
| 8 | `CLAUDE.md` | Add OpenClaw commands, architecture, key paths |
| 9 | `.github/agents/Convergio.agent.md` | Add api_openclaw module + 4 endpoints + troubleshooting |

## Conventions

- Max 250 lines per file — split if exceeded
- English only in code/comments/docs
- Rust: `cargo fmt` + `cargo clippy`
- TypeScript: strict, no `any`, named exports, semicolons, single quotes
- Bash: `set -euo pipefail`, quote vars, `local`, `trap cleanup EXIT`
- Comments: WHY not WHAT, <5% density
- No secrets in code — env vars only
- Test domains: `example.com`/`example.org` (RFC 2606)
