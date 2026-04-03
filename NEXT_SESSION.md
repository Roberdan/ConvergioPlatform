# Next Session — Convergio Platform

**Last updated**: 29 Marzo 2026
**Current version**: 19.2.0
**Platform readiness**: 7/10 (sync broken)

---

## CRITICAL: DB Replication Architecture Is Broken

**Two separate sync systems exist, NEITHER works:**

| System | Files | Problem |
|--------|-------|---------|
| CRDT/crsqlite (mesh socket) | `mesh/sync/`, `daemon_sync_db.rs`, `ops_apply.rs` | **Feature-gated, NOT in default build** (`--features kernel` excludes it) |
| HTTP/timestamp (background_sync) | `background_sync.rs`, `api_sync.rs`, `libsql_adapter.rs` | **4 bugs (see below), only 3 tables** |
| rsync file-copy (scripts) | `deploy-node.sh` | **Works but snapshot-only, not live replication** |

**Decision required**: Pick ONE replication model. The HTTP/timestamp path is the right one (simpler, no C extension, works with rusqlite). Kill the CRDT path or keep it gated forever. Stop relying on rsync except for bootstrap/repair.

### BUG 1 (SHOWSTOPPER): Double http:// prefix in sync URLs

`background_sync.rs:67` builds `"http://100.x.x.x:8420"` but `background_sync_http.rs:14,34` prepends another `"http://"` → all HTTP calls target host `"http"` → instant failure → silent skip.

**Fix**: `query_active_peers()` must return `"IP:PORT"` without `http://` scheme, OR `background_sync_http.rs` must not add scheme.

### BUG 2: Send failure does not block fetch → checkpoint advances → data loss

`background_sync.rs:112-116`: if `send_changes_to_peer` fails, code continues to fetch+apply remote changes and updates `_sync_meta` checkpoint. Local changes are never sent but marked as synced.

**Fix**: add `return 0;` after the `warn!` on send failure (line 115).

### BUG 3: Tailscale-only — no Thunderbolt or LAN fallback

`query_active_peers()` reads only `tailscale_ip` from peers.conf. If Tailscale is down, sync stops. The `peer_resolver.rs` has a fallback chain but `background_sync.rs` ignores it entirely.

### BUG 4: Peer name mismatch (DB vs peers.conf section names)

Mesh heartbeat writes `node_id` to `peer_heartbeats.peer_name`. If this doesn't exactly match peers.conf section name (e.g., `MacBook-Pro-di-Roberdan` vs `m5max`), cross-reference fails silently.

### BUG 5: Missing startup migrations for sync schema

The sync loop starts (`main_dispatch.rs:94-117`, `ipc_handler/server.rs:69-101`) without ensuring `_sync_meta` table and `updated_at` columns exist. If migrations haven't run, exports fail silently.

### BUG 6: Only 3 tables synced

`SYNC_TABLES = ["tasks", "plans", "waves"]` — but `knowledge_base`, `peer_heartbeats`, `mesh_events`, `notifications`, `delegation_log`, `chat_sessions`, `chat_messages` are NOT replicated. Telegram on M1Pro responds with stale data because it queries tables that never sync.

---

## Task 1: Fix Sync Bugs + Architecture (T1 — do FIRST)

### 1a. Fix the 4 code bugs

All in `daemon/src/background_sync.rs` and `background_sync_http.rs`:

1. **Remove `http://` from `query_active_peers`** (line 67): change to `format!("{}:8420", ip)`
2. **Add `return 0;` after send failure** (line 115, after the `warn!`). Change `warn!` to `error!`.
3. **Change "no active peers" log from `debug!` to `error!`** (line 202) — FAIL LOUD
4. **Verify peer name matching**: check what `node_id` the mesh protocol actually sends vs peers.conf section names. Add canonical name resolution if they differ.
5. **Ensure startup migrations**: verify `_sync_meta` table + `updated_at` columns exist BEFORE sync loop starts. If not, `error!` and don't start the loop.

### 1b. Expand SYNC_TABLES

Add ALL tables that Telegram/kernel queries to `SYNC_TABLES`:
```rust
const SYNC_TABLES: &[&str] = &[
    "tasks", "plans", "waves",
    "knowledge_base", "notifications", "delegation_log",
];
```
Each table needs `updated_at` column — add migrations if missing.

### 1c. Architectural decision: kill CRDT path

The crsqlite CRDT path is dead code (not in default build). Either:
- Remove it entirely from `mesh/sync/` (clean)
- Or keep it gated but document it as deprecated

HTTP/timestamp sync is the ONE replication model. Make this explicit in ADR.

**Verify**:
```bash
cargo check --features kernel --manifest-path daemon/Cargo.toml
cargo test --features kernel --manifest-path daemon/Cargo.toml --lib -- background_sync
```

Then restart BOTH daemons:
```bash
# On M5Max:
cd ~/GitHub/ConvergioPlatform && ./daemon/start.sh
# On M1Pro (via SSH):
ssh roberdandev-m1Pro "cd ~/GitHub/ConvergioPlatform && git pull --rebase && ./daemon/start.sh"
```

**Live test**:
```bash
# Create test plan on M5Max
curl -sf http://localhost:8420/api/plan-db/create -d '{"project_id":"convergio","name":"sync-test-'$(date +%s)'"}'
# Wait 60s
sleep 60
# Check M1Pro sees it
curl -sf http://100.106.173.118:8420/api/plan-db/list | jq '.[].name' | grep sync-test
```

---

## Task 2: Multi-Transport Peer Resolution (T2)

The mesh must auto-discover the fastest reachable path. Peer names are FIXED (`m5max`, `macProM1`), only the IP changes.

### Fallback chain (ordered by speed):
1. **Thunderbolt** (10.0.0.x) — direct Mac-to-Mac, sub-ms latency
2. **LAN** (192.168.x.x or .local) — same network
3. **Tailscale** (100.x.x.x) — works from anywhere, higher latency

### Implementation:

**Step 1**: Add `thunderbolt_ip` to `PeerConfig` struct (`daemon/src/mesh/peers/types.rs`):
```rust
pub thunderbolt_ip: Option<String>,
```

**Step 2**: Parse it in `daemon/src/mesh/peers/parser.rs`

**Step 3**: Refactor `query_active_peers` in `background_sync.rs` to use the peer resolver with transport probing:
```
For each online peer:
  1. Try thunderbolt_ip:8420 (TCP connect, 2s timeout)
  2. Try LAN via ssh_alias.local:8420 (2s timeout)
  3. Try tailscale_ip:8420 (2s timeout)
  4. Use first reachable, cache result for 5 minutes
  5. If NONE reachable: error!() + Telegram alert. Do NOT silently skip.
```

**CRITICAL RULE**: NO silent failures. No `unwrap_or_default()`. No `continue` after `warn!()`.
Every error path must be visible. If sync fails, it must be LOUD — error log + Telegram.
Otherwise bugs stay hidden for weeks (like the double-http bug did).

**Step 4**: Add `lan_ip: Option<String>` to peers.conf and PeerConfig for explicit LAN addresses

**Verify**: connect Mac via Thunderbolt, disable Tailscale, confirm sync still works.

---

## Task 3: Worktree Auto-Cleanup (T3)

Worktree cleanup after task/wave completion does not work. Stale worktrees accumulate.

### Requirements:
- After task completes (status=done), if worktree exists and branch is merged, delete it
- After wave completes, cleanup all wave worktrees
- `git worktree prune` after deletions
- Safety: NEVER delete worktrees with uncommitted changes — log warning instead
- Run cleanup in: (a) Thor after validation, (b) `cvg wave merge`, (c) nightly job

### Check current state:
```bash
git worktree list
# Should show ONLY main. Any others are stale.
```

### Implementation locations:
- `daemon/src/workspace/` — workspace module handles worktree lifecycle
- `scripts/platform/session-reaper.sh` — may need worktree reaper logic
- Thor validation gates — add cleanup after wave merge

---

## Task 4: Live Verification (T4 — do LAST)

After all fixes:

```bash
# 1. Verify sync loop produces logs
grep 'background_sync' /tmp/convergio-daemon.log | tail -20

# 2. Create plan on M5Max, verify M1Pro sees it within 60s
curl -sf http://localhost:8420/api/plan-db/create -d '{"project_id":"convergio","name":"final-sync-test"}'
sleep 60
ssh roberdandev-m1Pro "curl -sf http://localhost:8420/api/plan-db/list | jq '.[].name' | grep final-sync-test"

# 3. Verify Telegram responds with fresh data
# Ask Jarvis via Telegram: "quanti piani ci sono?"
# Response must include the test plan just created

# 4. Run full E2E
bash scripts/test-e2e.sh --remote m1Pro  # must give 32/32
```

---

## Completed Plans

| Plan | Name | Status |
|------|------|--------|
| 742 | Plan X v2 Hardening | DONE |
| 745 | Jarvis Self-Healing Kernel | DONE |
| 746 | — | DONE |
| 749 | Mesh Ops (sync, deploy, E2E) | DONE (but sync not working live) |
| 739 | VirtualBPM | In progress (separate copilot) |
| 738 | convergio-web | Suspended |

## Current State

| Dimension | Status |
|-----------|--------|
| Daemon version | 19.2.0 |
| Sync model | HTTP background sync (BROKEN — double http://) |
| Peer resolver | Centralized, 3-stage fuzzy match (not used by sync) |
| Jarvis (M1 Pro) | Active — Qwen 7B, self-healing |
| Mesh connectivity | Tailscale only (needs Thunderbolt + LAN fallback) |
| Worktree cleanup | NOT WORKING — stale worktrees accumulate |

## Commands to Resume

```bash
# Verify daemon running
curl -sf http://localhost:8420/api/health

# Check kernel
cvg kernel status

# Check sync logs (look for "background_sync" entries)
grep -i 'background_sync\|sync.*peer' /tmp/convergio-daemon.log | tail -30

# Check peer heartbeats
curl -sf http://localhost:8420/api/heartbeat | jq .

# Build after fixes
cd daemon && cargo check --features kernel
```
