---
applyTo: "**/*.{ts,tsx}"
---

# TypeScript Coding Standards

- Strict mode: `"strict": true` in tsconfig
- No `any` — use proper types or `unknown` with type guards
- Prefer `interface` over `type` for object shapes
- `const` over `let`, never `var`
- `async/await` over raw promises
- Test files: `.test.ts` with AAA pattern (Arrange, Act, Assert)
- Max 250 lines per file
- Comments: WHY not WHAT
