# Convergio Platform — Strategic Vision & Architecture Spec

> Generated from strategic design session, March 2026.
> This document is the authoritative reference for all architectural decisions.
> Claude Code agents working on ConvergioPlatform MUST read this before making design choices.

---

## 1. THE THESIS

**Convergio is the agentic business factory — the Y Combinator where everyone is accepted.**

AI models will become commodity. The value is in everything above the model: orchestration, governance, checklists, memory, integrations, lifecycle management, voice, security, and marketplace — everything that turns a problem into a functioning business.

**Tagline:** "Give it a problem. It builds the business."

**Positioning:** NVIDIA builds factories that produce intelligence. Convergio builds factories that produce businesses. Y Combinator democratized startup creation for the elite. Convergio democratizes it for anyone who understands a problem.

---

## 2. CORE ARCHITECTURE DECISIONS

### 2.1 Agentic Memory (replaces raw SQL interaction)

**Decision: SQL is NOT removed. It is DEMOTED to physical storage layer under an Intent Graph.**

Agents never write SQL. They use five primitives:

```
remember(fact, context, confidence, ttl)  → Store knowledge with temporal decay
recall(query, scope, freshness)           → Semantic retrieval (vector + graph)
forget(pattern, reason)                   → Active deletion with audit trail
share(knowledge, recipients, scope)       → Inter-agent knowledge transfer
attest(fact_id, agent_id, evidence)       → Provenance chain entry
```

**Storage engines (three pillars under the Intent Graph):**

| Engine | Technology | Purpose |
|--------|-----------|---------|
| Structured store | SQLite WAL (existing) | Plans, tasks, waves, heartbeats, operational state |
| Vector store | LanceDB (Rust-native, embeddable, no server) | Embeddings for semantic recall() |
| Blob store | Content-addressed (SHA-256 hash as key) | Files, artifacts, logs |

**New daemon modules:**
```
daemon/src/memory/
├── mod.rs           // MemoryEngine trait
├── intent_graph.rs  // Intent nodes, causal edges, confidence scores
├── remember.rs      // remember() implementation
├── recall.rs        // Hybrid retrieval (exact + semantic)
├── forget.rs        // Deletion with audit
├── share.rs         // Inter-agent knowledge transfer
├── attest.rs        // Provenance chain
├── lance.rs         // LanceDB integration
└── blob.rs          // Content-addressed blob store
```

**Migration strategy:**
- Sprint 1: Build five primitive MCP tools over existing SQLite. No storage change.
- Sprint 2: Add LanceDB. Dual-write: structured → SQLite, embeddings → LanceDB.
- Sprint 3: Add blob store. Intent graph becomes single source of truth.

### 2.2 Checklist Engine (replaces workflows)

**Decision: Replace workflow-driven agent execution with Checklist Manifesto discipline.**

Inspired by Atul Gawande's "The Checklist Manifesto." Two types:

**DO-CONFIRM (default, 80% of tasks):** Agent executes from skill memory. At pause points, daemon runs checklist against output. Blocks if killer items unchecked. Respects agent autonomy.

**READ-DO (novel/high-risk tasks):** Agent receives items one at a time, executes each before the next. Used when recall() confidence < 0.7 or task is destructive.

**Rules (from Gawande):**
- 5-9 killer items maximum per checklist
- Simple, exact wording in professional language
- Fits on one screen (YAML file)
- Must include communication checkpoint (share() before proceeding)
- Checklists evolve via evolution engine feedback loop
- Never static — regularly refined from failure telemetry

**New daemon modules:**
```
daemon/src/checklist/
├── mod.rs           // ChecklistEngine trait
├── types.rs         // DoConfirm, ReadDo, CommunicationChecklist
├── runner.rs        // Execute checklist at pause points
├── evolution.rs     // Telemetry → refinement loop
└── registry.rs      // Per-agent checklist registry (YAML)
```

**Example checklist (YAML):**
```yaml
agent: rex-reviewer
type: do_confirm
pause_point: post_execution
killer_items:
  - id: security_scan
    check: "Security vulnerability assessment completed"
    severity: critical
  - id: test_coverage
    check: "Test coverage above threshold"
    severity: high
  - id: constitution_alignment
    check: "Output respects CONSTITUTION.md articles"
    severity: critical
  - id: share_findings
    check: "Findings shared via share() to requesting agent"
    severity: high
    type: communication
  - id: confidence_attestation
    check: "Confidence score attested via attest()"
    severity: medium
fallback_mode: read_do
```

### 2.3 Local Inference (MLX + Apple Intelligence)

**Decision: Three-tier inference strategy with zero marginal cost for 95% of operations.**

| Tier | Technology | Use | % of calls |
|------|-----------|-----|-----------|
| Light | Apple Foundation Models (3B, free via framework) | Intent parsing, entity extraction, summarization | 80% |
| Medium | MLX local (Qwen3-32B-Q4 or similar) | Code gen, analysis, complex reasoning | 15% |
| Heavy | MLX local (70B-Q4) or cloud fallback | Strategic reasoning, Ali orchestrator decisions | 5% |

**Apple Intelligence integration points:**
- Foundation Models framework: free on-device 3B LLM via Swift, guided generation, tool calling
- App Intents: Siri can invoke Convergio agents ("Hey Siri, ask Convergio for MirrorBuddy status")
- App Intents domains: File Management, Mail, Spreadsheets, Presentations — all available to agents

**New daemon modules:**
```
daemon/src/inference/
├── mod.rs           // InferenceRouter
├── mlx.rs           // MLX subprocess management (mlx-lm HTTP server on Unix socket)
├── apple_fm.rs      // Apple Foundation Models bridge (via Swift FFI or subprocess)
├── cloud.rs         // Cloud API fallback (Anthropic, OpenAI)
├── router.rs        // Model selection logic (tier routing)
└── registry.rs      // Model registry (YAML config)
```

**Model config:**
```yaml
inference:
  models:
    reasoning: { path: "~/.convergio/models/qwen3-72b-q4", engine: mlx, max_ctx: 8192 }
    execution: { path: "~/.convergio/models/qwen3-32b-q4", engine: mlx, max_ctx: 4096 }
    embedding: { path: "~/.convergio/models/nomic-embed", engine: mlx }
    code:      { path: "~/.convergio/models/qwen-coder-32b", engine: mlx, max_ctx: 8192 }
    light:     { engine: apple_fm }  # Apple Foundation Models, free
  routing:
    - agents: [ali, thor, antonio, satya, amy]
      model: reasoning
    - agents: [task-executor, rex, dario, omri]
      model: execution
    - agents: ["*"]
      model: execution
  fallback:
    provider: anthropic
    model: claude-sonnet-4-20250514
    trigger: confidence_below_0.6
```

### 2.4 Integration Architecture (Four Capability Rings)

**Decision: MCP client in daemon, not custom integrations. 5,000+ MCP servers instantly available.**

All four rings share a single Capability trait with unified permission model and checklist governance.

**Ring 1 — Local Machine Control (Agent's hands):**
Filesystem (scoped MCP), shell execution, macOS Shortcuts, Git, AppleScript/osascript, clipboard, screenshot.

**Ring 2 — Browser & Web (Agent's eyes):**
Playwright headless browser (MCP sidecar), web search (SearXNG/Brave), scraping, URL→markdown, vision via local MLX multimodal.

**Ring 3 — SaaS & Cloud (Agent's voice):**
Google Drive/Gmail/Calendar, Slack, Notion, GitHub, Stripe, Linear — all via existing MCP servers. OAuth tokens in macOS Keychain.

**Ring 4 — Legacy & Enterprise (Real world):**
SMTP/IMAP (Rust lettre), PostgreSQL/MySQL, Excel/CSV, PDF, SOAP/XML, FTP/SFTP, S3, OCR (MLX), webhook receiver (existing axum).

**New daemon modules:**
```
daemon/src/mcp/
├── mod.rs           // MCP client manager
├── connector.rs     // Discover, authenticate, route MCP tool calls
├── registry.rs      // Configured MCP servers (YAML)
└── proxy.rs         // Security proxy (rate limit, logging, header stripping)

daemon/src/capability/
├── mod.rs           // Capability trait
├── ring.rs          // CapabilityRing enum (Local, Browser, SaaS, Legacy)
├── permission.rs    // Per-agent capability grants
└── audit.rs         // Unified audit logging
```

**Capability trait:**
```rust
pub trait Capability: Send + Sync {
    fn name(&self) -> &str;
    fn ring(&self) -> CapabilityRing;
    fn risk_level(&self) -> RiskLevel; // Read | Write | Destructive | External
    fn checklist_type(&self) -> ChecklistMode; // DoConfirm | ReadDo
    async fn execute(&self, agent: &AgentId, action: &Action, ctx: &ExecutionContext) -> Result<ActionResult, CapabilityError>;
    fn audit_entry(&self, action: &Action, result: &ActionResult) -> AuditRecord;
}
```

### 2.5 Security (Five Perimeters, Zero Trust)

| Perimeter | What | Implementation |
|-----------|------|---------------|
| P1 Hardware | macOS FileVault, Secure Enclave, Keychain, SIP | Leverage Apple, don't reinvent |
| P2 Daemon | Rust memory safety, HMAC-SHA256 IPC, AES-GCM at rest | Already exists, extend to new modules |
| P3 Agent Sandbox | Per-agent capability ACL, token budget, kill switch, checklist gates | New: daemon/src/security/ |
| P4 Data | AES-256 all stores, mTLS mesh, immutable provenance chain | New: extend db module |
| P5 External | MCP proxy, URL allowlist, egress firewall, speaker verification | New: daemon/src/security/ |

**New daemon modules:**
```
daemon/src/security/
├── mod.rs           // SecurityEngine orchestrator
├── capability.rs    // Per-agent capability grants (YAML)
├── sandbox.rs       // Agent execution sandbox
├── vault.rs         // Keychain-backed secret storage
├── audit.rs         // Immutable provenance chain
├── egress.rs        // Outbound network firewall
├── budget.rs        // Token budget enforcer
└── kill_switch.rs   // Instant halt + rollback
```

### 2.6 Voice Control (100% Local)

**Pipeline:** Core Audio mic → Silero VAD (~2ms) → Wake word ("Hey Convergio") → Whisper/Qwen3-ASR via MLX → Local LLM intent → Ali orchestrator → CosyVoice TTS (~150ms) → Core Audio speaker

**End-to-end latency:** ~800ms on M3 Max

**Three modes:**
- Hands-free command: brief instructions while away from keyboard
- Conversational: deep interaction with Ali, follow-ups, full dialogue
- Accessibility: voice-only mode for users with motor disabilities (VoiceOver bridge)

**Speaker verification:** voiceprint enrollment (3 sentences), local biometric auth for destructive commands.

**New daemon modules:**
```
daemon/src/voice/
├── mod.rs           // VoicePipeline orchestrator
├── vad.rs           // Silero VAD via CoreML bridge
├── wake.rs          // Wake word detection
├── asr.rs           // Whisper/Qwen3-ASR via MLX subprocess
├── intent.rs        // LLM intent extraction
├── tts.rs           // CosyVoice via MLX subprocess
├── speaker.rs       // Voiceprint enrollment + verification
└── accessibility.rs // VoiceOver bridge, full voice-only mode
```

---

## 3. THE EIGHT-PHASE AUTONOMOUS BUSINESS LIFECYCLE

This is Convergio's core value proposition. From problem to profit, autonomously:

| Phase | Name | Mode | Key Agents | What Happens |
|-------|------|------|------------|-------------|
| 1 | Understand | HUMAN | Ali, Omri, Fiona | Founder describes problem. Ali decomposes, Omri sizes market, Fiona analyzes competitors |
| 2 | Build | AUTO | Task-executor, Rex, Dario, Luca | Agents code, review, debug, audit. Workspace layer handles git. Checklists enforce quality |
| 3 | Business shell | AUTO | Davide, Task-executor, Amy, Elena | Buy domain, deploy website, configure Stripe, set up email, generate legal docs |
| 4 | Launch | AUTO+APPROVE | Antonio, Andrea, Ali | Write launch copy, Product Hunt, email campaigns, social. Human approves once |
| 5 | Sell & onboard | AUTO | Andrea, Amy, Ava | Handle leads, qualify, process payments, welcome sequences, onboard users |
| 6 | Support & fix | AUTO | Dario, Rex, Andrea, Thor | Monitor errors, triage bugs, write/review/deploy fixes, handle support tickets |
| 7 | Monitor & optimize | AUTO | Ava, Amy, Ali, Satya | Track MRR/churn/NPS. Weekly voice briefing. Strategic recommendations |
| 8 | Evolve & grow | AUTO+VISION | Ali, Omri, Antonio, Task-executor | Evolution engine proposes features from feedback. Build, A/B test, deploy winners |

**The human's total involvement:**
- Phase 1: Describe the problem (5 minutes)
- Phase 4: Approve launch plan (15 minutes)
- Phase 7: Listen to weekly briefing (5 minutes/week)
- Phase 8: Provide strategic vision when needed

---

## 4. ECONOMICS & MONETIZATION

### Startup cost: $8,400/year all-in
- Mac mini M4: $599 (one-time)
- Convergio Pro: $49/mo = $588/yr
- Domain + hosting: $200/yr
- SaaS tools (Stripe, email): $600/yr

### Revenue model
| Tier | Price | What |
|------|-------|------|
| Core | $0 | Open-source. 12 agents, checklist engine, MLX, mesh, MCP client. MPL-2.0 forever |
| Pro | $49/mo | All 89 agents, premium packs, voice, dashboard, analytics |
| Business | $199/mo | Pro + marketplace publishing (80/20 rev share), custom training, SLA |
| Marketplace | 15% take | Creators sell agent packs. Stripe Connect instant payouts |
| ConvergioEDU | $19/mo | Educational vertical, DSA-accessible, student→founder pipeline |

### Billing architecture
- Stripe via MCP server
- Offline-first: signed JWT license validated locally by daemon
- Graceful degradation to Core on lapse (no data loss, no hostage-taking)
- Marketplace: Stripe Connect for instant creator payouts

---

## 5. STRATEGIC ROADMAP

| Phase | When | What | Milestones |
|-------|------|------|-----------|
| 1 Sovereign Foundation | Now → Q4 2026 | Ship local-first platform for technical solopreneurs | 1K installs, 10 community agents, MVP marketplace |
| 2 Problem-Solver's Platform | 2027 | NL onboarding, agent marketplace, non-technical founders | 10K founders, 500 agent packs, $1M ARR |
| 3 One-Person Company OS | 2028 | Full business stack, vertical stacks, agent-to-agent marketplace | 100K founders, 50 verticals, $10M+ ARR |
| 4 Inclusive Economy | 2029-2030 | Accessibility-first, global reach, FightTheStroke integration | 1M+ founders, impact metrics |

---

## 6. COMPETITIVE POSITIONING

**Convergio has ZERO competitors in its actual category.** All competitors compete on a different axis:

| Capability | Convergio | CrewAI | LangGraph | AutoGen | OpenAI SDK |
|-----------|-----------|--------|-----------|--------|------------|
| Local-first / offline | Native | No | Partial | Partial | No |
| Mesh / multi-node | Tailscale P2P | No | No | Network | No |
| Agentic memory | Intent graph | Short-term | State only | Chat history | Thread |
| Apple Silicon native | MLX + Rust | No | No | No | No |
| Self-improving | Evolution engine | No | No | Limited | No |
| Data sovereignty | Full local | Cloud | Depends | Depends | Cloud |
| Agent count | 89 specialized | User-defined | User-defined | User-defined | User-defined |
| Governance | Constitutional | No | No | No | No |
| Checklist-driven | Gawande method | Workflow | Graph | Conversation | Function |
| Full business lifecycle | End-to-end | Build only | Build only | Build only | Build only |
| Zero API cost | MLX local | No | No | No | No |

---

## 7. KEY REFERENCES

- CONSTITUTION.md — Non-negotiable governance articles
- AgenticManifesto.md — Design philosophy ("Human purpose. AI momentum.")
- CLAUDE.md — Existing platform conventions and commands
- AGENTS.md — Full catalog of 89 agents across 12 domains

---

## 8. THE MANIFESTO (closing)

> "AI models become commodity. The value layer is everything above the model
> that turns a problem into a business. Convergio is that layer."

> "Give it a problem. It builds the business."

> "Empathy with execution is transformation."

Copyright © 2026 Roberto D'Angelo. All rights reserved.
