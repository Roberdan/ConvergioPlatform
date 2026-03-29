# Telegram Credential Management

Authoritative source: `~/.convergio/env` on each node.

## Architecture

```
~/.convergio/env (chmod 600)
  ├── TELEGRAM_BOT_TOKEN=<token>
  ├── HF_TOKEN=<token>
  └── other secrets...
```

## Storage Locations

| Location | Purpose | Access |
|----------|---------|--------|
| `~/.convergio/env` | Runtime secrets (authoritative) | `source ~/.convergio/env` |
| macOS Keychain (M5Max) | Backup storage | `security find-generic-password` |
| `.env` files | NEVER — gitignored, not authoritative | — |

## Setup

### Initial Setup (per node)

```bash
# Create secrets directory
mkdir -p ~/.convergio && chmod 700 ~/.convergio

# Write token (chmod 600 automatic)
cat > ~/.convergio/env << 'EOF'
TELEGRAM_BOT_TOKEN=<your-token-here>
HF_TOKEN=<your-hf-token-here>
EOF
chmod 600 ~/.convergio/env
```

### Sync to Peer

```bash
# Via mesh auth sync
mesh-auth-sync.sh push --peer <name>

# Manual SCP
scp ~/.convergio/env <peer>:~/.convergio/env
ssh <peer> chmod 600 ~/.convergio/env
```

## How the Daemon Loads Tokens

1. Daemon reads `~/.convergio/env` on startup
2. Kernel module uses `TELEGRAM_BOT_TOKEN` for bot long-polling
3. If token missing: kernel Telegram module skips initialization (degraded mode)

## Rotation

1. Create new token via @BotFather on Telegram
2. Update `~/.convergio/env` on all nodes
3. Restart daemon: `cvg kernel stop && cvg kernel start`
4. Verify: `cvg kernel status` shows Telegram connected

## Security Rules (NON-NEGOTIABLE)

- NEVER commit tokens to git (hook-enforced)
- NEVER store in `.env` files in project directories
- NEVER hardcode in source code
- NEVER ask users for tokens — they are in `~/.convergio/env`
- Always chmod 600 on `~/.convergio/env`
- Replicate via `mesh-auth-sync.sh`, not manual copy
