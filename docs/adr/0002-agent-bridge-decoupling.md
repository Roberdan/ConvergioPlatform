# ADR 0002: Agent Bridge & Platform Decoupling

Status: Accepted | Date: 19 Mar 2026 | Plan: 668

## Context

| Problem | Impact |
|---------|--------|
| Plans 633-635 built IPC daemon but no bridge to Claude Code/Copilot | Agents can't discover or communicate |
| Plan 664 consolidated repo but ~/.claude/ still tightly coupled | Manual setup, fragile symlinks |
| No agent registry | Can't track which agents are active |
| No skill pool integration | Can't route tasks to best agent |
| No Copilot instructions | Copilot works blind, no conventions |

## Decision

| Component | Implementation |
|-----------|---------------|
| Entry point | `./setup.sh` — single command setup |
| Config separation | ~/.claude/ is thin symlink layer → claude-config/ |
| Agent registration | SubagentStart hook → agent-bridge.sh → POST /api/ipc/agents/register |
| Agent lifecycle | register on start, heartbeat periodic, unregister on stop |
| Skill discovery | agent-skills-sync.sh reads commands/*.md → registers in IPC pool |
| Copilot alignment | .github/copilot-instructions.md (same source of truth) |
| Enforcement | .claude/settings.json with 17 hooks |
| Communication backbone | Daemon HTTP API (port 8420) |

## Alternatives Considered

| Alternative | Pro | Con | Rejected because |
|------------|-----|-----|------------------|
| File-based IPC (JSON files) | Simple, no daemon needed | No real-time, polling required, race conditions | Too slow for agent coordination |
| Direct agent-to-agent TCP | Low latency | Complex discovery, no central registry, firewall issues | Over-engineered for current scale |
| Shared SQLite without daemon | No HTTP overhead | No WebSocket broadcast, no real-time dashboard | Dashboard needs live updates |
| Environment variables only | Zero infrastructure | No persistence, no cross-session, no multi-agent | Can't track agent lifecycle |

## Consequences

### Positive

- Agents discoverable via GET /api/ipc/agents
- Skills routable to best agent via confidence scores
- Cross-platform: Claude Code and Copilot share same IPC
- Setup reduced to single command
- Dashboard gets live agent visibility via WebSocket

### Negative

- Daemon must be running for IPC (graceful degradation when not)
- HTTP overhead per registration/heartbeat (~3ms)
- curl dependency in all bridge scripts

## Enforcement

| Rule | Check command |
|------|--------------|
| Every agent MUST register via agent-bridge.sh | `curl localhost:8420/api/ipc/agents` |
| setup.sh MUST be run after clone | `test -L ~/.claude/CLAUDE.md` |
| .claude/settings.json MUST exist in project | `jq . .claude/settings.json` |
| Hooks MUST be >= 15 | `jq '[.hooks.PreToolUse[]] | length' .claude/settings.json` |
| Daemon MUST be accessible | `curl -s localhost:8420/api/ipc/status` |

## API Quick Reference

| Method | Path | Purpose |
|--------|------|---------|
| POST | /api/ipc/agents/register | Register agent |
| POST | /api/ipc/agents/unregister | Remove agent |
| POST | /api/ipc/agents/heartbeat | Update heartbeat |
| GET | /api/ipc/agents | List active agents |
| GET | /api/ipc/status | IPC system status |
| POST | /api/ipc/send | Send message |
| GET | /api/ipc/messages | Query messages |
| GET | /api/ipc/channels | List channels |
| GET | /api/ipc/context | Shared context |
| GET | /api/ipc/locks | File locks |
| GET | /api/ipc/worktrees | Worktree registry |
| GET | /api/ipc/conflicts | Detect conflicts |
| GET | /api/ipc/skills | Skill pool |
| GET | /api/ipc/models | Model registry |
| GET | /api/ipc/budget | Budget status |
| GET | /api/ipc/auth-status | Token health |
| GET | /api/ipc/route-history | Routing log |
| GET | /api/ipc/metrics | System metrics |
| GET | /api/ipc/logs | IPC logs |

## Scripts

| Script | Purpose |
|--------|---------|
| setup.sh | Single entry point installer |
| scripts/platform/agent-bridge.sh | Agent register/unregister/heartbeat/checkpoint |
| scripts/platform/copilot-bridge.sh | Copilot-specific wrapper |
| scripts/platform/agent-heartbeat.sh | Periodic heartbeat sender |
| scripts/platform/agent-skills-sync.sh | Sync skills from commands/ to IPC |
| scripts/platform/test-agent-ipc.sh | E2E agent IPC tests (TAP) |
| scripts/platform/test-setup-e2e.sh | E2E setup tests (TAP) |
