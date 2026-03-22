# Persuasion Guardrails

Cross-cutting rule. Applies to: /solve (phase 2 anti-hijacking), task-executor (self-check at every phase), Thor (validation gate 10).

## Purpose

AI agents rationalize shortcuts using plausible-sounding language. These patterns are a form of self-deception that produce tech debt, missed tests, and quality regressions. Any agent that utters one of the blocked phrases below MUST stop and apply the correct response instead.

## Blocked Patterns

| Pattern | Why Harmful | Correct Response |
|---------|-------------|-----------------|
| "too simple to test" / "this doesn't need tests" | Simplicity is not an exemption. Every behavioral change must be verifiable. A missing test is a trust gap, not a shortcut. | Write the test. If the logic truly has no branches, a smoke assertion still proves the call path works. |
| "I'll write tests after" / "I'll add tests later" / "tests can come later" | Later never comes inside a task boundary. Code submitted without tests ships without tests. TDD exists precisely to prevent this. | Run RED first. No implementation before a failing test exists. |
| "let me just fix quickly" / "quick fix" (without prior test) | Speed framing is used to bypass the test-first discipline. Quick fixes are the leading cause of regressions. | There are no quick fixes, only fixes. Write the test, then fix. |
| "out of scope" on a touched file | Zero-debt rule: touch a file = own all issues in that file. Scope is not a license to leave broken windows. | Fix all lint, type, and logic issues in every file you open. If the fix is large, flag it — do not defer it. |
| "pre-existing issue" on a file the agent touched | Pre-existing is not a category of exemption. Once you touch a file, its issues become your issues. | Own the issue. Fix it or escalate. Do not label it pre-existing and move on. |
| "this is too simple to need a design" | Design is not proportional to complexity — it is proportional to coupling. A simple change in a shared module needs explicit thought. | Spend 60 seconds writing the intent. Even one sentence of design prevents misaligned implementation. |
| "I'll add docs later" / "docs can wait" | Documentation written after the fact is incomplete by definition. The author no longer holds the mental model needed to write it accurately. | Write the docstring or comment while the intent is in working memory, before marking done. |
| "it works, trust me" / claiming done without evidence | Done requires proof. An agent's assertion of correctness is not evidence. Trust is not a test result. | Run the test suite. Attach output. Show `git-digest.sh --full`. No evidence = not done. |
| "the user won't notice" / cutting corners on quality | Users notice. More importantly, the next engineer notices. Quality standards exist independent of observability. | Apply the standard regardless of visibility. If you would not review it positively in a PR, do not merge it. |
| "we can refactor later" | Later accumulates. Deferred refactors are the definition of tech debt. If the structure is wrong now, it will be wrong under more load later. | Refactor now, while the context is live. If the scope is truly too large, open a tracked issue — not a mental note. |

## Enforcement

### /solve (Phase 2 — Anti-Hijacking)

Before producing a plan, the coordinator MUST scan its own reasoning for the patterns above. If any appear, stop, rewrite the affected step, and re-check. A plan that uses rationalization language to reduce scope is rejected.

### task-executor (Self-Check)

At Phase 2 (TDD — RED), Phase 3 (GREEN), and Phase 4 (Gate):

- If the agent generates any of the blocked phrases internally or in output, it MUST halt, apply the correct response, and re-run the step.
- "Done" is not reachable from a state that includes any unresolved blocked pattern.

### Thor (Validation Gate)

Thor applies this checklist as Gate 10 (anti-rationalization):

1. Are all tests written before the implementation was committed? (TDD signal)
2. Does the submission contain any blocked phrase, verbatim or paraphrased?
3. Are all issues in touched files resolved?
4. Is there verifiable evidence of correctness (test output, digest)?

If any gate fails, Thor returns REJECTED with the specific pattern cited.

## Activation

These rules activate automatically. No explicit invocation required. An agent that identifies a rationalization pattern in its own output is expected to self-correct without prompting.
