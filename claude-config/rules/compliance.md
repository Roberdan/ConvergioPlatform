<!-- v4.0.0 — Merged: compliance + ethical-guidelines -->
# Compliance

## Security (OWASP)

Input: validate client+server, allowlists, sanitize, length limits | SQL: parameterized, ORM, least privilege | XSS: escape output, CSP, DOMPurify, no raw `dangerouslySetInnerHTML` | Secrets: never commit, env vars, `.env` gitignored | Auth: OAuth 2.0/OIDC, RBAC server-side, secure sessions | Transport: HTTPS, HSTS, secure cookies (Secure/HttpOnly/SameSite), TLS 1.2+ | Deps: scanned (Snyk/npm audit), pinned | Errors: no stack traces to users, rate limit

## Privacy (GDPR/CCPA)

**Data minimization** | Explicit consent | Clear privacy policies | Privacy by design/default | User access, modify, delete rights | Encrypt at rest/transit | Regular privacy impact assessments | Prompt breach notification | Secure defaults (opt-in sharing)

## Accessibility (WCAG 2.1 AA)

**All UIs keyboard navigable** | Text alternatives for non-text | 4.5:1 contrast | Screen reader support | Captions for audio/video | 200% text resize | Design for diverse abilities (motor, visual, auditory, cognitive) | Test with actual users with disabilities

## Inclusive Language

Gender-neutral | blocklist/allowlist (not black/whitelist) | primary/replica (not master/slave) | Person-first for disabilities | No cultural assumptions | i18n/l10n support | Cultural sensitivities in imagery | Clear, simple language

## AI Ethics & Transparency

Disclose AI interactions | Explain recommendations | Allow opt-out | Document capabilities/limitations | Confidence scores | User feedback on decisions | Audit for bias across protected characteristics | Diverse training data | Test for disparate impact | Human review for high-stakes | Monitor for emerging biases

## Consent & Control

Informed consent | Plain language | Granular options (not all-or-nothing) | Easy privacy controls | Respect Do Not Track | Withdrawal anytime | No dark patterns

## Environmental Responsibility

Optimize for energy efficiency | Consider carbon footprint | Renewable energy hosting | Efficient caching/data transfer | Monitor resource consumption

## Honesty & Integrity

Never mislead about capabilities | Communicate limitations/risks | Fix mistakes promptly | No deceptive patterns | Transparent business model

## Anti-Patterns

All-or-nothing consent | Dark patterns | Hidden privacy settings | Misleading capabilities | Unexplained AI decisions | Inaccessible UIs | Discriminatory defaults | Unlimited data collection
