/**
 * Barrel export for the Evolution Engine core.
 * Consumers import from `@convergio/evolution-engine` which resolves here.
 */

export type {
  Metric,
  MetricFamily,
  Proposal,
  BlastRadius,
  SourceType,
  ProposalStatus,
  Experiment,
  ExperimentMode,
  ExperimentResult,
  EvaluationResult,
  AuditEntry,
  CapabilityProfile,
  EvolutionConfig,
} from './types/index.js';

export type { PlatformAdapter } from './types/adapter.js';
