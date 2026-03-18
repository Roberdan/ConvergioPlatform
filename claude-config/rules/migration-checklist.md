# Migration Checklist

For ANY infra change: migrations, services, schema, mesh topology.

## Impact (MANDATORY)

Mesh nodes | Legacy scripts | DB schema (init-db.sql + remote) | Sync pipeline | Frontend contract | Daemon lifecycle | **IF IN DOUBT → ASK USER**

**Pre**: Map endpoints+responses | init-db.sql + init-db-migrate.sql | E2E real server | Inventory scripts
**During**: curl vs JS per endpoint | Playwright real server | Production DB | Verify remote nodes SSH
**Post**: Playwright audit (0 errors) | `mesh-sync.sh` ALL nodes | `mesh-health.sh` migrations+health | Restart daemons | Archive obsolete | `mesh-preflight.sh` (0 issues) | KB entries

## New Node

`mesh-provision-node.sh <peer>` → (1) `mesh-auth-sync.sh --peer <name>` (2) Auth fail: Screen Sharing → `claude auth login` + `gh auth login` (3) `mesh-preflight.sh --peer <name>` = 0
