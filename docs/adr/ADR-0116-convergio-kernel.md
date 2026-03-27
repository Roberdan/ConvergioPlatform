# ADR-0116: Convergio Kernel — Always-On Local LLM Watchdog

**Status**: Accepted
**Date**: 27 Marzo 2026
**Plan**: 729

## Context

Convergio needed a persistent, always-on health monitor running on a dedicated low-power node (M1 Pro). The monitor must classify system events using local LLM inference, execute deterministic recovery actions, deliver voice/text notifications, and route audio to the active user node — all without external API dependencies during failure conditions.

## Decisions

### Mistral 3 8B as base model

Mistral 3 8B was selected over Llama 3 8B and Gemma 2 9B because it provides native function calling (required for structured classification output), ships under Apache 2.0 (no usage restrictions), and fits within 5 GB RAM — comfortably within the M1 Pro 16 GB budget with headroom for daemon + OS.

### MLX over Ollama for inference

MLX (Apple's ML framework) was chosen over Ollama because it delivers 20–30% faster inference on Apple Silicon via Metal GPU acceleration. The existing `AppleFmBridge` integration layer already abstracts MLX, eliminating new integration work. Ollama would require an additional server process and HTTP round-trips with no performance benefit on Apple Silicon.

### M1 Pro as dedicated kernel node

The kernel runs exclusively on the M1 Pro (macProM1) rather than being distributed across the mesh. Rationale: the M1 Pro is always-on, draws low power (~10 W idle), is not used for heavy development workloads, and a fixed kernel location simplifies mesh routing (all nodes know where to find the kernel). The watchdog role benefits from a stable, predictable host.

### Deterministic recovery: LLM classifies, rules act

The kernel uses a two-phase approach: the LLM classifies the event type and severity, then a deterministic rule engine selects and executes the recovery action. The LLM never directly triggers side effects. This eliminates hallucination risk in recovery paths — a misclassified event degrades to a notification, not an incorrect action.

### Telegram for notifications

Telegram was selected over Slack, ntfy.sh, and custom push because its Bot API is simple HTTP with no SDK dependency, supports OGG/Opus voice messages natively (Voxtral output format), and works globally without self-hosting. The bot token is a single env var (`CONVERGIO_TELEGRAM_TOKEN`), keeping secrets management minimal.

### Audio mesh routing

Voice alerts are routed via the mesh rather than played locally on the kernel node. The user may be working at any mesh node (macProM4, macProM1, or any remote node), but the kernel is always fixed on M1 Pro. Audio routing resolves `cvg kernel here` to the active node and sends the audio payload there, ensuring alerts reach the user regardless of their current working node.

### macOS `say` as TTS fallback

`say` is used as the current TTS backend because it is available today with zero dependencies on all macOS nodes. The kernel falls back to `say` when Voxtral is unavailable. Voxtral (Mistral's voice model) is planned as the production TTS replacement when it reaches stable local inference on MLX. The fallback path is explicit and not hidden.

## Node Lifecycle (Plan 732)

Plan 732 extends the kernel with node-aware lifecycle management. Three decisions were made:

### Single-command node deployment via deploy-node.sh

`scripts/mesh/deploy-node.sh` was added as a single-command entry point for bootstrapping a new mesh node (SSH key push, peers.conf sync, daemon install, kernel config). The alternative was a manual checklist. A script was chosen to enforce repeatability and eliminate configuration drift across nodes.

### /api/node/readiness endpoint (10-check report)

A structured readiness endpoint was added (daemon, mesh reachability, DB sync, SSH access, kernel config, role assignment, disk space, model presence, Telegram token, audio target). This makes pre-deployment validation machine-readable and integrable with future CI/CD pipelines. The 10 checks were selected to cover all known failure modes from Plan 729 post-mortem.

### Periodic readiness check in kernel monitor (every 5 min)

The kernel monitor loop was extended to call `/api/node/readiness` every 5 minutes and emit a WARNING-level classification event if any check fails. This surfaces degraded nodes proactively rather than waiting for a task failure. The interval (5 min) was chosen to balance signal freshness against inference cost on the M1 Pro.

### scripts/kernel/sync-db.sh for safe DB rsync

A dedicated sync script was added instead of raw `rsync` invocations. The script performs a pre-sync integrity check (`PRAGMA integrity_check`), uses `--checksum` mode to avoid unnecessary transfers, and verifies the destination after transfer. Direct `rsync` without these guards was the cause of two corrupted-DB incidents in Plan 729 mesh handoff testing.

## Consequences

- Kernel binary must be present on M1 Pro; `cvg kernel start` fails with clear error if mlx_lm is not installed.
- `CONVERGIO_TELEGRAM_TOKEN` must be set in the daemon environment on M1 Pro for Telegram delivery.
- Audio routing adds a mesh dependency — if the mesh is partitioned, audio falls back to local `say` on kernel node.
- Mistral 3 8B model file (~5 GB) must be downloaded before first use; `cvg kernel start` handles this with a progress indicator.
- Future Voxtral upgrade is a drop-in replacement of the TTS adapter; no architectural change required.
