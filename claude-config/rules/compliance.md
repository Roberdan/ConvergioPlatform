<!-- v3.0.0 -->
# Compliance

## Security (OWASP)

Input: validate client+server, allowlists, sanitize, length limits | SQL: parameterized, ORM, least privilege | XSS: escape output, CSP, DOMPurify, no raw `dangerouslySetInnerHTML` | Secrets: never commit, env vars, `.env` gitignored | Auth: OAuth 2.0/OIDC, RBAC server-side, secure sessions | Transport: HTTPS, HSTS, secure cookies (Secure/HttpOnly/SameSite), TLS 1.2+ | Deps: scanned (Snyk/npm audit), pinned | Errors: no stack traces to users, rate limit

## Ethics (GDPR/WCAG)

Privacy: data minimization, explicit consent, user rights, encrypt at rest+transit | A11y WCAG 2.1 AA: keyboard, 4.5:1 contrast, screen readers, 200% resize | Language: gender-neutral, blocklist/allowlist, i18n | AI: disclose, explain, opt-out | Consent: plain language, no dark patterns
