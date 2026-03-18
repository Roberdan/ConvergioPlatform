/**
 * Shared types for the Evolution Engine.
 * All adapters and core modules import from here.
 */

// ── Metric ────────────────────────────────────────────────────────────────────

/** Telemetry signal categories collected across platform targets. */
export type MetricFamily =
  | 'Runtime'
  | 'Mesh'
  | 'Database'
  | 'Workload'
  | 'Agent'
  | 'Build'
  | 'Bundle';

/**
 * A single telemetry reading with optional dimension labels.
 * Values are always numeric; semantics are encoded in `name` and `family`.
 */
export interface Metric {
  /** Dot-namespaced name, e.g. `http.p95_latency_ms` */
  name: string;
  /** Raw numeric value at collection time */
  value: number;
  /** Unix epoch milliseconds */
  timestamp: number;
  /** Arbitrary key-value dimensions (service, region, env, …) */
  labels: Record<string, string>;
  /** High-level grouping for routing and budget tracking */
  family: MetricFamily;
}

/**
 * Pluggable metric source that the engine polls on each evaluation cycle.
 * Adapters implement this to feed domain-specific telemetry into the core.
 */
export interface MetricCollector {
  /** Stable collector identifier, e.g. `lighthouse`, `vitest-coverage` */
  readonly id: string;
  /** Metric families this collector can produce */
  readonly families: readonly MetricFamily[];
  /** Gather metrics from the underlying source. */
  collect(): Promise<Metric[]>;
}

// ── Proposal ─────────────────────────────────────────────────────────────────

/**
 * How many repos/files a proposal may touch.
 * Used to gate approval requirements and rollout strategy.
 */
export type BlastRadius = 'SingleFile' | 'SingleRepo' | 'MultiRepo' | 'Ecosystem';

/** Where the optimisation idea originated. */
export type SourceType = 'Internal' | 'ExternalHypothesis' | 'ToolUpgrade';

/** Lifecycle state of a proposal through evaluation → deployment. */
export type ProposalStatus =
  | 'Draft'
  | 'Shadow'
  | 'Canary'
  | 'PendingApproval'
  | 'Approved'
  | 'Rejected'
  | 'Applied'
  | 'Reverted';

/**
 * An optimisation hypothesis that the engine evaluates and potentially applies.
 * Immutable once status transitions past Draft.
 */
export interface Proposal {
  /** Stable ID, format EVO-YYYYMMDD-NNNN */
  id: string;
  /** Human-readable hypothesis statement */
  hypothesis: string;
  /** Primary metric this proposal aims to improve */
  targetMetric: string;
  /** Expected relative delta range (negative = improvement for latency/cost) */
  expectedDelta: { min: number; max: number };
  /** Condition that marks the experiment a success */
  successCriteria: string;
  /** Condition that triggers automatic rollback */
  failureCriteria: string;
  /** Scope of impact — drives approval gate selection */
  blastRadius: BlastRadius;
  /** How this proposal was generated */
  sourceType: SourceType;
  /** Current lifecycle state */
  status: ProposalStatus;
  /** Optional SHA / PR reference once applied */
  appliedRef?: string;
}

// ── Experiment ────────────────────────────────────────────────────────────────

/** Deployment mode for a live experiment. */
export type ExperimentMode = 'Shadow' | 'Canary' | 'Full';

/**
 * Statistical outcome of a completed experiment run.
 */
export interface ExperimentResult {
  /** Bayesian confidence that the change is beneficial [0–1] */
  confidence: number;
  /** Two-tailed p-value from significance test */
  pValue: number;
  /** Engine recommendation based on result */
  recommendation: 'Apply' | 'Reject' | 'ExtendCanary' | 'Inconclusive';
  /** Observed delta on the target metric */
  delta: number;
  /** Any collateral metric regressions detected */
  sideEffects: Array<{ metric: string; delta: number }>;
}

/**
 * Full experiment lifecycle record linking proposal → before/after metrics.
 */
export interface Experiment {
  id: string;
  proposalId: string;
  mode: ExperimentMode;
  startedAt: number;
  completedAt?: number;
  beforeMetrics: Metric[];
  afterMetrics: Metric[];
  result?: ExperimentResult;
}

// ── Evaluation ────────────────────────────────────────────────────────────────

/**
 * A concrete optimisation opportunity surfaced by an evaluator.
 * Carries enough detail for the engine to auto-generate a Proposal.
 */
export interface OptimizationOpportunity {
  /** Short label, e.g. `tree-shake lodash`, `enable gzip` */
  title: string;
  /** Free-form explanation of the improvement */
  description: string;
  /** Estimated relative gain as a human-readable string, e.g. `-15% bundle size` */
  estimatedGain: string;
  /** Domain this opportunity belongs to */
  domain: string;
  /** Suggested blast radius for any resulting proposal */
  suggestedBlastRadius: BlastRadius;
}

/**
 * Output of an evaluator scanning one domain (latency, cost, quality, …).
 */
export interface EvaluationResult {
  /** Domain label, e.g. `latency`, `bundle_size`, `agent_cost` */
  domain: string;
  /** Detected regressions or anomalies */
  anomalies: Array<{ metric: string; severity: 'low' | 'medium' | 'high'; detail: string }>;
  /** Actionable opportunities discovered */
  opportunities: OptimizationOpportunity[];
  /** Composite health score [0–100] */
  score: number;
}

// ── Audit ─────────────────────────────────────────────────────────────────────

/**
 * Immutable audit log entry for every engine action.
 * Written before and after state-changing operations.
 */
export interface AuditEntry {
  id: string;
  timestamp: number;
  /** Actor identity: `engine`, `human:<github-login>`, `ci` */
  actor: string;
  action: string;
  input: Record<string, unknown>;
  output: Record<string, unknown>;
  /** Proposal this entry relates to, if any */
  proposalId?: string;
}

// ── Capability ────────────────────────────────────────────────────────────────

/**
 * Snapshot of an LLM provider's capabilities at a point in time.
 * Used by the engine to select the right model for each task.
 */
export interface CapabilityProfile {
  provider: string;
  model: string;
  contextWindow: number;
  tools: string[];
  costPerToken: { input: number; output: number };
  /** Supported feature flags, e.g. `streaming`, `vision`, `code-interpreter` */
  features: string[];
  lastChecked: number;
}

// ── Capability Delta ──────────────────────────────────────────────────────────

/**
 * Difference between two CapabilityProfile snapshots.
 * Produced when re-scanning a provider reveals changes (new model,
 * price change, dropped feature).
 */
export interface CapabilityDelta {
  /** Provider being compared */
  provider: string;
  /** Model being compared */
  model: string;
  /** Fields that changed between snapshots */
  changes: Array<{
    field: keyof CapabilityProfile;
    before: unknown;
    after: unknown;
  }>;
  /** When the delta was detected (Unix epoch ms) */
  detectedAt: number;
}

// ── Config ────────────────────────────────────────────────────────────────────

/**
 * Top-level engine configuration loaded from `evolution.config.json`.
 */
export interface EvolutionConfig {
  /** Cron expression for daily lightweight scans */
  dailyCron: string;
  /** Cron expression for weekly deep evaluation runs */
  weeklyCron: string;
  /** Per-domain LLM spend caps in USD */
  budgetLimits: Record<string, number>;
  /** Max proposals per day/week to prevent runaway automation */
  rateLimits: { proposalsPerDay: number; proposalsPerWeek: number };
  /** Maximum cumulative storage for metrics snapshots, in MB */
  storageLimitMb: number;
}
