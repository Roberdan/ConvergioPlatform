import type { EvaluationResult, Experiment } from './types';

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
