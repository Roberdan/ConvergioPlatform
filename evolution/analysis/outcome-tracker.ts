import type { EvaluationResult } from '../core/types/index.js';
import { OutcomeTracker as CoreOutcomeTracker } from '../core/evaluators/outcome-tracker.js';

export interface OutcomeSummary {
  effectivenessDelta: number;
  improved: boolean;
}

export class OutcomeTracker {
  private readonly tracker: CoreOutcomeTracker;

  constructor(dbPath?: string) {
    this.tracker = new CoreOutcomeTracker({ dbPath });
  }

  record(proposalId: string, before: EvaluationResult, after: EvaluationResult): OutcomeSummary {
    this.tracker.record(proposalId, before, after);
    const roi = this.tracker.getROI(proposalId);
    return {
      effectivenessDelta: roi?.deltaScore ?? 0,
      improved: roi?.improved ?? false,
    };
  }
}
