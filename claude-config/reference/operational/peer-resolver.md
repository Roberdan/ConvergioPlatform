# Peer Resolver

Single source of truth for peer addressing. Replaces ad-hoc hostname lookups across the codebase.

## Location

`daemon/src/mesh/peer_resolver.rs` — Rust module, no external dependencies beyond peers.conf.

## API

```rust
// High-level: resolve any peer name/alias
peer_resolver::resolve("m1pro") -> ResolvedPeer

// With explicit config path
peer_resolver::resolve_with_conf("m1pro", &path) -> ResolvedPeer

// From pre-loaded registry (no I/O)
peer_resolver::resolve_from_registry("m1pro", &registry) -> ResolvedPeer

// Normalize to canonical name
peer_resolver::canonicalize("m1pro", &path) -> String

// Build SSH destination
peer_resolver::ssh_destination(&resolved) -> String
```

## ResolvedPeer Structure

| Field | Type | Source |
|-------|------|--------|
| `canonical_name` | String | Section name in peers.conf |
| `host` | String | Best available host (see fallback chain) |
| `port` | u16 | Default: 22 |
| `user` | String | `user` field from peers.conf |
| `ssh_alias` | String | `ssh_alias` field |
| `tailscale_ip` | String | `tailscale_ip` field |

## Resolution Strategy (3-stage)

```
Input: "m1pro"
  │
  ├─ 1. Exact match on section name → [m1pro] found? → done
  │
  ├─ 2. Case-insensitive → "M1Pro" matches [m1pro]? → done
  │
  └─ 3. Fuzzy match across fields:
       ├─ tailscale_ip: exact match
       ├─ ssh_alias: normalized match (strip -_.' and lowercase)
       └─ dns_name: substring match on normalized form
```

### Normalization

`normalize_name()` lowercases and strips: `-`, `_`, ` `, `'`, `.`

Examples: `My-Peer` → `mypeer`, `m1_pro` → `m1pro`, `M1.Pro` → `m1pro`

## Host Fallback Chain

When building the connection `host`, resolver uses first non-empty:

1. `ssh_alias` (preferred — uses SSH config multiplexing)
2. `tailscale_ip` (direct P2P)
3. `dns_name` (mDNS/DNS)
4. `canonical_name` (last resort)

## SSH Destination

`ssh_destination()` returns:
- `ssh_alias` if non-empty (uses SSH config)
- `user@host` otherwise

## Configuration

`~/.claude/config/peers.conf` (INI format):

```ini
[mesh]
shared_secret = <hmac-secret>

[m5max]
ssh_alias = roberdandev-m5Max
user = Roberdan
os = macos
role = coordinator
tailscale_ip = 100.x.x.x
dns_name = m5max.tail12345.ts.net

[m1pro]
ssh_alias = roberdandev-m1Pro
user = Roberdan
os = macos
role = worker
tailscale_ip = 100.x.x.x
```

## Why peer_resolver Exists

Before peer_resolver (B6, B9 bugs): 5 different name formats for 2 machines, SSH couldn't resolve peers from peers.conf, each script had its own lookup logic.

After: single `resolve()` call handles any name format, fuzzy matching, and consistent fallback chain.

## Usage in Codebase

| Consumer | How |
|----------|-----|
| `mesh/delegate.rs` | `resolve()` for SSH target |
| `cli_delegation.rs` | `resolve()` for delegation commands |
| `mesh/coordinator.rs` | `resolve_from_registry()` for scoring |
| `mesh/sync.rs` | `ssh_destination()` for rsync targets |
