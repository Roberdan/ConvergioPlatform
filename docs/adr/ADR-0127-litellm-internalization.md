# ADR-0127: LiteLLM Internalization (Zero Python Deps)

**Status**: Accepted
**Date**: 2026-04-02

## Context

LiteLLM was used as a Python proxy to multiplex LLM providers. This introduced a mandatory Python runtime dependency, a separate process to manage, and an additional failure domain. Provider support was limited to what LiteLLM exposed; adding new subscription-based providers (Claude, Copilot) required monkey-patching the proxy.

## Decision

Remove `Provider::LiteLLM` and the LiteLLM proxy sidecar. Replace with native Rust provider implementations:
- `ClaudeSubscription` — direct Anthropic API via `reqwest`
- `CopilotSubscription` — GitHub Copilot chat API via `reqwest`
- `LocalLLM` — Ollama-compatible local endpoint

All provider selection and routing is handled inside the daemon. No Python runtime is required.

## Consequences

**Positive**
- Zero Python dependencies; single Rust binary deployment.
- Provider code is type-checked at compile time; no runtime proxy configuration drift.
- Budget tracking integrated natively into the chat pipeline.
- Faster cold-start; no proxy warmup required.

**Negative**
- New providers must be implemented in Rust rather than added via LiteLLM config.
- Existing deployments using `Provider::LiteLLM` must migrate to a native provider.
