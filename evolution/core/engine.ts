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
  AuditEntry,
  EvolutionConfig,
} from './types/index.js';
import type { PlatformAdapter } from './types/adapter.js';
import type { Evaluator } from './evaluators/evaluator.js';
import type { AggregatedPoint } from '../telemetry/aggregation.js';
import { createDefaultConfig, mergeConfig } from './config.js';
import { EvaluatorRegistry } from './evaluators/registry.js';

/** Callback signature for audit log subscribers. */
export type AuditSink = (entry: AuditEntry) => void;

type MetricStoreLike = {
  aggregate(options: { from: number; to: number }): AggregatedPoint[];
};

export class EvolutionEngine {
  private readonly config: EvolutionConfig;
  private readonly adapters: ReadonlyArray<PlatformAdapter>;
  private readonly evaluators: ReadonlyArray<Evaluator>;
  private readonly metricStore?: MetricStoreLike;
  private readonly auditSinks: AuditSink[] = [];
  private cycleCount = 0;

  constructor(opts: {
    adapters: PlatformAdapter[];
    evaluators?: Evaluator[];
    config?: Partial<EvolutionConfig>;
    metricStore?: MetricStoreLike;
  }) {
    this.adapters = opts.adapters;
    this.evaluators = opts.evaluators ?? new EvaluatorRegistry().getAll();
    this.metricStore = opts.metricStore;
    this.config = opts.config ? mergeConfig(opts.config) : createDefaultConfig();
  }

  onAudit(sink: AuditSink): void {
    this.auditSinks.push(sink);
  }

  getConfig(): Readonly<EvolutionConfig> {
    return { ...this.config };
  }

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

  private async checkHealth(): Promise<PlatformAdapter[]> {
    const results = await Promise.allSettled(
      this.adapters.map(async (a) => {
        const health = await a.healthCheck();
        this.audit('adapter.health', { adapter: a.name, ...health });
        return health.healthy ? a : null;
      }),
    );
    return results
      .filter((r): r is PromiseFulfilledResult<PlatformAdapter | null> => r.status === 'fulfilled')
      .map((r) => r.value)
      .filter((a): a is PlatformAdapter => a !== null);
  }

  private async collectMetrics(adapters: PlatformAdapter[]): Promise<Metric[]> {
    const batches = await Promise.allSettled(adapters.map((a) => a.collectMetrics()));
    const all: Metric[] = [];
    for (const batch of batches) {
      if (batch.status === 'fulfilled') all.push(...batch.value);
    }
    this.audit('metrics.collected', { count: all.length });
    return all;
  }

  private historyFromStore(metrics: Metric[]): AggregatedPoint[] {
    if (!this.metricStore || metrics.length === 0) return [];
    const timestamps = metrics.map((metric) => metric.timestamp);
    return this.metricStore.aggregate({ from: Math.min(...timestamps), to: Math.max(...timestamps) });
  }

  private async evaluate(metrics: Metric[]): Promise<EvaluationResult[]> {
    if (this.evaluators.length === 0) {
      return [{ domain: 'baseline', anomalies: [], opportunities: [], score: metrics.length > 0 ? 75 : 0 }];
    }

    const history = this.historyFromStore(metrics);
    const evaluations = await Promise.all(
      this.evaluators.map(async (evaluator) => {
        const evaluatorMetrics = metrics.filter((metric) => evaluator.metricFamilies.includes(metric.family));
        const evaluatorHistory = history.filter((point) => evaluator.metricFamilies.includes(point.family));
        return evaluator.evaluate(evaluatorMetrics, evaluatorHistory);
      }),
    );

    const weightedScore = this.computeCompositeScore(evaluations);
    this.audit('evaluations.composite', {
      domains: evaluations.map((evaluation) => evaluation.domain),
      score: weightedScore,
    });

    return evaluations;
  }

  private computeCompositeScore(results: EvaluationResult[]): number {
    if (results.length === 0) return 0;
    const weighted = results.map((result) => ({
      score: result.score,
      weight: Math.max(1, result.anomalies.length + result.opportunities.length),
    }));
    const weightSum = weighted.reduce((sum, item) => sum + item.weight, 0);
    const scoreSum = weighted.reduce((sum, item) => sum + item.score * item.weight, 0);
    return Math.round(scoreSum / weightSum);
  }

  private generateProposals(evaluations: EvaluationResult[], cycleId: number): Proposal[] {
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

  private async runExperiments(adapters: PlatformAdapter[], proposals: Proposal[]): Promise<Experiment[]> {
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
      try {
        sink(entry);
      } catch {
        // never let a sink crash the engine
      }
    }
  }
}

export interface CycleSummary {
  cycleId: number;
  startedAt: number;
  completedAt: number;
  metricsCollected: number;
  evaluations: EvaluationResult[];
  proposalsGenerated: number;
  experimentsRun: number;
  experiments: Experiment[];
}
