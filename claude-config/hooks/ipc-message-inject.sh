#!/usr/bin/env bash
set -euo pipefail
AGENT="${CONVERGIO_AGENT_NAME:-}"
[ -z "$AGENT" ] && exit 0
API_URL="${CONVERGIO_API_URL:-http://localhost:8420}"
CURSOR_DIR="${HOME}/.claude/data"
CURSOR_FILE="${CURSOR_DIR}/ipc-cursor-${AGENT}"
mkdir -p "$CURSOR_DIR"
LAST_ID=""
[ -f "$CURSOR_FILE" ] && LAST_ID="$(cat "$CURSOR_FILE" 2>/dev/null || true)"
JSON="$(curl -sf "${API_URL}/api/ipc/messages?to_agent=${AGENT}&limit=5" 2>/dev/null || true)"
[ -z "$JSON" ] && exit 0
python3 - "$JSON" "$LAST_ID" "$CURSOR_FILE" <<'PY'
import json,sys
payload=sys.argv[1]
last_id=sys.argv[2]
cursor=sys.argv[3]
try:
    data=json.loads(payload)
except Exception:
    raise SystemExit(0)
messages=data.get("messages",[])
messages=list(reversed(messages))
seen=False
latest=last_id
for m in messages:
    mid=str(m.get("id",""))
    if not mid:
        continue
    latest=mid
    if last_id and not seen:
        if mid==last_id:
            seen=True
        continue
    frm=m.get("from_agent","?")
    content=m.get("content","")
    print(f"📨 [{frm} → you]: {content}", file=sys.stderr)
if latest:
    open(cursor,"w",encoding="utf-8").write(latest)
PY
