# Migration Symlinks

During transition from ~/.claude to ConvergioPlatform, these symlinks ensure backward compatibility.

## Active symlinks (to be created by operator on each machine)

```bash
# Dashboard: old path → new location
ln -sf ~/GitHub/ConvergioPlatform/dashboard ~/.claude/scripts/dashboard_web_new

# Daemon: old path → new location
ln -sf ~/GitHub/ConvergioPlatform/daemon ~/.claude/rust/claude-core-new

# Scripts: mesh ops
ln -sf ~/GitHub/ConvergioPlatform/scripts/mesh ~/.claude/scripts/mesh-platform
```

## Path references to update in .claude scripts

Any script referencing these paths needs updating:
- `~/.claude/scripts/dashboard_web/` → `~/GitHub/ConvergioPlatform/dashboard/`
- `~/.claude/rust/claude-core/` → `~/GitHub/ConvergioPlatform/daemon/`

## DB access

`dashboard.db` stays at `~/.claude/data/dashboard.db`. The dashboard server reads `DASHBOARD_DB` env var
or falls back to this path. No change needed if env var is set or default path is used.
