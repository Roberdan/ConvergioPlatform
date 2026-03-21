# ADR 0104: Agent Push Notifications via Hooks

Status: Accepted | Date: 21 Mar 2026

## Context

Agents running in Claude Code/Copilot cannot receive messages in real-time — they must poll. MCP servers would solve this but add deployment complexity (npm install per agent, config per project). Need a zero-config solution.

## Decision

Use Claude Code's **Notification hook** mechanism:
- Hook runs after each interaction, checks daemon API for unread messages
- Agent name set via `CONVERGIO_AGENT_NAME` env var (set by `convergio` CLI at launch)
- Zero dependencies: just curl + daemon API
- Works with any tool that supports hooks

Rejected alternatives:
- MCP server: heavy (npm install, config, process management)
- WebSocket client in agent: Claude Code can't maintain persistent connections
- File-based polling: fragile, no cross-machine support

## Consequences

- Positive: zero-config push notifications, works cross-machine via daemon API
- Negative: notifications appear after tool use (not truly instant), requires daemon running
