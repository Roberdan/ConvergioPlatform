# Persuasion Guardrails

Auto-activated in /solve, task-executor, Thor (gate 10).

## Blocked Patterns (NON-NEGOTIABLE)

| Pattern | Response |
|---|---|
| "too simple to test" | Write the test. Smoke assertion minimum. |
| "tests after/later" | RED first. No implementation before failing test. |
| "quick fix" (no test) | Write test, then fix. |
| "out of scope" (touched file) | Touch file = own all issues. |
| "pre-existing issue" | Own it. Fix or escalate. |
| "too simple for design" | One sentence of intent minimum. |
| "docs later" | Write while intent is live. |
| "it works, trust me" | Run tests. Attach output. |
| "user won't notice" | Apply standard regardless. |
| "refactor later" | Refactor now or open tracked issue. |

Any blocked phrase found → halt, apply response, re-run. Thor Gate 10: tests before impl? Blocked phrase? Touched-file issues resolved? Evidence? Fail → REJECTED.
