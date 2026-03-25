# ADR-0112: Channel Adapters Architecture (Plan 725)

## Status: Accepted
## Date: 25 Marzo 2026

## Context
Plan 725 adds bidirectional human-in-the-loop communication channels (Telegram, Slack, Email, ntfy). The platform needed a pluggable architecture for channel adapters that keeps each channel independent and testable.

## Decision
Implement channel adapters as separate Rust modules implementing a shared `ChannelAdapter` trait in `daemon/src/channels/`. Each channel (Telegram, Slack, Email, ntfy) is an independent module with fan-out routing configured via `notifications.conf`. Three waves, ten tasks.

## Alternatives Considered
- **Single unified adapter**: rejected because it does not scale when adding new channels.
- **External microservice**: rejected because it adds deployment complexity and network overhead for a local-first platform.

## Consequences
- New `daemon/src/channels/` module tree with trait-based adapter pattern
- Each adapter is independently testable with mock boundaries at the network layer
- Configuration-driven routing via `notifications.conf` enables user customization without code changes
- Adding a new channel requires only implementing `ChannelAdapter` trait and registering in the router
