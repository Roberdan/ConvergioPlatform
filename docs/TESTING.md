# Testing -- Ali Orchestrator & Plan Management

## Test Suites Overview

| Suite | Type | Count | Command | Description |
|---|---|---|---|---|
| Cargo unit tests | Unit | 1386 passed | `cd daemon && cargo test` | Full daemon test suite covering all 107 modules |
| Orchestrator unit tests | Unit | 16 passed | `cargo test orchestrator` | Coordinator, wave lifecycle, task dispatch, event handling |
| Orchestrator E2E stress | E2E | 24/25 passed | `bash scripts/test-orchestrator-e2e.sh` | 19 scenarios + 6 stress/validation checks |
| Plan management API | Integration | 21 endpoints | Inline curl against `:8420` | Full plan DB, IPC, mesh, checkpoint API coverage |
| Mesh rsync | Manual + script | Verified | Real M5Max to M1Pro transfers | Rsync push/pull, SSH exec, concurrent access |

## Running Tests

```bash
# Full daemon unit tests (all 107 modules)
cd daemon && cargo test

# Orchestrator-specific unit tests
cd daemon && cargo test orchestrator

# Orchestrator E2E stress suite (requires running daemon)
bash scripts/test-orchestrator-e2e.sh

# Plan management API (requires running daemon on :8420)
# Run individual curl commands per endpoint (see Plan Management API Coverage)

# Type check only (~5s)
cd daemon && cargo check
```

## Orchestrator E2E Scenarios

All scenarios from `scripts/test-orchestrator-e2e.sh`:

| ID | Scenario | Verifies |
|---|---|---|
| T1 | Plan creation via API | `POST /api/plan-db/create` returns valid plan ID |
| T2 | Wave start event | Starting a wave emits `wave_started` on coordinator channel |
| T3 | Wave needs validation | Wave completion triggers `wave_needs_validation` event |
| T4 | Task assignment | Tasks dispatched to correct agent via IPC |
| T5 | Task completion | `cvg task update <id> done` transitions state correctly |
| T6 | Checkpoint save | `POST /api/plan-db/checkpoint/save` persists state |
| T7 | Checkpoint restore | `POST /api/plan-db/checkpoint/restore` recovers state |
| T8 | Agent registration | `cvg agent start` registers agent in active roster |
| T9 | Agent heartbeat | Registered agents emit periodic heartbeats |
| T10 | Parallel task dispatch | Multiple independent tasks dispatched in single message |
| T11 | Task blocking | Blocked task halts wave progress, emits `need_human` |
| T12 | Wave merge trigger | All tasks done triggers `wave_ready_to_merge` |
| T13 | Execution tree | `/api/plan-db/execution-tree/{id}` returns full DAG |
| T14 | Plan readiness | `/api/plan-db/readiness/{id}` checks preconditions |
| T15 | Drift detection | `/api/plan-db/drift-check/{id}` detects plan/code drift |
| T16 | Review registration | `/api/plan-db/review/register` stores review result |
| T17 | Review gate check | `/api/plan-db/review/check` enforces review before execute |
| T18 | IPC message routing | `/api/ipc/send` delivers typed messages between agents |
| T19 | Agent reaper | Stale agents reaped after timeout, resources released |

Additional stress/validation checks (T20-T25):

| ID | Check | Verifies |
|---|---|---|
| T20 | 200-event burst | Coordinator handles 200 rapid events without loss |
| T21 | Concurrent API reads | 20 simultaneous GET requests return consistent data |
| T22 | Plan cancel mid-wave | Cancellation propagates to all active tasks |
| T23 | Wave retry after failure | Failed wave can be retried with clean state |
| T24 | Route count contract | API exposes expected number of routes (88) |
| T25 | Static serve isolation | Static file serving does not interfere with API routes |

## Plan Management API Coverage

| Endpoint | Method | Purpose | Verified |
|---|---|---|---|
| `/api/plan-db/list` | GET | List all plans | Yes |
| `/api/plan-db/json/{id}` | GET | Full plan JSON with waves/tasks | Yes |
| `/api/plan-db/start/{id}` | POST | Transition plan to `doing` | Yes |
| `/api/plan-db/task/update` | POST | Update task state and summary | Yes |
| `/api/plan-db/execution-tree/{id}` | GET | DAG of plan execution | Yes |
| `/api/plan-db/readiness/{id}` | GET | Pre-execution readiness check | Yes |
| `/api/plan-db/drift-check/{id}` | GET | Detect plan vs codebase drift | Yes |
| `/api/plan-db/checkpoint/save` | POST | Persist execution state | Yes |
| `/api/plan-db/checkpoint/restore` | POST | Recover from compaction/crash | Yes |
| `/api/plan-db/review/register` | POST | Store plan review result | Yes |
| `/api/plan-db/review/check` | GET | Enforce review gate | Yes |
| `/api/ipc/send` | POST | Route IPC message between agents | Yes |
| `/api/ipc/agents` | GET | List registered agents | Yes |
| `/api/mesh` | GET | Mesh topology and peer list | Yes |
| `/api/mesh/status` | GET | Mesh health and connectivity | Yes |
| `/api/mesh/exec` | POST | Execute command on remote peer | Yes |
| `/api/agents` | GET | Agent roster with heartbeats | Yes |
| `/api/health` | GET | Daemon health check | Yes |
| `/api/plan-db/create` | POST | Create new plan | Yes |
| `/api/plan-db/cancel/{id}` | POST | Cancel plan with reason | Yes |
| `/api/plan-db/import` | POST | Import plan from YAML spec | Yes |

## Mesh Infrastructure Tests

| Test | Verified | Performance |
|---|---|---|
| Rsync push (M5Max to M1Pro) | Yes | 464MB in 19s (25 MB/s) |
| Rsync pull (M1Pro to M5Max) | Yes | Comparable throughput |
| SSH remote exec | Yes | Command execution on peer via `ssh2` |
| Concurrent access | Yes | 20 simultaneous operations, no corruption |
| Heartbeat monitoring | Yes | Sub-second detection of peer state changes |
| HMAC-SHA256 auth | Yes | Mutual authentication on every sync |

## Known Issues

| Issue | Details | Status |
|---|---|---|
| T3 false negative | `wave_needs_validation` event confirmed in daemon logs but test misses it. Channel accumulates thousands of messages across runs; test searches only last 500. | Known, non-blocking |
| Route count contract | Updated from 87 to 88 after `static_serve` extraction into dedicated module. | Resolved |

## Performance Benchmarks

| Metric | Result | Conditions |
|---|---|---|
| Event latency (coordinator) | ~3s | Single event, daemon under normal load |
| Rsync throughput | 25 MB/s (464MB/19s) | M5Max to M1Pro over Tailscale |
| Stress burst survival | 200 events | No event loss, no crashes |
| Concurrent API reads | 20 simultaneous | Consistent responses, no timeouts |
| Checkpoint save/restore | < 1s | Full plan state with 4 waves |
| Agent registration | < 100ms | Start to visible in roster |

## Sync Model Verification

| Layer | Mechanism | Verified |
|---|---|---|
| Working files | rsync (NOT git) | Yes |
| DB state | CRDT sync via daemon | Yes |
| Finished commits | git push to origin only | Yes |
| Peer-to-peer git | Blocked (no git push/pull between peers) | Yes |

The sync model enforces a strict separation: rsync handles file transfer between
peers, CRDT handles database convergence, and git is reserved exclusively for
committing finished work to origin. Direct git operations between peers are
blocked by design to prevent merge conflicts and history divergence.
