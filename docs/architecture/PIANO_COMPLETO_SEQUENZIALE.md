# Convergio Platform — Piano Completo Sequenziale

> 24 Marzo 2026 | Ogni fase e verificabile indipendentemente.
> Ogni piano ha criteri di accettazione binari (passa/non passa).

---

## STATO DI PARTENZA (v17.1.0)

### Cosa esiste e funziona su main

**Daemon Rust (56K LOC, 107 moduli)**
- 90 API endpoint (HTTP + SSE + WS) su :8420
- Mesh P2P (Tailscale + HMAC-SHA256 + CRDT sync)
- IPC protocol (auth, budget, models, skills, conflicts, locks, worktrees)
- Workspace layer (git ops, 8 quality gates meccanici, release agent)
- TUI ratatui (9 views, drill-down, WS real-time, chat)
- Validation meccanica (credentials, line count, patterns, commit lint)
- Hook enforcement (secret detection, branch protection)

**Multi-Model (gia operativo)**
- Cloud routing: Claude Opus/Sonnet/Haiku + GPT-5.3 Codex/5.4/Codex-mini + Gemini 3 Pro
- Model routing spec con decision tree per fase (plan/execute/validate/explore)
- Phase routing: Opus (planning/validation) + Codex (execution) + Sonnet (coordinator) + Haiku (exploration)
- Task-type routing: CodeGen/Refactor/Architecture/Testing/Documentation/SecurityReview

**Local Inference (infra pronta, non integrata nel daemon)**
- oMLX backend per macOS Apple Silicon (:8321)
- vLLM backend per Linux NVIDIA
- llama.cpp backend per CPU
- LiteLLM proxy (:4000) come unified interface OpenAI-compatible
- Model catalog: Qwen3.5-27B (72.4% SWE-bench, 20GB), Qwen3-Coder-30B (256K ctx, 22GB)
- Scripts: setup, download, start/stop, test
- IPC router nel daemon: task analysis, capability scoring, fallback chain (local -> cloud -> degraded)
- Model registry DB: ipc_model_registry, ipc_node_capabilities, ipc_subscriptions

**Evolution Engine (TypeScript, 127 file)**
- Proposals, experiments, ROI tracking, canary/shadow deployment
- Model intelligence: traccia versioni Claude/Copilot/OpenCode/Gemini

**Governance**
- Constitution 10 articoli (3 NON-NEGOTIABLE)
- Thor 10 validation gates
- 89 agenti specializzati in 12 domini
- Persuasion guardrails (10 blocked patterns)

**SwiftUI Command Center** (Plan F, su branch 706)

### Cosa NON esiste ancora

- `daemon/src/checklist/` — Checklist Engine
- `daemon/src/mcp/` — MCP Client
- `daemon/src/memory/` — Agentic Memory (5 primitive)
- `daemon/src/security/` — Security Perimeters (P3-P5)
- `daemon/src/inference/` — Inference Router nel daemon (l'infra esterna c'e, il modulo daemon no)
- `daemon/src/voice/` — Voice Pipeline
- Apple Foundation Models integration

---

## PIANO 0: CHIUSURA F + G

**Obiettivo**: portare su main tutto il lavoro gia completato ma non mergiato.

### Task

| # | Azione | Verifica |
|---|---|---|
| 0.1 | Verificare worktree 706 e 708-W1 post-reboot | `git worktree list` mostra tutti accessibili |
| 0.2 | Completare 4 task residui Plan F (706) | `cvg plan show 706` → 50/50 task done |
| 0.3 | Thor validation wave per wave su Plan G (708) | `cvg plan validate` per ogni wave |
| 0.4 | Rebase 706 e 708 su main | `git rebase origin/main` senza conflitti |
| 0.5 | Squash merge tutte le wave | PR merged, CI green |
| 0.6 | Cleanup worktree e branch | `git worktree list` mostra solo main |
| 0.7 | Cleanup workspace temporanei | Nessun ws-* in `git worktree list` |

### Criteri di accettazione
- [ ] `cvg plan show 706` → status: done
- [ ] `cvg plan show 708` → status: done
- [ ] `git worktree list` → solo main
- [ ] `cargo check` → 0 errori
- [ ] `cargo test` → tutti passano
- [ ] VERSION.md = 18.0.0

---

## PIANO 1: HARDENING NASA (Plan H)

**Obiettivo**: il daemon rispetta le 12 regole Holzmann prima di crescere.

### Task

| # | Regola NASA | Azione | Verifica |
|---|---|---|---|
| 1.1 | 02 — Bound loops | Audit tutti `loop`/`while` nel daemon. Aggiungere `MAX_ITERATIONS` esplicito | `grep -r 'loop\|while ' daemon/src/` → ogni occorrenza ha cap |
| 1.2 | 04 — Function size | Configurare clippy lint `too_many_lines` a 60 | `cargo clippy` → 0 warning su function size |
| 1.3 | 06 — No error swallowing | Audit `.ok()`, `unwrap_or_default()` su Result con logica business | Grep → zero occorrenze non giustificate |
| 1.4 | 07 — Narrow state | Spezzare `AppState` in sotto-state tipizzati | AppState ha sotto-struct (MeshState, PlanState, etc.) |
| 1.5 | 01 — Nesting depth | Aggiungere gate max 2 livelli nesting | `cargo clippy` → 0 warning su nesting |
| 1.6 | 05 — Assertions | Aggiungere `debug_assert!` alle funzioni pubbliche chiave | Almeno 1 assert per funzione pubblica in moduli critici |
| 1.7 | 08 — Side effects naming | Convention: funzioni con I/O nominate `write_`/`send_`/`delete_`/`fetch_` | ADR documenta la convention |

### Criteri di accettazione
- [ ] `cargo clippy` → 0 warning (inclusi lint custom)
- [ ] `cargo test` → tutti passano
- [ ] ADR-0112 documenta le regole NASA applicate
- [ ] Zero `.ok()` su Result con business logic (audit report)
- [ ] VERSION.md = 18.1.0

---

## PIANO 2: CHECKLIST ENGINE (Plan I)

**Obiettivo**: la disciplina diventa meccanica. Thor si evolve.

### Nuovi file

```
daemon/src/checklist/
  mod.rs           — ChecklistEngine trait + pub interface
  types.rs         — DoConfirm, ReadDo, ChecklistItem, ChecklistResult
  runner.rs        — Esegue checklist ai pause point
  evolution.rs     — Telemetria feedback loop
  registry.rs      — Carica YAML per-agent dal filesystem
```

### Task

| # | Azione | Verifica |
|---|---|---|
| 2.1 | Definire `ChecklistEngine` trait e tipi | `cargo check` compila, test tipi |
| 2.2 | Implementare runner DO-CONFIRM | Test: checklist con 5 item, 1 fail → blocca |
| 2.3 | Implementare runner READ-DO | Test: item sequenziali, blocca al primo fail |
| 2.4 | Implementare registry YAML | Test: carica checklist da file, valida schema |
| 2.5 | Creare primo pack: NASA Rules | 12 killer items YAML per thor-validator |
| 2.6 | Integrare con Thor gate 10 | Thor chiama checklist prima dei gate di reasoning |
| 2.7 | CLI `cvg checklist run <agent> <task>` | Comando funziona, output strutturato |
| 2.8 | API `POST /api/checklist/run` | Endpoint funziona, JSON response |
| 2.9 | Feedback telemetria → evolution engine | Checklist result scritto in DB, leggibile da evolution |

### Criteri di accettazione
- [ ] `cvg checklist run thor-validator test-task` → output con pass/fail per item
- [ ] NASA pack: 12 killer items definiti in YAML
- [ ] Thor gate 10 chiama checklist meccanica prima di reasoning
- [ ] `cargo test` → test checklist tutti verdi
- [ ] ADR-0113 documenta il Checklist Engine
- [ ] VERSION.md = 19.0.0

---

## PIANO 3: MCP CLIENT + CAPABILITY RINGS (Plan J)

**Obiettivo**: il daemon parla con il mondo esterno. Business lifecycle fasi 3-4-5.

### Nuovi file

```
daemon/src/mcp/
  mod.rs           — MCP client manager
  connector.rs     — Connect, authenticate, route tool calls a MCP server
  registry.rs      — Server configurati (YAML)
  proxy.rs         — Security proxy (rate limit, logging, sanitization)

daemon/src/capability/
  mod.rs           — Capability trait
  ring.rs          — CapabilityRing enum (Local, Browser, SaaS, Legacy)
  permission.rs    — Per-agent capability grants (YAML ACL)
  audit.rs         — Unified audit logging
```

### Task

| # | Azione | Verifica |
|---|---|---|
| 3.1 | Definire `Capability` trait Rust | `cargo check` compila |
| 3.2 | Implementare 4 ring con enum | Test: ring classification corretto |
| 3.3 | MCP connector: connect a un server MCP locale | Test: connect a filesystem MCP, list tools |
| 3.4 | MCP registry: YAML config per server | Test: carica config, valida schema |
| 3.5 | MCP proxy: rate limit + audit log | Test: rate limit blocca dopo N call |
| 3.6 | Per-agent permission grants | Test: agente A puo Ring 1-2, agente B solo Ring 1 |
| 3.7 | Integrare con checklist (Ring 3-4 = ReadDo) | Test: capability Destructive triggera READ-DO checklist |
| 3.8 | CLI `cvg capability list` | Comando funziona, mostra ring/permessi |
| 3.9 | API `POST /api/capability/execute` | Endpoint funziona con audit trail |
| 3.10 | Demo: Stripe MCP → creare un prodotto | End-to-end: agente chiama Stripe via MCP |

### Criteri di accettazione
- [ ] `cvg capability list` → mostra 4 ring con permessi per-agent
- [ ] Almeno 3 MCP server configurati (filesystem, github, uno SaaS)
- [ ] Audit trail per ogni capability execution
- [ ] Checklist integrata: Ring 3-4 richiedono READ-DO
- [ ] `cargo test` → test MCP tutti verdi
- [ ] ADR-0114 documenta MCP Client + Capability Rings
- [ ] VERSION.md = 20.0.0

**Questo e il punto di svolta. Primo prodotto end-to-end dimostrabile.**

---

## PIANO 4: AGENTIC MEMORY (Plan K)

**Obiettivo**: gli agenti ricordano, condividono, attestano.

### Nuovi file

```
daemon/src/memory/
  mod.rs           — MemoryEngine trait
  remember.rs      — remember(fact, context, confidence, ttl)
  recall.rs        — recall(query, scope, freshness) — exact + semantic
  forget.rs        — forget(pattern, reason) — audit trail
  share.rs         — share(knowledge, recipients, scope)
  attest.rs        — attest(fact_id, agent_id, evidence)
  lance.rs         — LanceDB integration (Sprint 2)
  blob.rs          — Content-addressed blob store (Sprint 3)
```

### Sprint incrementali (NASA #9 — one layer of magic)

| Sprint | Cosa | Storage |
|---|---|---|
| 4A | 5 primitive come thin wrapper su SQLite esistente | Solo SQLite |
| 4B | Aggiungere LanceDB per embedding. Dual-write | SQLite + LanceDB |
| 4C | Blob store. Intent Graph single source of truth | SQLite + LanceDB + Blob |

### Task (Sprint 4A — minimo verificabile)

| # | Azione | Verifica |
|---|---|---|
| 4.1 | `remember()` su SQLite | Test: salva fatto, recupera per ID |
| 4.2 | `recall()` exact match su SQLite | Test: query esatta ritorna fatto |
| 4.3 | `forget()` con audit | Test: cancella fatto, audit trail presente |
| 4.4 | `share()` tra agenti | Test: agente A condivide, agente B vede |
| 4.5 | `attest()` provenance | Test: fatto ha catena di attestazione |
| 4.6 | CLI `cvg memory recall <query>` | Comando funziona |
| 4.7 | API `POST /api/memory/{remember,recall,forget,share,attest}` | 5 endpoint funzionanti |
| 4.8 | TTL decay: fatti scaduti vengono marcati | Test: fatto con TTL 0 → marcato expired |

### Criteri di accettazione (Sprint 4A)
- [ ] 5 primitive funzionanti su SQLite
- [ ] CLI e API operativi
- [ ] Test per ogni primitiva
- [ ] Zero dipendenze nuove (solo SQLite esistente)
- [ ] ADR-0115 documenta Memory primitives
- [ ] VERSION.md = 21.0.0

### Criteri Sprint 4B (separato)
- [ ] LanceDB aggiunto come dipendenza Cargo
- [ ] `recall()` con semantic search funzionante
- [ ] Embedding generato da modello locale (nomic-embed via LiteLLM)
- [ ] VERSION.md = 21.1.0

### Criteri Sprint 4C (separato)
- [ ] Blob store content-addressed (SHA-256)
- [ ] `remember()` puo salvare file/artefatti
- [ ] VERSION.md = 21.2.0

---

## PIANO 5: SECURITY PERIMETERS (Plan L)

**Obiettivo**: prima di esporre MCP Ring 3-4 in produzione, serve sicurezza per-agente.

### Nuovi file

```
daemon/src/security/
  mod.rs           — SecurityEngine orchestrator
  capability.rs    — Per-agent ACL (YAML)
  sandbox.rs       — Agent execution sandbox (resource limits)
  vault.rs         — macOS Keychain-backed secret storage
  audit.rs         — Immutable provenance chain
  egress.rs        — Outbound network firewall (URL allowlist)
  budget.rs        — Token budget enforcer (estende ipc/budget)
  kill_switch.rs   — Instant halt + rollback per agente
```

### Task

| # | Azione | Verifica |
|---|---|---|
| 5.1 | Per-agent ACL YAML | Test: agente con permesso Ring 2 non puo Ring 3 |
| 5.2 | Sandbox con resource limits | Test: agente che eccede timeout viene killato |
| 5.3 | Keychain vault per secret | Test: store/retrieve secret via macOS Keychain |
| 5.4 | Immutable audit chain | Test: audit entry non modificabile dopo scrittura |
| 5.5 | Egress firewall con URL allowlist | Test: URL non in allowlist → blocked |
| 5.6 | Token budget enforcer | Test: agente che eccede budget → blocked |
| 5.7 | Kill switch per agente | Test: `cvg agent kill <id>` → halt immediato + rollback |
| 5.8 | Integrare con MCP proxy | Test: MCP call a Ring 4 passa per tutti i gate security |

### Criteri di accettazione
- [ ] Ogni agente ha ACL esplicito (nessun default permissivo)
- [ ] Kill switch funziona in <1s
- [ ] Audit chain immutabile con proof
- [ ] `cvg security audit <agent>` → report permessi/budget/attivita
- [ ] ADR-0116 documenta Security Perimeters
- [ ] VERSION.md = 22.0.0

---

## PIANO 6: INFERENCE ROUTER NEL DAEMON (Plan M)

**Obiettivo**: unificare l'infra locale esistente (oMLX/vLLM/llama.cpp/LiteLLM) + cloud routing nel daemon come modulo first-class. Aggiungere Apple Foundation Models.

### Cosa esiste gia (da integrare, non riscrivere)

| Componente | Path | Stato |
|---|---|---|
| oMLX backend macOS | `scripts/llm/convergio-llm.sh` | Funzionante |
| vLLM backend Linux | `scripts/llm/convergio-llm.sh` | Funzionante |
| llama.cpp backend CPU | `scripts/llm/convergio-llm.sh` | Funzionante |
| LiteLLM proxy | `config/llm/litellm.yaml` | Funzionante su :4000 |
| Model catalog | `config/llm/models.yaml` | Qwen3.5-27B, Coder-30B |
| LLM client (stream) | `daemon/src/server/llm_client.rs` | Claude + LiteLLM |
| IPC model registry | `daemon/src/ipc/models/` | DB + probes Ollama/LM Studio |
| IPC router dispatch | `daemon/src/ipc/router/dispatch.rs` | Task analysis + scoring + fallback |
| Cloud routing spec | `model-routing-spec.md` | Claude/GPT/Gemini decision tree |
| Model intelligence | `evolution/agents/model-registry.ts` | Version tracking + cost savings |

### Nuovi file (quello che manca)

```
daemon/src/inference/
  mod.rs           — InferenceRouter (unified entry point)
  tier.rs          — Three-tier logic: Light/Medium/Heavy
  apple_fm.rs      — Apple Foundation Models bridge (Swift FFI o subprocess)
  health.rs        — Health check tutti i backend (oMLX, vLLM, llama.cpp, LiteLLM, cloud)
  metrics.rs       — Latenza, throughput, costo per tier
  config.rs        — YAML config per tier routing
```

### I 3+1 tier

| Tier | Tecnologia | Uso | % call | Costo |
|---|---|---|---|---|
| **Light** | Apple Foundation Models (3B, on-device) | Intent parsing, entity extraction, summarization | 70% | $0 |
| **Medium** | oMLX/vLLM/llama.cpp via LiteLLM (Qwen3.5-27B) | Code gen, analysis, reasoning medio | 20% | $0 |
| **Heavy** | Cloud multi-model (Claude/GPT/Gemini via routing spec) | Strategic reasoning, planning, validation | 10% | API cost |
| **Embedding** | nomic-embed via MLX locale | Semantic recall per memory module | On-demand | $0 |

### Task

| # | Azione | Verifica |
|---|---|---|
| 6.1 | `InferenceRouter` che wrappa LiteLLM + cloud routing esistenti | Test: route task semplice → local, task complesso → cloud |
| 6.2 | Tier classification logic | Test: intent parsing → Light, code gen → Medium, planning → Heavy |
| 6.3 | Apple FM bridge (subprocess o Swift FFI) | Test: intent extraction via Apple FM su macOS |
| 6.4 | Health check tutti i backend | `cvg inference status` → mostra stato ogni backend |
| 6.5 | Metrics collection (latenza, costo, throughput) | API `/api/inference/metrics` funzionante |
| 6.6 | Fallback automatico (locale down → cloud) | Test: stop oMLX → request fallisce → retry via cloud |
| 6.7 | Config YAML per tier routing per-agent | Test: ali → Heavy only, explorer → Light preferred |
| 6.8 | Integrare con IPC router esistente | dispatch.rs usa InferenceRouter invece di logica diretta |

### Criteri di accettazione
- [ ] `cvg inference status` → mostra tutti i backend con health
- [ ] `cvg inference route "analyze this code"` → mostra tier selezionato + rationale
- [ ] Apple FM funziona per intent parsing su macOS (graceful skip su Linux)
- [ ] Fallback automatico testato (locale → cloud)
- [ ] Metriche per tier visibili in TUI/API
- [ ] Zero regressione: tutto il routing cloud esistente continua a funzionare
- [ ] ADR-0117 documenta Inference Router + tier strategy
- [ ] VERSION.md = 23.0.0

---

## PIANO 7: VOICE CONTROL (Plan N)

**Obiettivo**: input alternativo hands-free, 100% locale.

### Nuovi file

```
daemon/src/voice/
  mod.rs           — VoicePipeline orchestrator
  vad.rs           — Silero VAD via CoreML (~2ms)
  wake.rs          — "Hey Convergio" wake word
  asr.rs           — Whisper via MLX subprocess (~300ms)
  intent.rs        — LLM intent extraction (via InferenceRouter tier Light)
  tts.rs           — CosyVoice via MLX subprocess (~150ms)
  speaker.rs       — Voiceprint enrollment + verification
  accessibility.rs — VoiceOver bridge, voice-only mode
```

### Task

| # | Azione | Verifica |
|---|---|---|
| 7.1 | VAD: Silero via CoreML | Test: rileva speech vs silenzio |
| 7.2 | Wake word: "Hey Convergio" | Test: attivazione su wake word |
| 7.3 | ASR: Whisper via MLX | Test: audio → testo con WER < 10% |
| 7.4 | Intent: via InferenceRouter Light tier | Test: "mostrami lo stato dei piani" → intent strutturato |
| 7.5 | TTS: CosyVoice via MLX | Test: testo → audio naturale |
| 7.6 | Pipeline completa: mic → risposta | Test: end-to-end < 1s su M3 Max |
| 7.7 | Speaker verification | Test: voce autorizzata → ok, altra voce → rejected |
| 7.8 | VoiceOver bridge | Test: voice-only mode accessibile |
| 7.9 | CLI `cvg voice start` | Comando avvia pipeline |

### Criteri di accettazione
- [ ] End-to-end < 1s su M3 Max
- [ ] 100% locale (zero chiamate cloud)
- [ ] Speaker verification funzionante
- [ ] 3 modi: command, conversational, accessibility
- [ ] ADR-0118 documenta Voice Pipeline
- [ ] VERSION.md = 24.0.0

---

## RIEPILOGO VISUALE

```
v17.1.0 (oggi)
│
├── PIANO 0: Chiudi F+G ————————————————> v18.0.0
│   Merge 23 wave. UI pronta.
│
├── PIANO 1: Hardening NASA ————————————> v18.1.0
│   12 regole. Clippy lint. Daemon pulito.
│
├── PIANO 2: Checklist Engine ——————————> v19.0.0
│   Disciplina meccanica. Thor evolve. YAML pack.
│
├── PIANO 3: MCP Client ———————————————> v20.0.0  *** PUNTO DI SVOLTA ***
│   4 ring. 5000+ MCP server. Business lifecycle 3-4-5.
│
├── PIANO 4: Agentic Memory ———————————> v21.0.0 (4A) → v21.2.0 (4C)
│   5 primitive. SQLite → +LanceDB → +Blob.
│
├── PIANO 5: Security Perimeters ——————> v22.0.0
│   5 perimetri. Per-agent sandbox. Kill switch.
│
├── PIANO 6: Inference Router —————————> v23.0.0
│   Unifica local (oMLX/vLLM/llama.cpp) + cloud (Claude/GPT/Gemini)
│   + Apple FM. 3 tier. Multi-model multi-provider.
│
└── PIANO 7: Voice ————————————————————> v24.0.0
    100% locale. Hands-free. Accessibility.
```

### Stack multi-modello finale (v23.0.0+)

```
                         ┌─────────────────────────────────────────┐
                         │         InferenceRouter (daemon)         │
                         │   tier classification + fallback chain   │
                         └────────┬──────────┬──────────┬──────────┘
                                  │          │          │
                    ┌─────────────▼──┐  ┌────▼────┐  ┌─▼──────────────┐
                    │  LIGHT (free)  │  │ MEDIUM  │  │  HEAVY (cloud)  │
                    │ Apple FM (3B)  │  │ (free)  │  │ multi-provider  │
                    │ on-device      │  │ oMLX    │  │                 │
                    │ intent parsing │  │ vLLM    │  │ Claude Opus     │
                    │ summarization  │  │ llama   │  │ Claude Sonnet   │
                    │ entity extract │  │ via     │  │ GPT-5.3 Codex   │
                    └────────────────┘  │ LiteLLM │  │ GPT-5.4         │
                                        │ :4000   │  │ Gemini 3 Pro    │
                                        │         │  │ Codex-mini      │
                                        │ Qwen3.5 │  │ Haiku           │
                                        │ Coder30B│  │                 │
                                        │ Codestrl│  │ Phase routing   │
                                        └─────────┘  │ Task routing    │
                                                      │ Budget routing  │
                                                      └─────────────────┘
```

### Principi multi-modello

1. **Nessun vendor lock-in**: ogni tier ha alternative. Cloud = Claude OR GPT OR Gemini.
2. **Local-first**: 90% delle call (Light+Medium) a costo zero. Cloud solo per heavy reasoning.
3. **Fallback automatico**: locale down → cloud. Cloud down → degraded mode (IPC router esistente).
4. **Per-agent routing**: Ali/Thor → Heavy (cloud). Explorer → Light (Apple FM). Executor → Medium (local).
5. **Budget-aware**: IPC budget enforcer gia esiste. InferenceRouter lo rispetta.
6. **Platform-agnostic**: oMLX su macOS, vLLM su NVIDIA, llama.cpp su CPU. Stessa interfaccia.

---

## DIPENDENZE TRA PIANI

```
Piano 0 (F+G) ─── prerequisito per tutto

Piano 1 (Hardening) ─── prerequisito per Piano 2
Piano 2 (Checklist) ─── prerequisito per Piano 3
Piano 3 (MCP) ──┬── prerequisito per Piano 5 (Security protegge MCP)
                └── punto di svolta mercato

Piano 4 (Memory) ─── indipendente dopo Piano 2 (usa checklist)
Piano 5 (Security) ─── dopo Piano 3 (protegge capability esterne)
Piano 6 (Inference) ─── indipendente dopo Piano 1 (puo parallelizzare con 3-4-5)
Piano 7 (Voice) ─── dopo Piano 6 (usa InferenceRouter per intent)
```

**Parallelizzazione possibile**: Piano 4 (Memory) e Piano 6 (Inference) possono procedere in parallelo con Piano 3 (MCP) e Piano 5 (Security), dopo che Piano 2 (Checklist) e chiuso.

---

*24 Marzo 2026. Ogni piano ha criteri binari. Nessun piano si chiude senza evidenza.*
