/**
 * Default Evolution Engine configuration.
 *
 * Values are conservative production defaults — override per-deployment
 * via `evolution.config.json` at the repository root.
 */

import type { EvolutionConfig } from './types/index.js';

/** Daily scan: 06:00 UTC weekdays */
const DEFAULT_DAILY_CRON = '0 6 * * 1-5';

/** Weekly deep eval: Sunday 02:00 UTC */
const DEFAULT_WEEKLY_CRON = '0 2 * * 0';

/**
 * Sensible budget defaults (USD) per evaluation domain.
 * Keeps LLM spend predictable during autonomous operation.
 */
const DEFAULT_BUDGET_LIMITS: Record<string, number> = {
  latency: 5.0,
  bundle: 3.0,
  agent: 10.0,
  accessibility: 2.0,
  security: 5.0,
  general: 5.0,
};

/**
 * Returns a deep-copied default EvolutionConfig.
 * Safe to mutate — each call produces a fresh object.
 */
export function createDefaultConfig(): EvolutionConfig {
  return {
    dailyCron: DEFAULT_DAILY_CRON,
    weeklyCron: DEFAULT_WEEKLY_CRON,
    budgetLimits: { ...DEFAULT_BUDGET_LIMITS },
    rateLimits: {
      proposalsPerDay: 5,
      proposalsPerWeek: 15,
    },
    storageLimitMb: 512,
    maxProposalsPerCycle: 5,
  };
}

/**
 * Merges a partial override into the default config.
 * Nested objects are shallow-merged one level deep.
 */
export function mergeConfig(
  overrides: Partial<EvolutionConfig>,
): EvolutionConfig {
  const base = createDefaultConfig();
  return {
    dailyCron: overrides.dailyCron ?? base.dailyCron,
    weeklyCron: overrides.weeklyCron ?? base.weeklyCron,
    budgetLimits: { ...base.budgetLimits, ...overrides.budgetLimits },
    rateLimits: { ...base.rateLimits, ...overrides.rateLimits },
    storageLimitMb: overrides.storageLimitMb ?? base.storageLimitMb,
    maxProposalsPerCycle:
      overrides.maxProposalsPerCycle ?? base.maxProposalsPerCycle,
  };
}
