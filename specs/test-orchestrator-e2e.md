# Ali Orchestrator E2E Test Plan

> 24 Marzo 2026 | Validates rsync-based delegation, event-driven orchestration, reaper cleanup.

## Prerequisites

| Requirement | Command |
|---|---|
| Daemon running | `./daemon/start.sh` (port 8420) |
| Mesh peer online | `scripts/mesh/mesh-heartbeat.sh` |
| Plan 719 in DB | `cvg plan show 719` (status: todo) |
| Plan 712 depends on 719 | `cvg plan show 712` (depends_on: 719) |

## Test Scenarios

### T1 — Plan Start and Rsync Delegation

**Steps:**
1. `cvg plan start 719`
2. Observe Ali receives `plan_started` on `#orchestration`
3. Ali checks `dependencies_met(719)` — should return true (no deps)
4. Ali finds peer via `/api/mesh/status`
5. Ali runs `rsync_files` coordinator → peer
6. Ali writes prompt + done script on peer
7. Ali launches claude in tmux on peer

**Verify:**
- `convergio-bus.sh read ali-orchestrator --channel '#orchestration'` shows `plan_delegated`
- Peer has repo files (NOT via git): `ssh peer 'ls ~/GitHub/ConvergioPlatform/CLAUDE.md'`
- Tmux session exists: `ssh peer 'tmux has-session -t plan-719'`
- Done script uses rsync, NOT git: `ssh peer 'grep -c mesh-rsync /tmp/convergio-plan-719-done.sh'`

### T2 — Task Done and Wave Completion

**Steps:**
1. Simulate task completion:
   ```bash
   convergio-bus.sh send executor-peer '#orchestration' \
     '{"type":"task_done","task_id":"T1-01","plan_id":719}' event
   ```
2. Repeat for all tasks in wave 1

**Verify:**
- Ali emits `wave_done` when last task completes
- Ali emits `wave_needs_validation` (auto-validates since Thor is not a service yet)
- Ali emits `wave_validated` → `wave_ready` for next wave OR `plan_done` if last wave

### T3 — Plan Done and Dependency Unblocking

**Steps:**
1. Simulate plan completion:
   ```bash
   convergio-bus.sh send executor-peer '#orchestration' \
     '{"type":"plan_done","plan_id":719}' event
   ```

**Verify:**
- Ali calls `master_rollup` on parent plan 711
- Ali finds plan 712 (depends_on: 719) is now unblocked
- Ali emits `plan_ready` for plan 712
- Ali attempts delegation of plan 712 to available peer

### T4 — Delegation Failed and Retry

**Steps:**
1. Send delegation failure:
   ```bash
   convergio-bus.sh send mesh-agent '#orchestration' \
     '{"type":"delegation_failed","plan_id":720,"peer":"offline-peer","reason":"connection refused"}' event
   ```

**Verify:**
- Ali logs warning about failed delegation
- Ali calls `find_available_peer` excluding "offline-peer"
- If alternative peer exists: delegates to it
- If no peers available: emits `need_human`

### T5 — Peer Offline (need_human)

**Steps:**
1. Ensure no mesh peers are online (stop mesh daemon on peer)
2. `cvg plan start 720`

**Verify:**
- Ali emits `need_human` with reason "no online peers available"
- Notification API called: `curl http://localhost:8420/api/notify/list`
- TUI shows notification (if running)

### T6 — Rsync Sync-Back After Completion

**Steps:**
1. On peer, create a test file in the repo: `ssh peer 'touch ~/GitHub/ConvergioPlatform/test-rsync-back.txt'`
2. On peer, run the done script: `ssh peer 'bash /tmp/convergio-plan-719-done.sh'`

**Verify:**
- `test-rsync-back.txt` exists on coordinator (rsync sync-back worked)
- Ali receives `plan_done` event
- No git push/pull in the done script output

### T7 — Reaper Cleans Temp Files

**Steps:**
1. Create stale temp files on peer:
   ```bash
   ssh peer 'touch -t 202603220000 /tmp/convergio-plan-999.md /tmp/convergio-plan-999-done.sh'
   ```
2. Wait for reaper cycle (5 min) or trigger manually

**Verify:**
- Stale files removed: `ssh peer 'ls /tmp/convergio-plan-999*'` (should fail)
- Recent files NOT removed (active plans preserved)

### T8 — Reaper Cleans Stale Agents and Delegations

**Steps:**
1. Insert stale agent: via DB API or wait 30+ min with no heartbeat
2. Insert stale delegation: plan with `execution_host` set, `updated_at` > 24h ago

**Verify:**
- `cvg who agents` no longer shows stale agent
- Plan has `execution_host = NULL` (eligible for re-delegation)

## Edge Cases

### E1 — Peer Dies Mid-Task

**Scenario:** Peer goes offline while claude is running.
**Expected:** Delegation times out → reaper clears stale delegation after 24h → plan becomes re-delegatable.
**Manual intervention:** `cvg plan start <plan_id>` to re-trigger.

### E2 — Database Locked

**Scenario:** Multiple concurrent DB writes (Ali + HTTP API + reaper).
**Expected:** SQLite WAL handles concurrent reads. Writes retry via busy_timeout. No data loss.
**Verify:** `PRAGMA journal_mode` returns `wal`.

### E3 — Double Event

**Scenario:** Same `plan_started` event arrives twice.
**Expected:** Second delegation attempt finds plan already `doing` with `execution_host` set. Should either skip (idempotent) or delegate to same peer (no-op rsync).
**Risk:** Could launch two tmux sessions. Mitigation: tmux `kill-session` before `new-session`.

### E4 — Malformed Event

**Scenario:** Event with missing `plan_id` or invalid JSON.
**Expected:** `require_i64` returns error → reactor logs error → emits error event → continues processing.
**Verify:** `convergio-bus.sh send test '#orchestration' 'not-json' event` — Ali logs error, does not crash.

### E5 — Rsync Fails (Network Issue)

**Scenario:** rsync to peer fails (SSH timeout, disk full).
**Expected:** `rsync_files` returns error → `delegate_to_peer` returns error → handler emits `delegation_failed`.
**Verify:** Ali retries on different peer or escalates to `need_human`.

### E6 — Coordinator Restarts Mid-Plan

**Scenario:** Daemon restarts while a plan is running on peer.
**Expected:** Ali re-registers on startup. Peer continues running (tmux persists). Done script calls back to coordinator via HTTP (will succeed once daemon is back).
**Risk:** Events emitted while daemon was down are lost. Mitigation: peer retries IPC callback.

## Performance Metrics

| Metric | Target | How to Measure |
|---|---|---|
| Event → reaction latency | < 100ms | Timestamp in Ali log vs event timestamp |
| Rsync coordinator → peer | < 30s for full repo | `time mesh-rsync.sh` |
| Rsync peer → coordinator | < 30s for delta | `time mesh-rsync.sh` (after changes) |
| Reaper cycle overhead | < 1s local, < 5s remote | Reaper log timestamps |
| Memory (Ali idle) | < 1MB | `ps aux | grep convergio` |
| IPC message throughput | > 100 msg/s | Benchmark `receive_wait` batch size |

## Sync Model Validation

| What | Mechanism | Verify |
|---|---|---|
| Working files | rsync (mesh-rsync.sh) | `grep -r 'git pull\|git push' daemon/src/orchestrator/` returns nothing |
| DB state | CRDT sync (crsqlite) | `cvg plan show` consistent on both nodes |
| Finished commits | git push to origin (manual) | Only after Thor validation + PR merge |
| Temp files | Reaper cleanup on peers | No stale `/tmp/convergio-plan-*` after 24h |
