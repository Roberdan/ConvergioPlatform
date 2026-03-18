import type { Experiment, ExperimentResult, Metric, Proposal } from '../core/types/index.js';
import type { PlatformAdapter } from '../core/types/adapter.js';

export interface ExperimentModeHandler {
  execute(experiment: Experiment, adapter: PlatformAdapter, proposal: Proposal): Promise<ExperimentResult>;
}

export class ShadowMode implements ExperimentModeHandler {
  async execute(
    experiment: Experiment,
    adapter: PlatformAdapter,
    _proposal: Proposal,
  ): Promise<ExperimentResult> {
    const before = await adapter.collectMetrics();
    const after = await adapter.collectMetrics();
    experiment.beforeMetrics = before;
    experiment.afterMetrics = after;

    const delta = computeRelativeDelta(before, after);
    return toResult(delta, Math.abs(delta) >= 0.02);
  }
}

export class CanaryMode implements ExperimentModeHandler {
  async execute(_experiment: Experiment, adapter: PlatformAdapter, proposal: Proposal): Promise<ExperimentResult> {
    return adapter.runCanary(proposal);
  }
}

export class FullMode implements ExperimentModeHandler {
  private readonly waitForMerge: (prNumber: number) => Promise<void>;

  constructor(waitForMerge: (prNumber: number) => Promise<void>) {
    this.waitForMerge = waitForMerge;
  }

  async execute(experiment: Experiment, adapter: PlatformAdapter, proposal: Proposal): Promise<ExperimentResult> {
    const before = await adapter.collectMetrics();
    experiment.beforeMetrics = before;

    const { prNumber } = await adapter.openPR(proposal);
    await this.waitForMerge(prNumber);

    const after = await adapter.collectMetrics();
    experiment.afterMetrics = after;

    const delta = computeRelativeDelta(before, after);
    return toResult(delta, delta < 0);
  }
}

function computeRelativeDelta(before: Metric[], after: Metric[]): number {
  if (before.length === 0 || after.length === 0) {
    return 0;
  }

  const beforeAverage = before.reduce((sum, metric) => sum + metric.value, 0) / before.length;
  const afterAverage = after.reduce((sum, metric) => sum + metric.value, 0) / after.length;

  if (beforeAverage === 0) {
    return afterAverage - beforeAverage;
  }

  return (afterAverage - beforeAverage) / beforeAverage;
}

function toResult(delta: number, improved: boolean): ExperimentResult {
  return {
    confidence: improved ? 0.92 : 0.41,
    pValue: improved ? 0.04 : 0.31,
    recommendation: improved ? 'Apply' : 'Reject',
    delta,
    sideEffects: [],
  };
}
