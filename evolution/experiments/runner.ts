import type { AuditSink } from '../core/engine.js';
import type { PlatformAdapter } from '../core/types/adapter.js';
import type { Experiment, ExperimentResult, Proposal } from '../core/types/index.js';
import { CanaryMode, FullMode, ShadowMode } from './modes.js';

export interface ExperimentRunnerOptions {
  auditSink?: AuditSink;
  waitForMerge?: (prNumber: number) => Promise<void>;
}

export class ExperimentRunner {
  private readonly auditSink?: AuditSink;
  private readonly waitForMerge: (prNumber: number) => Promise<void>;

  constructor(options: ExperimentRunnerOptions) {
    this.auditSink = options.auditSink;
    this.waitForMerge = options.waitForMerge ?? (async () => {
      await new Promise<void>((resolve) => {
        setTimeout(resolve, 5000);
      });
    });
  }

  async run(experiment: Experiment, adapter: PlatformAdapter, proposal = this.syntheticProposal(experiment.proposalId)): Promise<ExperimentResult> {
    this.audit('experiment.start', {
      experimentId: experiment.id,
      mode: experiment.mode,
      adapter: adapter.name,
      proposalId: proposal.id,
    });

    try {
      const result = await this.dispatch(experiment, adapter, proposal);
      experiment.result = result;
      experiment.completedAt = Date.now();

      if (experiment.mode === 'Canary' && result.recommendation !== 'Apply') {
        await adapter.rollback(experiment.id);
        this.audit('experiment.rollback', { experimentId: experiment.id, reason: 'canary_result_rejected' });
      }

      this.audit('experiment.complete', {
        experimentId: experiment.id,
        recommendation: result.recommendation,
        delta: result.delta,
      });
      return result;
    } catch (error) {
      await adapter.rollback(experiment.id).catch(() => undefined);
      this.audit('experiment.rollback', { experimentId: experiment.id, reason: 'unhandled_error' });
      const message = error instanceof Error ? error.message : String(error);
      this.audit('experiment.error', { experimentId: experiment.id, message });
      throw error;
    }
  }

  private async dispatch(
    experiment: Experiment,
    adapter: PlatformAdapter,
    proposal: Proposal,
  ): Promise<ExperimentResult> {
    if (experiment.mode === 'Shadow') {
      return new ShadowMode().execute(experiment, adapter, proposal);
    }
    if (experiment.mode === 'Canary') {
      return new CanaryMode().execute(experiment, adapter, proposal);
    }
    return new FullMode(this.waitForMerge).execute(experiment, adapter, proposal);
  }

  private syntheticProposal(proposalId: string): Proposal {
    return {
      id: proposalId,
      hypothesis: `Synthetic proposal for ${proposalId}`,
      targetMetric: 'runtime.score',
      expectedDelta: { min: -0.05, max: -0.01 },
      successCriteria: 'score improves',
      failureCriteria: 'regression detected',
      blastRadius: 'SingleRepo',
      sourceType: 'Internal',
      status: 'Approved',
    };
  }

  private audit(action: string, input: Record<string, unknown>): void {
    if (!this.auditSink) {
      return;
    }
    this.auditSink({
      id: `audit-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      timestamp: Date.now(),
      actor: 'engine',
      action,
      input,
      output: {},
    });
  }
}
