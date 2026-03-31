# ADR: Agent Network as a Network-of-Companies Runtime

- Status: Accepted
- Date: 2026-03-31
- Scope: Daemon org model, IPC bus, CLI/TUI/dashboard integration

## Context

Convergio needed a single runtime model that supports:
- many agents grouped by business orgs/departments;
- intra-org and inter-org message flows with isolation;
- CEO-driven bootstrap and dynamic agent factory behavior;
- observability across CLI, TUI, websocket dashboards, and APIs.

Point features existed, but without one architectural decision tying org registry, messaging, telemetry, and visual surfaces together.

## Decision

Adopt a **network-of-companies** architecture:

1. **Org registry as source of truth**  
   `ipc_orgs`, `ipc_org_members`, `ipc_org_services`, `ipc_decisions`, `ipc_org_telemetry`, `ipc_org_digests`.

2. **Channel isolation contract**  
   - Intra-org: `org:<org_id>`
   - Inter-org: `inter-org:<src_org>:<dst_org>`
   - Guardrails enforce membership/budget constraints.

3. **CEO bootstrap + agent factory path**  
   CEOs create/shape org topology and services; agent creation is auditable via decision logs.

4. **Unified event surfaces**  
   WS brain events (`org_update`, `org_message`, `org_topology`, `agent_factory`) plus SSE bus stream for direct operator visibility.

5. **Operator workflows in CLI/TUI/dashboard**  
   `cvg org ...`, `cvg bus ...`, org-aware TUI widgets, and animated dashboard pages all consume the same runtime model.

## Consequences

### Positive
- Shared mental model across backend, orchestration, and UI layers.
- Stronger auditability (decisions + telemetry + message history).
- Better extensibility for Jarvis routing and multi-org autonomy features.

### Tradeoffs
- More schema and event coupling than flat-agent mode.
- Budget/isolation constraints require additional integration testing and operational runbooks.

## Validation

- End-to-end integration test added for full org flow (creation, bootstrap actions, messaging, telemetry, SSE route, budget gate).
- CLI org chart command (`cvg bus org`) added as operator-facing confirmation of hierarchy state.
