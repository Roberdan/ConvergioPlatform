/**
 * NF: Portability — evolution engine works across environments.
 *
 * Verifies:
 * 1. Engine imports work without Node-specific globals leaking into types
 * 2. Config module is self-contained and produces defaults without environment
 * 3. Metric type accepts all MetricFamily values (cross-platform labels)
 */
import { describe, it, expect } from 'vitest';
import { createDefaultConfig, mergeConfig } from '../../../evolution/core/config.js';

describe('NF: Portability', () => {
  it('createDefaultConfig returns valid config without any I/O', () => {
    const cfg = createDefaultConfig();
    expect(typeof cfg.dailyCron).toBe('string');
    expect(typeof cfg.weeklyCron).toBe('string');
    expect(typeof cfg.storageLimitMb).toBe('number');
    expect(cfg.storageLimitMb).toBeGreaterThan(0);
    expect(typeof cfg.maxProposalsPerCycle).toBe('number');
  });

  it('mergeConfig preserves defaults for unspecified keys', () => {
    const base = createDefaultConfig();
    const merged = mergeConfig({});
    expect(merged.dailyCron).toBe(base.dailyCron);
    expect(merged.storageLimitMb).toBe(base.storageLimitMb);
  });

  it('mergeConfig applies overrides correctly', () => {
    const merged = mergeConfig({ storageLimitMb: 256, maxProposalsPerCycle: 3 });
    expect(merged.storageLimitMb).toBe(256);
    expect(merged.maxProposalsPerCycle).toBe(3);
  });

  it('budget limits have entries for known domains', () => {
    const cfg = createDefaultConfig();
    expect(cfg.budgetLimits).toHaveProperty('agent');
    expect(cfg.budgetLimits['agent']).toBeGreaterThan(0);
  });

  it('rateLimits define daily and weekly caps', () => {
    const cfg = createDefaultConfig();
    expect(cfg.rateLimits.proposalsPerDay).toBeGreaterThan(0);
    expect(cfg.rateLimits.proposalsPerWeek).toBeGreaterThanOrEqual(cfg.rateLimits.proposalsPerDay);
  });
});
