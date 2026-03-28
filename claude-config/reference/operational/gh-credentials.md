# Per-Repo GitHub Credential Routing

Eliminates `gh auth switch` by configuring each repo's local `credential.helper` to use `gh auth token` directly.

## Problem

Switching between GitHub accounts (personal vs org) wastes tokens and causes auth failures when agents work across repos on mesh nodes.

## Solution

`scripts/mesh/setup-gh-credentials.sh` scans all repos in a directory, identifies those with `github.com` remotes, and sets:

```
git config --local credential.helper '!gh auth token'
```

This routes all git credential requests through the active `gh` session without needing per-repo account switching.

## Usage

| Command | Purpose |
|---|---|
| `setup-gh-credentials.sh` | Configure all repos in ~/GitHub |
| `setup-gh-credentials.sh --scan-dir /path` | Custom scan directory |
| `setup-gh-credentials.sh --dry-run` | Preview without changes |

## Integration

Runs automatically during `deploy-node.sh` (step 5b2) after gh auth token propagation. Also safe to run manually on any node.

## Prerequisites

- `gh` CLI installed and authenticated (`gh auth login`)
- Repos cloned with HTTPS or SSH remotes pointing to github.com

## Scope

Only repos with `github.com` in their origin URL are configured. GitLab, Bitbucket, and other remotes are skipped.
