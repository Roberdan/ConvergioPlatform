# API Development Standards

## REST

Methods: GET (retrieve) POST (create) PUT (replace) PATCH (partial) DELETE (remove) | No GET side effects
Naming: Plural nouns `/api/users` | kebab-case | Max 3 levels | `/api/v1/` prefix
Status: 200/201/204 | 400/401/403/404/409/422/429/500/503
Error: `{error: {code, message, details?, requestId, timestamp}}` | No internals exposed
Pagination: `?page=1&limit=20` (max 100) | `{data, pagination: {page, limit, total, hasNext, hasPrev}, links}`
Filter/Sort: `?status=active&sort=createdAt&order=desc` | Multi: `?sort=priority,createdAt`
Versioning: URL `/api/v1/` | Backwards compatible within major | Support 2+ majors
Rate limit: All public endpoints | 429 + `X-RateLimit-{Limit,Remaining,Reset}` headers
Auth: OAuth 2.0/JWT | `Authorization: Bearer {token}` | 401 invalid, 403 insufficient
Docs: OpenAPI spec | All endpoints+params+responses+examples | Interactive explorer
CORS: Allowlist origins (no `*` prod) | Specify methods/headers | Handle preflight
