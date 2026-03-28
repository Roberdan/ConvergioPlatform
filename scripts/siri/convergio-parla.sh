#!/bin/bash
# Siri Shortcut: "Convergio" (free-form conversation)
# Takes dictated text from Siri, sends to kernel, speaks the answer.
QUESTION="$1"
[ -z "$QUESTION" ] && { echo "Dimmi qualcosa."; exit 0; }

# Try local kernel, fall back to M1 Pro via SSH
DAEMON="http://localhost:8420"
if curl -sf "${DAEMON}/api/kernel/status" > /dev/null 2>&1; then
  ANSWER=$(curl -sf -X POST "${DAEMON}/api/kernel/ask" \
    -H "Content-Type: application/json" \
    -d "{\"question\":\"${QUESTION}\"}" 2>/dev/null \
    | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d.get('answer','Non ho una risposta.'))" 2>/dev/null)
else
  ANSWER=$(ssh m1Pro "curl -sf -X POST http://localhost:8420/api/kernel/ask \
    -H 'Content-Type: application/json' \
    -d '{\"question\":\"${QUESTION}\"}'" 2>/dev/null \
    | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print(d.get('answer','Non ho una risposta.'))" 2>/dev/null)
fi

echo "${ANSWER:-Il kernel non ha risposto.}"
