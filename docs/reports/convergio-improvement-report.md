# Convergio Platform — Improvement Report
> Compiled from two independent sessions (Plan 738 execution). 28 March 2026.

## ETA Piano 738
- **Done**: 1/29 (T0-01 done, T1-01 submitted, T1-02+ in progress)
- **Claude attivo**: PID 79435 su macProM1, 18 minuti elapsed
- **Stima**: ~6-8 ore (29 task, singolo agent, non parallelizzato)

---

## 🔴 BUG CONFERMATI (da entrambe le sessioni)

| # | Bug | Dettaglio | Severità |
|---|-----|-----------|----------|
| B1 | `cvg review register --spec-file` crasha | `NOT NULL constraint failed: plan_reviews.plan_id`. Non puoi registrare una review prima di `cvg plan create`. Ma il workflow documentato dice review → create. **Workflow rotto.** | P0 |
| B2 | `/api/channels` non montato nel router | `api_channels.rs` esiste ma `routes/mod.rs` non lo monta. `cvg channel` CLI esiste ma il backend ritorna 404. | P1 |
| B3 | `/api/health/deep` non montato | `api_health_deep.rs` esiste ma non è wired nel router. | P2 |
| B4 | `cvg delegation status` → 404 | `POST /api/mesh/delegate` ritorna ok, ma `/api/delegation/{id}/progress` usa un ID diverso (plan_id vs delegation_id). Mismatch. | P1 |
| B5 | `coordinator/emit` → CRDT error | `crsql_internal_sync_bit` function missing. Il coordinator non può emettere eventi. **Blocca l'orchestrazione Ali.** | P0 |
| B6 | `/api/mesh/exec` SSH non risolve peers | Il fallback SSH usa il peer name come hostname raw (`ssh macProM1`), non legge `tailscale_ip` da peers.conf. Fallisce sempre. | P1 |
| B7 | Daemon binda su `127.0.0.1` non `0.0.0.0` | Il daemon non è raggiungibile da altri nodi mesh via Tailscale. Rompe la comunicazione daemon-to-daemon che `/api/mesh/exec` tenta prima dell'SSH fallback. | P1 |
| B8 | `cvg plan tree` non mostra wave names | L'output tree non include `wave_name` o `wave_id` nei dati — solo i task sono listati senza contesto wave. | P2 |
| B9 | **SSH peer naming è un casino** | Lo stesso peer ha 5+ nomi diversi: `macProM1` (peers.conf), `roberdandev-m1Pro` (ssh_alias), `Roberdans-MacBook-Pro-M1.local` (Bonjour/LAN), `roberdans-macbook-pro-m1` (Tailscale DNS), `100.106.173.118` (Tailscale IP), `10.0.0.2` (Thunderbolt). Il daemon usa `peer_ssh_alias()` che costruisce `user@tailscale_ip`, ma `/api/mesh/exec` usa il peer name raw come hostname. `~/.ssh/config` ha alias `m1Pro`, `m1Pro-tb`, `m1Pro-lan` con un ProxyCommand custom. Risultato: SSH funziona dalla shell ma fallisce dal daemon, o funziona via Tailscale ma non via LAN, etc. **Serve UN singolo resolver centralizzato** che tutti i componenti usano (daemon SSH, mesh exec, SSE delegate, scripts) con fallback chain: Thunderbolt → LAN → Tailscale → DNS. | P1 |

## 🟡 GAP DI DOCUMENTAZIONE (confermati + nuovi)

| # | Gap | Impatto |
|---|-----|---------|
| D1 | **Schema spec YAML non documentato** | Ho dovuto scoprire trial-and-error che: `type` accetta solo `bug/feature/fix/refactor/test/config/doc/chore`, `depends_on` deve essere stringa non array, `docs` non è un type valido (è `doc`). 3 tentativi di import falliti. |
| D2 | **Nessun OpenAPI spec per il daemon** | 200+ endpoint senza documentazione API. Ho dovuto leggere il Rust source per capire i field names, body format, response shapes. `specs/openapi.yaml` esiste ma è stale/incompleto. |
| D3 | **Mesh delegation non documentata nelle rules** | CLAUDE.md e le rules parlano solo di `copilot-worker.sh`. Non menzionano `POST /api/mesh/delegate`, `mesh-delegate-task.sh`, il flusso orchestratore Ali, o `delegate_to_peer()`. Un agent non sa come delegare. |
| D4 | **`cvg plan execution-tree` vs `cvg plan tree`** | Le rules riferiscono `execution-tree` ma il comando CLI è `tree`. Confusione. |
| D5 | **Telegram: env vars vs notifications.conf vs Keychain** | Tre posti diversi per le credenziali Telegram. Nessuna docs su quale è autoritativo. I script in `scripts/kernel/` usano Keychain, `scripts/mesh/` usano Keychain, ma l'env var `CONVERGIO_TELEGRAM_TOKEN` è documentata altrove. |
| D6 | **Nessuna guida "come delegare un piano a un peer"** | Il flusso completo (rsync → tmux → claude → completion hook → rsync back) non è documentato da nessuna parte. Ho dovuto leggere `executor.rs`, `mesh-delegate-task.sh`, `sse_delegate.rs` per capire i 3 diversi flussi di delegazione. |
| D7 | **peers.conf: `ssh_alias` non usato consistentemente** | `peer_ssh_alias()` costruisce `user@tailscale_ip`, ma `/api/mesh/exec` usa il peer name letterale come hostname SSH. Due risoluzioni diverse per lo stesso peer. |
| D8 | **Nessuna guida "cosa usare quando"** | Un agent non sa se deve usare `cvg plan validate` o lanciare un Thor agent, se deve usare `gh pr create` o un endpoint Convergio, se deve fare `git push` o `mesh-rsync`. Non c'è un **decision tree** o un `cvg help workflows` che dica: "per validare usa X, per pushare usa Y, per PR usa Z". Risultato: l'agent ricade su strumenti generici (gh, git, agent custom) invece di usare il platform. |
| D9 | **`cvg --help` non mostra tutti i comandi disponibili** | I subcommand come `cvg plan validate`, `cvg review`, `cvg delegation` non sono facilmente discoverable. Un `cvg commands` o `cvg cheatsheet` eviterebbe il problema. |

## 🔵 SUGGERIMENTI DI MIGLIORAMENTO

### Priorità Alta — Sblocca il mesh

| # | Suggerimento | Razionale |
|---|-------------|-----------|
| S1 | **Daemon deve bindare su `0.0.0.0:8420`** (o su Tailscale IP) | Senza questo, daemon-to-daemon HTTP non funziona. Il mesh è cieco. Flag `--bind 0.0.0.0` o config in peers.conf. |
| S2 | **`/api/mesh/exec` deve risolvere peers da peers.conf** | Usare `peer_ssh_alias()` (che già funziona per SSE delegate) anche nel fallback SSH di mesh/exec. Una riga di fix. |
| S3 | **Fix CRDT extension per coordinator** | `crsql_internal_sync_bit` mancante blocca Ali. Il coordinator è il cervello dell'orchestrazione. Senza di esso, la delegazione deve essere manuale. |
| S4 | **`cvg delegation` CLI completo** | Serve: `cvg delegation start <plan_id> --peer <peer>`, `cvg delegation status <plan_id>`, `cvg delegation cancel <plan_id>`. Oggi il CLI ha solo `status` e `list` (inspection). |

### Priorità Media — Developer Experience

| # | Suggerimento | Razionale |
|---|-------------|-----------|
| S5 | **`cvg plan template`** | Genera un YAML di esempio con tutti i campi required. Evita 3 tentativi di import. 5 minuti di lavoro, ore di risparmio. |
| S6 | **Messaggi di errore contestuali in `cvg plan import`** | "missing field id" → "wave[0] missing field 'id'". "CHECK constraint failed: type" → "task T1-03 has invalid type 'docs', valid: bug/feature/fix/refactor/test/config/doc/chore". |
| S7 | **`cvg review register` pre-plan** | Permettere review su spec-file PRIMA di creare il piano nel DB. Il workflow naturale è: scrivi spec → review → fix → create plan. Oggi devi: create plan → review → fix → reimport. |
| S8 | **OpenAPI auto-generated dal daemon** | Un `GET /api/docs` che serve lo schema OpenAPI generato dai route handlers axum. O almeno un `cvg api list` che mostra tutti gli endpoint con metodo/path/body. |
| S9 | **Planner skill genera YAML nel formato esatto** | La planner skill produce un formato che non è quello che `cvg plan import` accetta. Allineamento obbligatorio. |
| S10 | **Progress callback dal peer al coordinator** | Quando Claude su macProM1 completa un task, il coordinator dovrebbe ricevere un IPC callback. Oggi devo fare `cvg plan show 738` manualmente. Il `delegation-complete.sh` fa callback solo alla FINE, non per-task. |

### Priorità Bassa — Nice to Have

| # | Suggerimento | Razionale |
|---|-------------|-----------|
| S11 | **`cvg mesh ping <peer>` con SSH test** | Oggi `GET /api/mesh/ping/:peer` controlla solo heartbeat DB, non connettività reale. Un SSH check integrato aiuterebbe a diagnosticare prima di delegare. |
| S12 | **Telegram integrato nel daemon** | Invece di script bash per Telegram, un `POST /api/notify` che supporta Telegram nativamente. Il daemon ha già le credenziali. |
| S13 | **`cvg plan watch 738`** | Polling automatico con output live del progresso. Come `kubectl get pods -w`. |
| S14 | **Dashboard delegation view** | La web UI (convergio-web) dovrebbe mostrare le delegazioni attive con progresso real-time via WebSocket. |
| S15 | **`cvg mesh delegate` come CLI shortcut** | Wrapper per `mesh-delegate-task.sh` con auto-resoluzione peer da peers.conf, auto-prompt da spec YAML, auto-tmux session naming. |

## 🔄 PROBLEMI DI FLUSSO (dalla mia esperienza diretta)

### L'agent delegato non esegue Thor validation

Problema critico: Claude su macProM1 sta eseguendo task e committando, ma:
- **Non aggiorna `cvg task update`** → il coordinator non sa a che punto è
- **Non esegue `cvg plan validate`** dopo ogni wave → nessun Thor quality gate
- **Non manda notifiche Telegram** → siamo al buio
- **Nessun feedback loop** → se produce codice sbagliato, nessuno lo ferma

Il flusso Convergio prevede: executor → submitted → **Thor valida** → done.
Ma il delegatee non ha questa disciplina perché:
1. Il prompt di delegazione non include le istruzioni Thor esplicite
2. `mesh-delegate-task.sh` non inietta il workflow cvg nel prompt
3. `delegate_to_peer()` genera un prompt generico ("Execute Plan X, mark done via API")
4. Nessuno dei 3 flussi di delegazione menziona Thor

**Fix necessario**: Il prompt generato da `build_plan_prompt()` in `executor.rs` deve includere:
- `cvg task update <id> in_progress` prima di ogni task
- `cvg task update <id> submitted` dopo ogni task
- `cvg plan validate <plan_id>` dopo ogni wave
- `./scripts/notify-telegram.sh` per notifiche
- Le verify[] commands dal spec YAML

Oppure: il coordinator dovrebbe monitorare i commit sul peer e triggerare Thor remotamente.

### Il flusso di delegazione è frammentato in 3 implementazioni

1. **`sse_delegate.rs`** — SSH sincrono, blocca finché l'agent finisce. OK per task brevi, inutile per piani lunghi.
2. **`orchestrator/executor.rs`** — rsync + tmux + claude asincrono. Il flusso GIUSTO, ma richiede il coordinator Ali attivo (che è rotto per CRDT).
3. **`mesh-delegate-task.sh`** — Script bash che fa lo stesso dell'orchestratore. Funziona ma non è "Convergio API".

**Dovrebbe esserci UN SOLO flusso**, accessibile via:
- CLI: `cvg mesh delegate 738 --peer macProM1`
- API: `POST /api/mesh/delegate` (con modalità async/tmux)
- Orchestratore: Ali che chiama lo stesso endpoint

### Il ciclo review → plan è invertito

Il workflow documentato dice: requirements → spec → review → plan.
Ma `cvg review register` richiede `plan_id` (il piano deve già esistere nel DB).
**Servono due fasi**: review-on-spec (pre-DB) e review-on-plan (post-DB).

### Git push dal peer è problematico

Il peer (macProM1) non aveva credenziali GitHub. Ho dovuto:
1. Copiare il token gh via pipe SSH
2. Configurare git credential store
3. Settare gh auth login

**Il daemon dovrebbe gestire le credenziali mesh** — quando fai `cvg mesh delegate`, dovrebbe propagare le credenziali necessarie al peer (o usare il coordinator come proxy git).

## 📊 RIEPILOGO PRIORITÀ

| Priorità | Count | Azione |
|----------|-------|--------|
| P0 — Bloccante | 3 | Fix B1 (review), B5 (CRDT coordinator), Thor non eseguito su delegazione |
| P1 — Mesh rotto | 4 | Fix B2, B4, B6, B7 |
| P2 — UX/DX | 2 | Fix B3, B8 |
| Alta | 4 | S1-S4 (sblocca mesh) |
| Media | 6 | S5-S10 (developer experience) |
| Bassa | 5 | S11-S15 (nice to have) |
| Doc | 7 | D1-D7 (documentazione) |

**Total: 30 issue/suggerimenti** di cui 6 bloccanti/critici.
