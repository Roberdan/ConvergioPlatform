#!/bin/bash
# Siri Shortcut: "Convergio stato"
# Returns Italian spoken summary of active plans.
DAEMON="http://localhost:8420"
DATA=$(curl -sf "${DAEMON}/api/plan-db/list" 2>/dev/null)
if [ -z "$DATA" ]; then echo "Il daemon Convergio non risponde."; exit 0; fi
python3 -c "
import json,sys
d=json.loads('''$DATA''')
doing=[p for p in d.get('plans',[]) if p.get('status')=='doing']
tasks=sum(p.get('tasks_total',0)-p.get('tasks_done',0) for p in doing)
names=', '.join(p.get('name','?')[:30] for p in doing[:3])
if doing:
    print(f'Hai {len(doing)} piani attivi con {tasks} task rimasti. {names}.')
else:
    print('Nessun piano attivo al momento.')
"
