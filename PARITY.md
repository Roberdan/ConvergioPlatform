# ConvergioPlatform — Capability Parity Audit

> Generated: audit of declared capabilities vs actual implementation.
> Method: file-existence verification in worktree `/tmp/cvg-plan-10046/`.

## Summary

| Status | Count |
|--------|-------|
| ✅ Working | 14 |
| ⚠️ Partial | 2 |
| ❌ Missing | 1 |

---

## Capability Audit

| # | Capability | Status | Evidence | Known Gaps |
|---|-----------|--------|----------|------------|
| 1 | **Core Architecture** — daemon build/serve | ✅ Working | `daemon/Cargo.toml` (crate `convergio-platform-daemon`), `daemon/start.sh` (launcher with ulimit, env loading) | None |
| 2 | **Plan Management** — lifecycle, Thor gates | ✅ Working | `daemon/src/server/api_plan_db.rs` (plan CRUD, evidence, task update routes), `daemon/src/server/api_validation.rs` (validation queue, verdict, enqueue endpoints) | None |
| 3 | **Task Execution** — evidence, validation | ✅ Working | `daemon/src/server/state_init_migrations.rs` contains `CREATE TABLE IF NOT EXISTS task_evidence`; api_plan_db.rs has evidence POST endpoint; api_validation.rs has verdict recording | None |
| 4 | **Agent Orchestration** — launch, bus, IPC, catalog | ✅ Working | `daemon/src/cli_launch.rs`, `daemon/src/cli_bus.rs`, `daemon/src/cli_ask.rs` all exist at top-level `src/`; `daemon/src/server/api_ipc/` has `handlers.rs`, `handlers_ask.rs`, `handlers_ext.rs`, `handlers_ext2.rs`, `routes.rs`, `schema.rs`, `sse_stream.rs` | CLI files are at `daemon/src/` not `daemon/src/cli/` — two CLI locations exist |
| 5 | **Mesh/Swarm** — TCP, HMAC, mDNS, peer sync | ✅ Working | `daemon/src/mesh/` — 60+ files: `auth.rs` (HMAC), `lan_discovery.rs` (mDNS `_convergio._tcp.local.`), `daemon_sync*.rs` (frame sync), `ws.rs` (WebSocket), `net.rs` (TCP), `peers/`, `coordinator/`, `token.rs` | None — comprehensive implementation |
| 6 | **Jarvis/Kernel** — Qwen 7B, MLX | ✅ Working | `daemon/src/kernel/` — `engine.rs`, `engine_context.rs`, `engine_tool_loop.rs` (inference engine); `api.rs`, `api_ask.rs` (kernel API); `monitor.rs`, `monitor_checks.rs` (health monitoring) | MLX integration exists in engine; Qwen 7B is the default local model |
| 7 | **Voice Engine** — TTS, STT | ✅ Working | `daemon/src/kernel/tts.rs`, `tts_ext.rs`, `tts_templates.rs` (text-to-speech); `daemon/src/kernel/stt.rs` (speech-to-text); `daemon/src/server/api_voice.rs` (start/stop/status/test API); `daemon/src/kernel/voice_router.rs`, `voice_routes.rs`, `audio.rs`, `audio_routing.rs` | None — full voice pipeline |
| 8 | **Telegram Bot** — poll, voice, transcription | ✅ Working | `daemon/src/kernel/telegram_poll.rs` (long-polling inbound text); `daemon/src/kernel/telegram_voice.rs` (OGG download, transcribe, route, reply); `daemon/src/kernel/telegram.rs` (outbound: `send_text`, `send_voice`, quiet-hours, NotifyMode) | None |
| 9 | **Siri** — shortcuts | ✅ Working | `scripts/siri/` — `convergio-parla.sh`, `convergio-stato.sh`, `speak-siri`, `speak.swift`, `README.md` | Shell+Swift scripts, not a native Shortcuts app |
| 10 | **Evolution Engine** — TS engine | ⚠️ Partial | `evolution/` exists with `package.json`, `vitest.config.ts`, subdirs: `adapters/`, `agents/`, `analysis/`, `cadence/`, `canary/`, `core/`, `experiments/`, `guardrails/`, `reporting/`, `research/`, `roi/`, `runtime/`, `telemetry/`, `tests/` | Not wired to daemon — standalone TS project. No daemon API routes reference evolution. No `evolution/src/` directory (code lives in subdirs directly) |
| 11 | **Frontend** — convergio-frontend | ❌ Missing | No `convergio-web/` or `convergio-frontend/` directory in this repo | Separate repository. `dashboard_web/` exists but is a static build output, not source |
| 12 | **Channels** — beyond Telegram | ✅ Working | `daemon/src/server/api_channels.rs` — routes: `GET /api/channels`, `POST /api/channels/:name/send`, `GET /api/channels/:name/health`; supported channels: **telegram**, **ntfy**, **macos** (macOS native notifications); `dashboard` exists in config but filtered out | Only 3 active channels |
| 13 | **LLM Routing** — multi-provider, fallback | ✅ Working | `daemon/src/server/provider.rs` — providers: `ClaudeSubscription`, `CopilotSubscription`, `LocalLLM` with alias mapping; `daemon/src/inference/fallback.rs` — `FallbackChain`, `FallbackExecutor::execute_with_fallback()`, max_attempts, ordered model list; `daemon/src/config/mod.rs` — `InferenceFallbackConfig` | None — full fallback chain implemented |
| 14 | **Security** — auth, hooks, secret scan | ✅ Working | `daemon/src/server/middleware.rs` — JWT validation + legacy bearer token (`CONVERGIO_AUTH_TOKEN`), constant-time compare, RBAC, `/api/health` exempt, `--dev-mode` bypass; `.git/hooks/pre-commit` and `.git/hooks/commit-msg` exist in main repo | Hooks are in main repo `.git/hooks/`, not in worktree (expected git behavior) |
| 15 | **Memory** — recall, share, attest | ✅ Working | `daemon/src/server/api_memory.rs` — endpoints: `POST /remember`, `GET /recall`, `DELETE /forget/{id}`, `POST /share`, `POST /attest` | None |
| 16 | **MCP Server** | ✅ Working | `daemon/src/mcp_server/` — 15 files: `main.rs`, `protocol.rs`, `handlers.rs`, `tool_catalog.rs`, `tools.rs`, `plan_tools.rs`, `org_tools.rs`, `platform_tools.rs`, `agent_chat.rs`, `agent_factory.rs`, `invoke_agent.rs`, `security.rs`, `web_search.rs`, `tests.rs` | None — full MCP protocol implementation |
| 17 | **A2UI** — agent-to-UI | ⚠️ Partial | `daemon/src/server/api_a2ui.rs` (block push handlers), `daemon/src/server/api_a2ui_sse.rs` (SSE stream + TTL cleanup), `state_init_migrations.rs` has `CREATE TABLE IF NOT EXISTS a2ui_blocks` | Backend exists; requires frontend (separate repo) to render blocks. No frontend in this repo to verify end-to-end |

---

## Architecture Cross-Reference

```
daemon/src/
├── cli/              # CLI command modules (auth, coordinator, env)
├── cli_launch.rs     # Agent launch commands
├── cli_bus.rs        # Bus/messaging commands
├── cli_ask.rs        # Ask/query commands
├── config/mod.rs     # InferenceFallbackConfig, runtime config
├── inference/        # fallback.rs — FallbackChain
├── ipc/              # Engine IPC, channels_context
├── kernel/           # Jarvis: engine, TTS, STT, Telegram, voice router
├── mcp_server/       # MCP protocol, tools, agent factory
├── mesh/             # TCP, HMAC auth, mDNS, peer sync, WebSocket
└── server/           # HTTP API: plan_db, validation, memory, voice,
                      #   channels, a2ui, ipc/, middleware, provider
```

## Notes

1. **Evolution** is a standalone TS project — no daemon integration routes found
2. **Frontend** lives in a separate repository; `dashboard_web/` is build output only
3. **A2UI** backend is complete but needs frontend to verify end-to-end rendering
4. **Channels** support 3 targets: Telegram, ntfy, macOS native
5. **LLM providers**: Claude (Anthropic), Copilot (GitHub), LocalLLM (Qwen/MLX)
6. **Security hooks** enforce: no main commits, 250-line limit, no secrets, conventional commits
