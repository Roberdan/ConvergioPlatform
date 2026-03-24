# Execution: Testing

## Mock Boundaries (NON-NEGOTIABLE)

ALLOWED: External APIs, Network I/O, File system, Time/Date | FORBIDDEN: Auth, DB queries (use test DB), module under test, internal routing. Boundaries only.

## Integration Tests

New endpoint → real middleware | New consumer → realistic shape | Interface change → ALL consumers | Every plan: >=1 integration test full data path.

## NON-NEGOTIABLE Test Rules

**Real Data**: No `Studio A`/`Test Studio`. Real names/shapes. | **Schema-Migration**: Model change → migration same PR. | **Signature Change**: Add/remove param → grep ALL callers+tests → update. | **Test Domains**: `example.com`/`example.org` only. | **Field Addition**: Update ALL test fixtures.

## Test Quality (Thor Gate 8)

Mock depth <=2 | No self-mock | Coverage with assertions | Format matches prod | Consumer tests | Migration exists | Safe domains
