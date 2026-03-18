/**
 * NF: Cost Control — evolution budget enforcement.
 */
import { describe, it, expect } from 'vitest';
import { readFileSync, existsSync } from 'fs';
import { join } from 'path';
import { RateLimiter } from '../../../evolution/core/guardrails/rate-limiter.js';
import { readDailyBudget, DailyRunner } from '../../../evolution/core/cadence/daily-runner.js';
import type { EvolutionEngine } from '../../../evolution/core/engine.js';

const REPO_ROOT = join(import.meta.dirname, '..', '..', '..');
const CONFIG_PATH = join(REPO_ROOT, 'config', 'platform_config.json');

describe('NF: Cost Control', () => {
  it('platform_config.json has evolution.daily_budget_usd', () => {
    expect(existsSync(CONFIG_PATH)).toBe(true);
    const cfg = JSON.parse(readFileSync(CONFIG_PATH, 'utf-8')) as Record<string, unknown>;
    const ev = cfg['evolution'] as Record<string, unknown>;
    expect(typeof ev['daily_budget_usd']).toBe('number');
    expect(ev['daily_budget_usd'] as number).toBeGreaterThan(0);
  });

  it('readDailyBudget returns positive value from config', () => {
    expect(readDailyBudget(CONFIG_PATH)).toBeGreaterThan(0);
  });

  it('readDailyBudget returns 0 for missing file', () => {
    expect(readDailyBudget('/nonexistent/config.json')).toBe(0);
  });

  it('DailyRunner.budgetCheck reads from platform_config', () => {
    const mockEngine = { run: async () => ({ cycleId: 1, startedAt: 0, completedAt: 0, metricsCollected: 0, evaluations: [], proposalsGenerated: 0, experimentsRun: 0, experiments: [] }) } as unknown as EvolutionEngine;
    const runner = new DailyRunner({ configPath: CONFIG_PATH, engine: mockEngine });
    expect(runner.budgetCheck()).toBeGreaterThan(0);
  });

  it('RateLimiter.checkTokenBudget returns false when over limit', () => {
    const limiter = new RateLimiter({ maxProposalsPerDay: 5, maxProposalsPerWeek: 15, dailyTokenBudgetUsd: 5.0 });
    expect(limiter.checkTokenBudget(4.9)).toBe(true);
    expect(limiter.checkTokenBudget(5.0)).toBe(false);
    expect(limiter.checkTokenBudget(6.0)).toBe(false);
  });
});
