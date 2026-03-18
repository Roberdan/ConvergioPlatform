/**
 * NF: Cost Control — validates evolution budget enforcement.
 *
 * Verifies:
 * 1. platform_config.json has evolution.daily_budget_usd key
 * 2. DailyRunner exposes budgetCheck() that reads from platform_config
 * 3. RateLimiter.checkTokenBudget returns false when over limit
 */

import { describe, it, expect } from 'vitest';
import { readFileSync, existsSync } from 'fs';
import { join } from 'path';
import { RateLimiter } from '../../evolution/core/guardrails/rate-limiter.js';
import { DailyRunner, readDailyBudget } from '../../evolution/core/cadence/daily-runner.js';
import type { EvolutionEngine } from '../../evolution/core/engine.js';

const REPO_ROOT = join(import.meta.dirname, '..', '..');
const CONFIG_PATH = join(REPO_ROOT, 'config', 'platform_config.json');

describe('NF: Cost Control', () => {
  it('platform_config.json has evolution.daily_budget_usd', () => {
    expect(existsSync(CONFIG_PATH)).toBe(true);
    const raw = readFileSync(CONFIG_PATH, 'utf-8');
    const cfg = JSON.parse(raw) as Record<string, unknown>;
    expect(cfg).toHaveProperty('evolution');
    const evolution = cfg['evolution'] as Record<string, unknown>;
    expect(evolution).toHaveProperty('daily_budget_usd');
    expect(typeof evolution['daily_budget_usd']).toBe('number');
    expect(evolution['daily_budget_usd'] as number).toBeGreaterThan(0);
  });

  it('readDailyBudget reads evolution.daily_budget_usd from config', () => {
    const budget = readDailyBudget(CONFIG_PATH);
    expect(budget).toBeGreaterThan(0);
  });

  it('readDailyBudget returns 0 for missing file', () => {
    const budget = readDailyBudget('/nonexistent/path/platform_config.json');
    expect(budget).toBe(0);
  });

  it('DailyRunner.budgetCheck() reads budget from config', () => {
    const mockEngine = {
      run: async () => ({
        cycleId: 1,
        startedAt: 0,
        completedAt: 0,
        metricsCollected: 0,
        evaluations: [],
        proposalsGenerated: 0,
        experimentsRun: 0,
        experiments: [],
      }),
    } as unknown as EvolutionEngine;

    const runner = new DailyRunner({
      configPath: CONFIG_PATH,
      engine: mockEngine,
    });
    const budget = runner.budgetCheck();
    expect(budget).toBeGreaterThan(0);
  });

  it('RateLimiter.checkTokenBudget returns false when over limit', () => {
    const limiter = new RateLimiter({
      maxProposalsPerDay: 5,
      maxProposalsPerWeek: 15,
      dailyTokenBudgetUsd: 5.0,
    });
    expect(limiter.checkTokenBudget(3.0)).toBe(true);
    expect(limiter.checkTokenBudget(5.0)).toBe(false);
    expect(limiter.checkTokenBudget(6.0)).toBe(false);
  });

  it('RateLimiter.canGenerateProposal respects daily limit', () => {
    const limiter = new RateLimiter({
      maxProposalsPerDay: 2,
      maxProposalsPerWeek: 10,
      dailyTokenBudgetUsd: 5.0,
    });
    expect(limiter.canGenerateProposal()).toBe(true);
    limiter.recordProposal();
    limiter.recordProposal();
    expect(limiter.canGenerateProposal()).toBe(false);
  });
});
