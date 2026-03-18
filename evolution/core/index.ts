/**
 * Barrel export for the Evolution Engine core.
 * Consumers import from `@convergio/evolution-engine` which resolves here.
 */

export type {
  Metric,
  MetricFamily,
  MetricCollector,
  Proposal,
  BlastRadius,
  SourceType,
  ProposalStatus,
  Experiment,
  ExperimentMode,
  ExperimentResult,
  EvaluationResult,
  OptimizationOpportunity,
  AuditEntry,
  CapabilityProfile,
  CapabilityDelta,
  EvolutionConfig,
} from './types/index.js';

export type { PlatformAdapter } from './types/adapter.js';

export { createDefaultConfig, mergeConfig } from './config.js';
export { EvolutionEngine } from './engine.js';
export type { CycleSummary, AuditSink } from './engine.js';
export type { Evaluator } from './evaluators/evaluator.js';
export { BaseEvaluator } from './evaluators/base-evaluator.js';
export {
  OutcomeTracker,
  LatencyEvaluator,
  BundleEvaluator,
  AgentCostEvaluator,
  MeshEvaluator,
  WorkloadEvaluator,
} from './evaluators/index.js';
