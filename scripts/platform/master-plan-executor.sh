#!/bin/bash
# master-plan-executor.sh — Generates prompt for next master plan session
# Usage: ./scripts/platform/master-plan-executor.sh
# Why: single Claude session can't hold 104 tasks. This chains sessions.
set -euo pipefail

/usr/bin/python3 << 'PYEOF'
import subprocess, json, sys

PLANS = [
    (719, "H0 — TUI Project Hierarchy", []),
    (720, "H0b — Mesh Delegation Robusta", []),
    (712, "H — Hardening NASA", [719]),
    (713, "I — Checklist Engine", [712]),
    (715, "K — Agentic Memory", [713]),
    (721, "F2 — SwiftUI Command Center", [713, 719]),
    (714, "J — MCP Client", [713]),
    (716, "L — Security Perimeters", [714]),
    (717, "M — Inference Router", [712]),
    (718, "N — Voice", [717]),
]

def get_status(pid):
    try:
        raw = subprocess.run(
            ["cvg", "plan", "show", str(pid)],
            capture_output=True, text=True, timeout=10
        ).stdout
        lines = [l for l in raw.strip().split('\n') if l.startswith('{')]
        if not lines: return {"status": "unknown", "done": 0, "total": 0, "wave": "?"}
        d = json.loads(lines[0])
        p = d.get("plan", {})
        tasks = d.get("tasks", [])
        done = sum(1 for t in tasks if t.get("status") == "done")
        waves = d.get("waves", [])
        cw = next((w.get("wave_id", "?") for w in waves if w.get("status") in ("in_progress", "pending")), "all_done")
        return {"status": p.get("status", "?"), "done": done, "total": len(tasks), "wave": cw}
    except:
        return {"status": "unknown", "done": 0, "total": 0, "wave": "?"}

statuses = {}
for pid, name, deps in PLANS:
    statuses[pid] = get_status(pid)

print("=== Convergio Master Plan v2.0 — Status ===\n")

completed = set()
next_plans = []

for pid, name, deps in PLANS:
    s = statuses[pid]
    st = s["status"]
    prog = f'{s["done"]}/{s["total"]}'

    if st in ("completed", "done"):
        print(f'  ✅ {pid} {name} — {prog}')
        completed.add(pid)
    elif st == "doing":
        print(f'  🔄 {pid} {name} — {prog} (wave {s["wave"]})')
        next_plans.append(pid)
    elif all(d in completed or statuses.get(d, {}).get("status") in ("completed", "done") for d in deps):
        print(f'  ⏳ {pid} {name} — ready')
        next_plans.append(pid)
    else:
        blocked = [str(d) for d in deps if d not in completed]
        print(f'  🔒 {pid} {name} — blocked by {", ".join(blocked)}')

# Also check Plan O (725) and Plan P
print(f'  ✅ 725 O — Channel Adapters — DONE')

if not next_plans:
    print("\n🎉 ALL PLANS COMPLETED")
    sys.exit(0)

# Max 2 plans per session to keep context manageable
session_plans = next_plans[:2]

print(f'\n=== Next session: {", ".join(str(p) for p in session_plans)} ===\n')

prompt = f"""Esegui i piani Convergio. Obiettivo: completare i piani target, poi stampa il prompt per la sessione successiva.

## Piani target per questa sessione
"""

for pid in session_plans:
    s = statuses[pid]
    name = next(n for p, n, _ in PLANS if p == pid)
    prompt += f"""
### Plan {pid}: {name}
- Status: {s['status']}, progress: {s['done']}/{s['total']}, wave: {s['wave']}
- Spec: `ls specs/plan-*` e cerca il match
- DB: `cvg plan show {pid}`
"""

prompt += """
## Regole di esecuzione (NON-NEGOTIABLE)
- Max 4 task per batch, poi verifica test e checkpoint
- Worktree isolati per ogni task (Agent con isolation="worktree")
- Cherry-pick SEMPRE via agent dedicato — MAI inline nel coordinator
- Checkpoint (`cvg checkpoint save <plan_id>`) dopo OGNI task
- `git log --oneline | grep -i BUG` prima di lanciare fix agents (no re-fix)
- Se auth failure: dimmi di fare /login
- Thor verify fix: se "cd daemon" fallisce, update notes via API
- Se context pieno: /compact, poi `cvg checkpoint restore <plan_id>`
- Piani paralleli: max 2 task concorrenti totali

## Dipendenze tra piani
725 (O) ✅ → usato da P
719 (H0) + 720 (H0b) → 712 (H) → 713 (I) → 714 (J) ∥ 715 (K) → 716 (L)
                                              713 (I) + 719 (H0) → 721 (F2)
                                 712 (H) → 717 (M) → 718 (N)
                                 713 (I) + 725 (O) → P (da creare)

Quando un piano è completato, controlla quali dipendenti si sbloccano.

## A fine sessione (OBBLIGATORIO)
1. `cvg checkpoint save` per ogni piano toccato
2. Stampa: stato aggiornato di tutti i piani
3. Genera il prompt per la prossima sessione eseguendo:
   `./scripts/platform/master-plan-executor.sh`

## Recovery
`cvg checkpoint restore <plan_id>` → `cvg plan show <plan_id>` → riprendi dal primo task non-done.
"""

print("--- PROMPT (copia tutto sotto) ---\n")
print(prompt)
PYEOF
