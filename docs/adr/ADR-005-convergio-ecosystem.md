# ADR-005: Convergio Ecosystem

**Status**: Accepted
**Date**: 22 Marzo 2026
**Plan**: Plan C (#692)

## Context

Plans A (Core Intelligence) and B (Platform) established the daemon as single source of truth. Plan C extends the ecosystem with plugin management, domain-aware activation, automatic CRDT sync, and community setup.

## Decisions

### Domain-Aware Tool Activation
MCP was explicitly excluded (21.4k token overhead). Instead, `/solve` detects problem domain via keyword matching and suggests skill activation via `cvg skill enable`. Domain-skill mappings are configurable via `cvg domain map`. HITL confirmation required before activation.

### CRDT Background Sync
Daemon runs automatic peer sync on startup via `tokio::spawn` loop (default 30s). Peers marked unreachable after 3 consecutive failures, auto-recovered on success. HTTP endpoints expose sync status for dashboard monitoring. `mesh-sync.sh` retained for git config sync only.

### MyConvergio Setup Architecture
`copilot-sync.sh` replaced by `setup.sh` with provider auto-detection (Claude Code, Copilot CLI, generic LLM). Daemon-first with file-based transpiler fallback. Install manifest enables `--rollback`. Community governance via GitHub Action skill-lint CI.

## Consequences

- Plugin activation is daemon-driven, not MCP-dependent
- CRDT sync is transparent — no manual commands needed
- MyConvergio setup works with or without daemon (community-friendly)
- Sequential wave execution enforced after parallel wave issues in this plan
