# Telegram Credential Management

Authoritative source: `~/.convergio/env` on each node.

## Architecture

```
~/.convergio/env (chmod 600)
  ├── CONVERGIO_TELEGRAM_TOKEN=<token>
  ├── CONVERGIO_TELEGRAM_CHAT_ID=<chat-id>
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
CONVERGIO_TELEGRAM_TOKEN=<your-token-here>
CONVERGIO_TELEGRAM_CHAT_ID=<your-chat-id-here>
HF_TOKEN=<your-hf-token-here>
EOF
chmod 600 ~/.convergio/env
```

Legacy compatibility: `TELEGRAM_BOT_TOKEN` and `TELEGRAM_CHAT_ID` are still
accepted at runtime, but all new setup should use the `CONVERGIO_*` names.

### Sync to Peer

```bash
# Via mesh auth sync (now: cvg mesh auth-sync)
cvg mesh auth-sync --peer <name>

# Manual SCP
scp ~/.convergio/env <peer>:~/.convergio/env
ssh <peer> chmod 600 ~/.convergio/env
```

## How the Daemon Loads Tokens

1. Daemon reads `~/.convergio/env` on startup
2. Kernel module uses `CONVERGIO_TELEGRAM_TOKEN` and
   `CONVERGIO_TELEGRAM_CHAT_ID` for bot long-polling
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
- Replicate via `cvg mesh auth-sync`, not manual copy
