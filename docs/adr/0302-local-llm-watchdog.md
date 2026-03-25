# ADR 0302 — Local LLM Watchdog (F-26)

**Date**: 25 Marzo 2026
**Status**: Accepted
**Plan**: 724 / Wave W2

## Context

Autonomous agent swarms can stall silently: task marked `in_progress` but no DB update for
minutes, worktrees orphaned, locks held. Remote LLM inference cannot be used for watchdog
decisions because: it adds network dependency, it leaks internal state to third-party APIs,
and it is unavailable when the network is the failure.

The watchdog needs to make restart-vs-escalate decisions locally.

## Decision

Use a **local LLM kernel** via Ollama HTTP API (`localhost:11434`) for decision summarisation
and triage. The watchdog (`daemon/src/resilience/watchdog.rs`) runs as a background tokio task
every 30 s.

### Why Ollama

| Criterion | Ollama | Remote API |
|---|---|---|
| Network dependency | None (local) | Required |
| Data privacy | State stays on device | Leaks to third party |
| Latency | ~200 ms (M-series) | ~1–3 s + TLS |
| Cost | Free | Per-token |
| Availability during failures | Yes | No |
| Model | `llama3` (default) | GPT-4 / Sonnet |

Ollama is already installed on macOS development nodes. Model `llama3` runs within
~2 GB RAM on M-series hardware — acceptable for background watchdog use.

### Fallback to Hardcoded Rules

When Ollama is unavailable (`/api/tags` returns error), the watchdog falls back to
deterministic rules with zero LLM dependency:

| Condition | Rule |
|---|---|
| Task stalled > `stale_threshold_secs` (default 300 s) | ALERT + mark blocked |
| Daemon `/api/health/deep` non-healthy | ALERT + attempt restart |
| Stale worktree detected | REAP (call reaper) |
| Lock file > 1 h old | REAP |
| Rate-limit error in logs | ALERT (critical) |
| Wave complete | NOTIFY (info) |

Hardcoded rules cover 95 % of practical cases. LLM is used only for human-readable
summaries and ambiguous escalation decisions.

### Decision Audit (F-27)

Every watchdog action is logged to `decision_log` table:

```
id, plan_id, task_id, decision, reasoning, first_principles,
alternatives_considered, outcome, created_at, agent
```

Query: `cvg decision log` or `GET /api/decisions?plan_id=X`

This creates an auditable trail: humans can review every restart, escalation, and reap
decision made by the watchdog.

### Notification Channels (F-28)

Watchdog dispatches via ordered `notification_channels` in `notifications.conf`:

| Channel | Trigger |
|---|---|
| ntfy.sh | Agent blocked > 5 min, crash, rate limit |
| Telegram | Critical failures only |
| macOS | Info/warning on local machine |

CLI: `cvg notify send "title" "message" [--severity critical|warning|info]`

### Self-Verification (F-29)

Before accepting `status=submitted`, the daemon runs all `verify[]` commands from
`test_criteria`. Failure rejects the submission with evidence. This prevents agents from
self-reporting success without proof. Verification results are logged to `decision_log`.

## Watchdog Configuration

File: `claude-config/config/notifications.conf`

```toml
[watchdog]
check_interval_secs = 30
ollama_url = "http://localhost:11434"
model_name = "llama3"
daemon_url = "http://localhost:8420"
stale_threshold_secs = 300

[[notification_channels]]
type = "ntfy"
url = "https://ntfy.sh/your-topic"

[[notification_channels]]
type = "telegram"
bot_token = "..."   # store in env: TELEGRAM_BOT_TOKEN
chat_id = "..."
```

CLI: `cvg watchdog start` / `cvg watchdog stop` / `cvg watchdog status`

## Consequences

**Positive**: watchdog is resilient to network failures; decisions are auditable; no
third-party data exposure; phone notifications enable truly autonomous overnight runs.

**Negative**: requires Ollama installed locally for LLM features (fallback covers absence);
`llama3` occupies ~2 GB RAM while loaded.

**Future**: replace Ollama with a lighter embedded model (llama.cpp GGUF) when startup
latency and RAM usage become constraints.

## Files

`daemon/src/resilience/watchdog.rs` | `notify.rs`
`daemon/src/server/api_decisions.rs`
`claude-config/config/notifications.conf`
