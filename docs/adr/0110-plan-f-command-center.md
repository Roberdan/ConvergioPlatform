<!-- Copyright (c) 2026 Roberto D'Angelo. MPL-2.0. -->
# ADR-0110: Plan F Command Center

## Status

Accepted

## Context

Plan F replaces the legacy web-embedded mission control experience with a native macOS
application named `CommandCenter`. The new surface needs first-class onboarding, secure
daemon access, live plan and agent visibility, mesh and evolution operations, cost and run
observability, PTY terminal access, a realtime brain graph, and native notifications.

The implementation also had to stay aligned with the live daemon APIs already present in
ConvergioPlatform instead of introducing parallel backend contracts.

## Decision

### Security fixes rationale

- Authentication stays daemon-issued and token-based, with the token stored in the macOS
  Keychain instead of in project files or app preferences.
- WebSocket and HTTP requests reuse the same bearer token path so native views do not
  invent separate trust flows.
- PTY access is routed through `/ws/pty`, which already enforces peer validation,
  tmux-session sanitization, idle timeout, and the daemon-level 10-session cap.
- Notification delivery is filtered client-side by category so the app can reduce noisy
  or high-risk alerting without disabling the daemon broadcast channel globally.

### Error handling strategy

- Native view models are `@Observable` and fail loudly through visible `errorMessage`
  state rather than silent returns.
- The app keeps the live daemon contract authoritative: when the backend does not expose
  a metric, the UI shows the gap explicitly instead of fabricating data.
- Dashboard and brain WebSocket refresh paths re-fetch canonical snapshots after events so
  the app does not drift from daemon state when payloads are partial.

### SwiftUI architecture

- `CommandCenter` is organized by feature folders under `Sources/Views/*`, with thin API
  extensions under `Sources/API/*`.
- Each major surface owns a focused `@Observable` model and a small set of feature-local
  views instead of a single global store.
- `ContentView` is the single routing source of truth for native sections: Plans, Agents,
  Mesh, Evolution, Costs, Terminal, and Brain.
- Reusable infrastructure stays centralized: `DaemonClient` for REST, `WebSocketManager`
  for broadcast channels, and feature-specific session types for multi-connection cases
  such as the terminal tabs.

### Ghostty integration approach

- The terminal UX is branded as the native `GhosttyTerminal` surface, but the transport is
  daemon-native `/ws/pty`, not a private terminal backend.
- The app implements cmux-style tab/session multiplexing on top of multiple PTY sockets,
  with explicit peer selection and tmux attach/create flows.
- Keyboard passthrough is handled by a custom AppKit-backed terminal text view so the
  terminal remains interactive inside SwiftUI without introducing non-repo dependencies.

### Metal brain visualization

- The brain surface uses a force-directed layout for plans, tasks, and agents, refreshed by
  `/api/brain` snapshots and `/ws/brain` events.
- A `.metal` shader artifact is kept in the repo for the feature contract, while runtime
  Metal compilation is used by the app so the surface remains functional even when the
  standalone Metal command-line toolchain is absent from the local Xcode installation.
- The graph remains clickable and inspectable at the SwiftUI layer, while the Metal-backed
  animated backdrop provides the visual depth expected from the native command center.

### Auth flow design

- First launch is gated behind onboarding.
- Onboarding generates or accepts a daemon token, verifies connectivity, and persists the
  token to Keychain before the main UI is shown.
- After onboarding, every native section reads the shared auth state through `DaemonClient`
  and does not duplicate token prompts.

## Consequences

- `CommandCenter` is now a native macOS control plane rather than a WebView shell.
- The app is tightly aligned with the daemon API surface, which lowers drift but means UI
  capabilities remain bounded by backend contracts.
- Runtime Metal shader compilation avoids a local toolchain blocker during development, but
  shipping environments should still prefer a full Xcode + Metal installation for the most
  predictable debugging experience.
