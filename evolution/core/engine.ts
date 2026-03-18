/**
 * EvolutionEngine — the standalone orchestrator.
 *
 * Lifecycle: collect metrics → evaluate → propose → experiment → report.
 * Adapters provide platform-specific I/O; the engine owns the control flow.
 */

import type {
  Metric,
  EvaluationResult,
  Proposal,
  ProposalStatus,
  Experiment,
  ExperimentResult,
  AuditEntry,
  EvolutionConfig,
} from './types/index.js';
import type { PlatformAdapter } from './types/adapter.js';
import { createDefaultConfig, mergeConfig } from './config.js';

/** Callback signature for audit log subscribers. */
export type AuditSink = (entry: AuditEntry) => void;

/**
 * Core Evolution Engine.
 *
 * Framework-agnostic, zero external dependencies.
 * Wire adapters + evaluators at construction time;
 * call {@link run} to execute one full optimisation cycle.
 */
export class EvolutionEngine {
  private readonly config: EvolutionConfig;
  private readonly adapters: ReadonlyArray<PlatformAdapter>;
  private readonly auditSinks: AuditSink[] = [];
  private cycleCount = 0;

  constructor(opts: {
    adapters: PlatformAdapter[];
    config?: Partial<EvolutionConfig>;
  }) {
    this.adapters = opts.adapters;
    this.config = opts.config
      ? mergeConfig(opts.config)
      : createDefaultConfig();
  }

  /** Register an audit log consumer (e.g. file writer, event bus). */
  onAudit(sink: AuditSink): void {
    this.auditSinks.push(sink);
  }

  /** Current engine configuration (read-only copy). */
  getConfig(): Readonly<EvolutionConfig> {
    return { ...this.config };
  }

  /**
   * Execute one full optimisation cycle across all registered adapters.
   *
   * Steps:
   * 1. Health-check every adapter; skip unhealthy ones.
   * 2. Collect metrics from healthy adapters.
   * 3. Evaluate metrics to find anomalies and opportunities.
   * 4. Generate proposals from opportunities (respecting rate limits).
   * 5. Run canary experiments for approved proposals.
   * 6. Report results and write audit entries.
   *
   * @returns Summary of the completed cycle.
   */
  async run(): Promise<CycleSummary> {
    const cycleId = ++this.cycleCount;
    const startedAt = Date.now();

    this.audit('cycle.start', { cycleId });

    const healthyAdapters = await this.checkHealth();
    const metrics = await this.collectMetrics(healthyAdapters);
    const evaluations = await this.evaluate(metrics);
    const proposals = this.generateProposals(evaluations, cycleId);
    const experiments = await this.runExperiments(healthyAdapters, proposals);

    const summary: CycleSummary = {
      cycleId,
      startedAt,
      completedAt: Date.now(),
      metricsCollected: metrics.length,
      evaluations,
      proposalsGenerated: proposals.length,
      experimentsRun: experiments.length,
      experiments,
    };

    this.audit('cycle.complete', { cycleId, summary });
    return summary;
  }

  /** Filter adapters to only those reporting healthy. */
  private async checkHealth(): Promise<PlatformAdapter[]> {
    const results = await Promise.allSettled(
      this.adapters.map(async (a) => {
        const health = await a.healthCheck();
        this.audit('adapter.health', { adapter: a.name, ...health });
        return health.healthy ? a : null;
      }),
    );
    return results
      .filter(
        (r): r is PromiseFulfilledResult<PlatformAdapter | null> =>
          r.status === 'fulfilled',
      )
      .map((r) => r.value)
      .filter((a): a is PlatformAdapter => a !== null);
  }

  /** Collect metrics from all healthy adapters in parallel. */
  private async collectMetrics(
    adapters: PlatformAdapter[],
  ): Promise<Metric[]> {
    const batches = await Promise.allSettled(
      adapters.map((a) => a.collectMetrics()),
    );
    const all: Metric[] = [];
    for (const batch of batches) {
      if (batch.status === 'fulfilled') all.push(...batch.value);
    }
    this.audit('metrics.collected', { count: all.length });
    return all;
  }

  /**
   * Evaluate collected metrics.
   * Placeholder: real evaluators will be plugged in via constructor in T0-03+.
   */
  private async evaluate(metrics: Metric[]): Promise<EvaluationResult[]> {
    const result: EvaluationResult = {
      domain: 'baseline',
      anomalies: [],
      opportunities: [],
      score: metrics.length > 0 ? 75 : 0,
    };
    return [result];
  }

  /** Convert evaluation opportunities into draft proposals (rate-limited). */
  private generateProposals(
    evaluations: EvaluationResult[],
    cycleId: number,
  ): Proposal[] {
    const proposals: Proposal[] = [];
    const { proposalsPerDay } = this.config.rateLimits;
    const dateStamp = new Date().toISOString().slice(0, 10).replace(/-/g, '');

    for (const evaluation of evaluations) {
      for (const opp of evaluation.opportunities) {
        if (proposals.length >= proposalsPerDay) break;
        const seq = String(proposals.length + 1).padStart(4, '0');
        const proposal: Proposal = {
          id: `EVO-${dateStamp}-${seq}`,
          hypothesis: opp.description,
          targetMetric: `${evaluation.domain}.score`,
          expectedDelta: { min: -0.05, max: -0.20 },
          successCriteria: `${evaluation.domain} score improves >= 5%`,
          failureCriteria: 'Any regression > 2% on guarded metrics',
          blastRadius: opp.suggestedBlastRadius,
          sourceType: 'Internal',
          status: 'Draft' as ProposalStatus,
        };
        proposals.push(proposal);
      }
    }
    this.audit('proposals.generated', { cycleId, count: proposals.length });
    return proposals;
  }

  /** Run canary experiments on approved proposals. */
  private async runExperiments(
    adapters: PlatformAdapter[],
    proposals: Proposal[],
  ): Promise<Experiment[]> {
    const approved = proposals.filter((p) => p.status === 'Approved');
    const experiments: Experiment[] = [];

    for (const proposal of approved) {
      for (const adapter of adapters) {
        const exp: Experiment = {
          id: `exp-${proposal.id}-${adapter.name}`,
          proposalId: proposal.id,
          mode: 'Canary',
          startedAt: Date.now(),
          beforeMetrics: [],
          afterMetrics: [],
        };
        try {
          exp.result = await adapter.runCanary(proposal);
          exp.completedAt = Date.now();
        } catch (err) {
          const msg = err instanceof Error ? err.message : String(err);
          this.audit('experiment.failed', { id: exp.id, error: msg });
          await adapter.rollback(exp.id).catch(() => {});
        }
        experiments.push(exp);
      }
    }
    return experiments;
  }

  /** Write an audit entry to all registered sinks. */
  private audit(action: string, data: Record<string, unknown>): void {
    const entry: AuditEntry = {
      id: `audit-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      timestamp: Date.now(),
      actor: 'engine',
      action,
      input: data,
      output: {},
    };
    for (const sink of this.auditSinks) {
      try { sink(entry); } catch { /* never let a sink crash the engine */ }
    }
  }
}

/** Summary returned after each optimisation cycle. */
export interface CycleSummary {
  /** Monotonically increasing cycle counter */
  cycleId: number;
  /** Unix epoch ms when cycle started */
  startedAt: number;
  /** Unix epoch ms when cycle completed */
  completedAt: number;
  /** Total metrics collected across all adapters */
  metricsCollected: number;
  /** Per-domain evaluation results */
  evaluations: EvaluationResult[];
  /** Number of proposals generated this cycle */
  proposalsGenerated: number;
  /** Number of experiments executed */
  experimentsRun: number;
  /** Full experiment records */
  experiments: Experiment[];
}
