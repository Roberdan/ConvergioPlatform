# Briefing — Fresh Claude Session

> Data: 29 Marzo 2026, 20:00 CET
> Versione: 19.5.0
> Autore: sessione precedente (Plan 756-758)

## Chi Sei

Sei Claude su ConvergioPlatform. Il daemon Rust è il cuore del sistema. Hai 69 agenti, un kernel Jarvis (Qwen 7B su M1 Pro), e un mesh P2P tra 2 nodi Mac.

## Cosa È Stato Fatto Oggi

| Plan | Versione | Cosa |
|---|---|---|
| 756 | v19.3.0 | 446→33 errori silenziosi, 126K token recuperati, wiring check automatico |
| 757 | v19.4.0 | 104→69 agenti consolidati, 13→2 file regole, CLAUDE.md ottimizzato |
| 758 | v19.5.0 | Memory CRUD API, Ali multi-dominio con 10 domain routing |

## PRIORITÀ ASSOLUTA: DB SYNC TRA NODI

**IL SYNC DEL DATABASE TRA M5MAX E M1 PRO NON FUNZIONA.**

Questo è il problema più grave. Senza sync:
- I piani creati su un nodo non appaiono sull'altro
- La delegation a M1 Pro fallisce
- Jarvis kernel non sa cosa sta succedendo
- Il mesh è di fatto rotto

### Nodi

| Nodo | IP Tailscale | SSH | Ruolo |
|---|---|---|---|
| M5Max | 100.89.245.79 | localhost | Coordinator |
| M1 Pro | 100.106.173.118 | roberdandev-m1Pro | Kernel (Jarvis) |

### Fix Già Applicati (ma non sufficienti)

- `6242e908` — rimosso double http:// in peer URLs
- `ee6cae3e` — detect local Tailscale IP, skip self in sync
- `de639ed7` — SQLite ALTER TABLE fix (function defaults → NULL)

### File Chiave

- `daemon/src/background_sync.rs` — loop principale
- `daemon/src/background_sync_http.rs` — fetch/send HTTP
- `daemon/src/server/api_sync.rs` — endpoint API
- `daemon/src/db/libsql_adapter.rs` — adapter timestamp-based
- `scripts/kernel/sync-db.sh` — fallback rsync
- `~/.claude/config/peers.conf` — registro peer

### Come Testare

```bash
# Su M5Max
curl -s http://localhost:8420/api/health
# Su M1 Pro
ssh roberdandev-m1Pro "curl -s http://localhost:8420/api/health"
# Crea piano test su M5Max, verifica su M1 Pro entro 60s
```

## IL BACKLOG COMPLETO

Lo spec YAML con tutti i task è in: `specs/master-backlog-v19.5.yaml`

| Wave | Task | Priorità | Cosa |
|---|---|---|---|
| W1 | T1-01 | P0 | **Fix DB sync tra nodi** |
| W2 | T2-01 | P1 | Fix evidence gate cache bug |
| W2 | T2-02 | P1 | Fix Telegram bot |
| W2 | T2-03 | P1 | Fix statusline sqlite3 violation |
| W3 | T3-01 | P2 | Enable notification channels |
| W3 | T3-02 | P2 | Add sync/notification telemetry |
| W4 | T4-01 | P2 | `/api/delegate/spawn` — daemon gestisce tmux |
| W4 | T4-02 | P2 | `/api/build` + `/api/test` — sostituisce digest scripts |
| W5 | T5-01 | P3 | Annotate 30 .ok() rimanenti |
| W5 | T5-02 | P3 | Auto-reaper processi orfani |
| W5 | T5-03 | P3 | Wire DS playbook in agent UI |
| W5 | T5-04 | P3 | Node self-provisioning al boot |
| W6 | TF-tests | — | Test completi |
| W6 | TF-doc | — | CHANGELOG, ADR-0125, v20.0.0 |

## COME ESEGUIRE

### Metodo 1: Plan Runner Autonomo (RACCOMANDATO)

```bash
# Importa e lancia — il runner gestisce tutto da solo
cvg plan create convergio "Master Backlog v19.5" --source-file specs/master-backlog-v19.5.yaml
cvg review register --plan-id <ID> plan-reviewer proceed
cvg plan import <ID> specs/master-backlog-v19.5.yaml
cvg plan start <ID>
tmux new-session -d -s Convergio -n plan-<ID> "claude-config/scripts/copilot-plan-runner.sh <ID>"
```

Il runner:
1. Chiama `GET /api/plan-db/execution-context/<ID>` per ogni task
2. Spawna un Claude/Copilot con contesto fresco
3. Loop fino a plan completo (max 50 iterazioni)
4. PreCompact hook salva checkpoint e ri-lancia se il contesto esplode

### Metodo 2: Manuale Per Task Critici

Per W1 (DB sync) potrebbe servire intervento manuale:
```bash
# SSH a M1 Pro per diagnosticare
ssh roberdandev-m1Pro
curl http://localhost:8420/api/health
echo $DASHBOARD_DB
ls -la ~/.claude/data/dashboard.db
```

## REGOLE IMPORTANTI

1. **Fail loud** — MAI `.ok()` o `let _ =` senza commento. Errori visibili sempre.
2. **Max 250 righe/file** — enforced da CI
3. **cargo check DEVE passare** prima di ogni commit
4. **Worktree per branch** — NON lavorare su main direttamente se altri agenti sono attivi
5. **Evidence gate** — ha un bug di cache. Se blocca: restart daemon (`pkill -f 'convergio-platform-daemon serve'`)
6. **No sqlite3 diretto** — usa `cvg` CLI o daemon API
7. **Agent identity** — `cvg agent start "claude-$(hostname -s)-$$"` all'inizio

## PROBLEMI NOTI

| Problema | Workaround |
|---|---|
| Evidence gate cache stale | Restart daemon per pulire cache |
| cargo_test timeout 90s→180s | Già fixato ma può servire di più per full test |
| Statusline chiama sqlite3 | Da fixare in T2-03 |
| 9+ processi copilot orfani | `pkill -f 'copilot.*yolo'` manuale |
| Telegram non risponde | Da diagnosticare in T2-02 |
| Notifiche tutte disabilitate | notifications.conf ha tutto a disabled |

## REFERENCE CHIAVE

| Doc | Path |
|---|---|
| Capabilities completo | claude-config/reference/operational/convergio-capabilities.md |
| DS Integration Playbook | claude-config/reference/operational/ds-integration-playbook.md |
| Hard enforcement rules | claude-config/rules/hard-enforcement.md |
| Best practices | claude-config/rules/best-practices.md |
| ADR-0122 Session Continuity | docs/adr/ADR-0122-session-continuity.md |
| ADR-0124 Fail-Loud | docs/adr/ADR-0124-fail-loud-policy.md |
| Plan DB Schema | claude-config/reference/operational/plan-db-schema.md |
| Core Workflow | claude-config/reference/operational/core-workflow.md |
