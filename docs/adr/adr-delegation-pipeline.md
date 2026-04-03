# ADR: Delegation Pipeline Architecture

**Status**: Accepted
**Date**: 2026-07-14
**Authors**: Engineering Team

## Context

Convergio's mesh architecture connects multiple macOS/Linux nodes via Tailscale
for distributed plan execution. Before this ADR, delegation was a stub with:

- No automated pipeline (manual SSH + tmux launch)
- No progress visibility (CLI printed "Delegation started..." and returned)
- No peer validation (unknown peers silently fell back to raw strings)
- No alias support (each node had 3-5 different names, causing confusion)
- Zero traceability in the `delegation_progress` table

Plan 706 highlighted the need for a reliable, observable delegation pipeline.

## Decision

### Pipeline Stages

Delegation follows a linear pipeline with progress tracking at each stage:

```
  ┌──────────┐    ┌────────────┐    ┌──────────────┐    ┌───────────┐    ┌───────────┐
  │ Resolving │───▶│ Connecting │───▶│ Transferring │───▶│ Executing │───▶│ Completed │
  └──────────┘    └────────────┘    └──────────────┘    └───────────┘    └───────────┘
       │                │                  │                  │
       ▼                ▼                  ▼                  ▼
   [peer not       [SSH/HTTP          [worktree            [agent
    found →         timeout →          create               spawn
    400 error]      blocked]           failed →             failed →
                                       blocked]             blocked]
```

### Peer Resolution with Aliases

`PeerConfig` gains an `aliases: Vec<String>` field parsed from `peers.conf`:

```ini
[m1pro]
ssh_alias=robertos-mbp-m1.tail01f12c.ts.net
aliases=worker1,macmini,milan-worker
```

Resolution order in `find_peer()`:
1. Exact section name match
2. Case-insensitive section name match
3. Field match (tailscale_ip, ssh_alias, dns_name fuzzy)
4. Alias match (case-insensitive)

### Dispatch Validation

`POST /api/mesh/delegate` now returns **400 Bad Request** if the peer cannot
be resolved through `peer_resolver::resolve()`. Previously, unresolved peers
silently fell back to the raw string, causing SSH failures downstream.

### Progress Tracking

Each pipeline stage writes to the `delegation_progress` table:

| Stage | status | current_task |
|-------|--------|-------------|
| Start | running | resolving |
| SSH connected | running | connecting |
| Worktree created | running | transferring |
| Agent spawned | running | executing |
| Success | done | completed |
| Failure | blocked | failed |

The CLI polls `GET /api/delegation/by-plan/{plan_id}` every 2 seconds,
printing stage transitions with timestamps:

```
[14:32:01] step: resolving
[14:32:03] step: connecting
[14:32:08] step: transferring
[14:32:12] step: executing
[done]
```

A `--no-wait` flag preserves fire-and-forget behavior for scripts.

### Transport Security

All inter-node communication uses:
- **Tailscale WireGuard tunnels** for network-level encryption
- **HMAC-SHA256 signatures** for API request authentication
- **Bearer tokens** via `CONVERGIO_AUTH_TOKEN` for daemon API auth

### Remote Worktree Management

Each delegated plan gets a detached worktree on the target peer:
- Path: `/private/tmp/wt-plan-{plan_id}`
- Created via `git worktree add --detach` (never `-b`, per platform rules)
- Cleaned up on failure; preserved on success for inspection
- Plan data synced via daemon API export/import

## Consequences

### Positive

- **Visibility**: Every delegation stage is observable via CLI and API
- **Reliability**: Unknown peers caught at dispatch time, not mid-pipeline
- **Usability**: Aliases eliminate the "5 names for 2 machines" problem (B9)
- **Debuggability**: Failed delegations record the failure stage and message
- **Backward compatibility**: `--no-wait` preserves existing script behavior

### Negative

- **DB writes per delegation**: 5 upserts to delegation_progress (negligible)
- **Polling overhead**: CLI polls every 2s during active delegation
- **Migration**: Existing peers.conf files work unchanged (aliases defaults
  to empty); no schema migration needed for delegation_progress (already exists)

### Risks

- Progress tracking depends on DB availability; if DB is locked, progress
  is silently skipped (logged as warning, non-blocking)
- Alias collisions across peers are resolved first-match; no uniqueness check

## Related

- Plan 706: Original delegation stub
- Plan 720 T2-01: Delegation progress traceability
- ADR-0107: Daemon consolidation (mesh architecture)
- B6: SSH doesn't resolve peers from peers.conf
- B9: 5 names for 2 machines
