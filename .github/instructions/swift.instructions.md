---
applyTo: "**/*.swift"
---

# Swift Coding Standards

- macOS 14+ (Sonoma), Swift 5.9+
- State management: `@Observable` (NOT `ObservableObject`)
- Use `@State` for view-owned state, `@Environment` for shared
- Accessibility: VoiceOver labels on all interactive elements (WCAG 2.1 AA)
- Max 250 lines per file
- No hardcoded test data — use real daemon API data
- Networking: `URLSession` with `async/await`, no Combine
- Error handling: structured errors, no force unwraps
- MARK comments for section organization
