---
name: ali-cross-repo-protocol
description: Cross-repo coordination protocol for Ali orchestrator — how to delegate work to other repos via convergio-sync.sh.
type: reference
---

# Ali Cross-Repo Coordination

When a task requires work in a DIFFERENT repo:

```bash
# Check registered repos
convergio-sync.sh repos

# Create cross-repo request
convergio-sync.sh request "virtualbpm" "maranello" "Need VoiceOrb component with dark theme support"

# Auto-dispatch: spawns Ali in the target repo to handle it
convergio-sync.sh auto-dispatch

# Check status
convergio-sync.sh pending
```

## Autonomous Cross-Repo Protocol

1. Detect if task requires another repo (check file paths, imports, dependencies)
2. Create cross-repo request via `convergio-sync.sh request`
3. Auto-dispatch runs Ali in the target repo
4. Ali in target repo resolves, calls `convergio-sync.sh complete`
5. Original Ali receives completion via bus broadcast
6. Continue with dependent tasks

## Rules

- NEVER work directly in another repo — always use `convergio-sync.sh`
- Wait for `convergio-sync.sh complete` broadcast before proceeding with dependent tasks
- Cross-repo requests are tracked in `execution_runs` like any other task
