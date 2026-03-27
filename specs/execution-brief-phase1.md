# Execution Brief — Phase 1 Parallel Launch

**Date**: 25 Marzo 2026
**Coordinator**: Claude Opus 1M (this session)
**Executor**: Copilot Opus / Claude Codex
**Status**: 3 plans launched in parallel

## Active Plans

| Plan | DB ID | Status | W1 Tasks | Working Dir |
|---|---|---|---|---|
| **O — Channel Adapters** | 725 | doing | T1-01, T1-02, T1-03 | `/Users/Roberdan/GitHub/convergio-daemon` |
| **H0 — TUI Hierarchy** | 719 | doing | T1-01, T1-02, T1-03, T1-04 | `/Users/Roberdan/GitHub/convergio-daemon` |
| **H0b — Mesh Delegation** | 720 | doing | T1-01, T1-02, T1-03, T1-04 | `/Users/Roberdan/GitHub/convergio-daemon` |

## Priority Order

**Execute Plan O W1 first** — highest strategic value, lowest effort, closes human-in-the-loop.

---

## Plan O W1 — Foundation + ntfy.sh Quick Win

### T1-01 (DB:9298) — Fix notify route + ntfy.sh

**P0 quick win. ~1 hour.**

1. Fix route mismatch in `src/orchestrator/reactor.rs`:
   - Find where orchestrator calls `/api/notify/send`
   - Change to `/api/notify` (the actual handler route)
2. Enable ntfy.sh in config:
   - Edit `config/notifications.conf` (or equivalent in convergio-daemon)
   - Add `[ntfy]` section: `enabled = true`, `topic = convergio`, `url = https://ntfy.sh`
3. Implement NtfyChannel in `src/resilience/notify.rs`:
   - Add `NtfyChannel` struct implementing `NotifyChannel` trait
   - Single `reqwest::Client::post(url/topic)` with title + message body
   - Priority mapping: critical→5, warning→3, info→1
4. Test: emit `need_human` event → verify ntfy.sh receives push
5. `cargo check && cargo test notify`

After: `cvg task update 9298 done "Fixed notify route, enabled ntfy.sh channel"`

### T1-02 (DB:9299) — ChannelAdapter trait

1. Create `src/channels/mod.rs`:
   ```rust
   pub mod telegram; // W2

   #[async_trait]
   pub trait ChannelAdapter: Send + Sync {
       async fn connect(&mut self) -> Result<()>;
       async fn send(&self, msg: &ChannelMessage) -> Result<()>;
       fn receive(&self) -> Pin<Box<dyn Stream<Item = ChannelMessage> + Send>>;
       async fn disconnect(&mut self) -> Result<()>;
       async fn health(&self) -> ChannelHealth;
   }

   pub struct ChannelMessage {
       pub id: String,
       pub source_channel: String,
       pub content: String,
       pub reply_to: Option<String>,
       pub metadata: serde_json::Value,
   }

   pub struct ChannelHealth {
       pub connected: bool,
       pub last_message_at: Option<DateTime<Utc>>,
       pub error_count: u64,
   }
   ```
2. Add `pub mod channels;` to lib.rs/main module
3. `cargo check && cargo test`

After: `cvg task update 9299 done "ChannelAdapter trait defined in src/channels/mod.rs"`

### T1-03 (DB:9300) — TUI notification inbox

1. Read `src/tui/views/` to understand view system
2. Add notification consumer:
   - Query `notification_queue` or `pending_notifications` table
   - Status bar widget: `[3 unread]` next to existing indicators
   - Key binding (e.g., `n`) to open notification list
   - Select notification → show detail → mark as read
3. `cargo check && cargo test tui`

After: `cvg task update 9300 done "TUI notification inbox with status bar indicator"`

### After W1 complete:
```bash
cvg task validate 9298 725
cvg task validate 9299 725
cvg task validate 9300 725
# Thor validates wave
cvg checkpoint save 725
```

---

## Plan H0 W1 — Project View + Hierarchy Navigation

Working dir: `/Users/Roberdan/GitHub/convergio-daemon`

### T1-01 (DB: check with `cvg plan show 719`) — Project view TUI tab

1. New TUI tab (key `0` or dedicated) showing projects list
2. Use `plan_hierarchy::project_plan_tree()` if exists, or query `/api/dashboard/projects`
3. Select project → drill into master plans → children

### T1-02 — Master plan tree with dependency arrows

1. Render master plan children as indented tree
2. Status colors: done=green, doing=yellow, blocked=red, todo=muted
3. Show `depends_on` as arrows or `→` labels

### T1-03 — Plan detail drill-down with hierarchy context

1. When drilling into sub-plan, show parent master plan name
2. Show sibling plans status as context bar

### T1-04 — Rollup progress bar for master plans

1. Master plan shows aggregate tasks_done/tasks_total from children
2. Percentage bar

After each: `cvg task update <id> done "summary"`

---

## Plan H0b W1 — Auto Sync-Back + Git Remote Setup

Working dir: `/Users/Roberdan/GitHub/convergio-daemon`

### T1-01 — `cvg mesh delegate` CLI

1. New CLI command wrapping `mesh-delegate-task.sh` logic
2. `cvg mesh delegate --peer X --prompt "..." --plan-id N`
3. Creates tmux, writes prompt, syncs repo, launches claude

### T1-02 — Auto git sync-back via post-commit hook

1. Post-commit hook pushes to coordinator via SSH remote
2. Fallback: saves patch, retries on heartbeat

### T1-03 — SSH remote auto-setup

1. `cvg mesh delegate` auto-configures `coordinator` git remote on peer
2. Verify SSH key access

### T1-04 — Prompt delivery via file

1. Write prompt to file, use `claude --input-file`
2. Never use `tmux send-keys` for long prompts

---

## Execution Rules

- TDD: failing test FIRST, then implement
- Max 250 lines/file
- `cargo check && cargo test` after every task
- Conventional commits: `feat(T1-01):`, `fix(T1-01):`
- Use worktrees: `worktree-create.sh plan-725-W1-T1-01`
- After task: `cvg task update <id> done "summary"`
- After wave: Thor validates, then `cvg checkpoint save <plan_id>`

## Sync Model

Code changes go to `convergio-daemon` repo. After wave merge, sync back to
ConvergioPlatform/daemon via rsync (per feedback_sync_model.md).
