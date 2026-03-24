# Compliance

## Security (OWASP)

Input: validate client+server, allowlists, sanitize | SQL: parameterized, least privilege | XSS: escape, CSP, DOMPurify | Secrets: env vars, `.env` gitignored | Auth: OAuth 2.0/OIDC, RBAC server-side | Transport: HTTPS, HSTS, secure cookies, TLS 1.2+ | Deps: scanned, pinned | Errors: no stack traces, rate limit

## Privacy (GDPR/CCPA)

Data minimization | Explicit consent | Privacy by design | User access/modify/delete | Encrypt at rest/transit | Breach notification

## Inclusive Language

Gender-neutral | blocklist/allowlist | primary/replica | Person-first | i18n/l10n

## AI Ethics

Disclose AI | Explain recommendations | Allow opt-out | Audit for bias | Human review high-stakes | No dark patterns | No misleading capabilities
