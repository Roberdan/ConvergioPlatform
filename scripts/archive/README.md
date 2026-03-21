# scripts/archive

Dead code archived during Plan 677 Wave W5 cleanup (2026-03-21).

Scripts here had zero callers in `scripts/` and are superseded by daemon API modules
or by the canonical copies in `claude-config/scripts/` (on `$PATH` via `~/.claude/scripts`).

## Archived

| Script | Reason | Superseded by |
|---|---|---|
| `mesh-env-setup.sh` | No callers in `scripts/`. Duplicate of `claude-config/scripts/mesh-env-setup.sh` which is the active copy used by mesh-networking.md "Add Peer" procedure. Daemon `mesh/env/` modules handle remote env setup. | `claude-config/scripts/mesh-env-setup.sh` |
| `mesh-normalize-hosts.sh` | No callers in any script. One-time migration for normalizing `plans.execution_host` and `tasks.executor_host`. Duplicate of `claude-config/scripts/mesh-normalize-hosts.sh`. Daemon `peers.rs` now owns canonical peer identity. | `claude-config/scripts/mesh-normalize-hosts.sh` |

## NOT Archived

| Script | Reason kept |
|---|---|
| `scripts/mesh/mesh-migrate-services.sh` | Actively called by `scripts/mesh/mesh-ubuntu-install.sh:142` via `exec "$SCRIPT_DIR/mesh-migrate-services.sh"` |
