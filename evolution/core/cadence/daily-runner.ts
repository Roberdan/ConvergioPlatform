/**
 * DailyRunner — cron-triggered daily evaluation cycle.
 *
 * Reads the daily budget from platform_config before executing any LLM calls.
 * The engine is skipped entirely if the KillSwitch is active or budget is zero.
 */

import { readFileSync, existsSync } from 'fs';
import { join } from 'path';
import type { EvolutionEngine, CycleSummary } from '../engine.js';

export interface DailyRunnerOptions {
  /** Path to platform_config.json. Defaults to repo-root/config/platform_config.json. */
  configPath?: string;
  engine: EvolutionEngine;
}

export interface DailyBudget {
  dailyBudgetUsd: number;
}

/**
 * Reads `evolution.daily_budget_usd` from platform_config.json.
 * Returns 0 if the file is missing or the key is absent (safe default: skip run).
 */
export function readDailyBudget(configPath: string): number {
  if (!existsSync(configPath)) {
    return 0;
  }
  try {
    const raw = readFileSync(configPath, 'utf-8');
    const cfg = JSON.parse(raw) as Record<string, unknown>;
    const evolution = cfg['evolution'] as Record<string, unknown> | undefined;
    const budget = evolution?.['daily_budget_usd'];
    return typeof budget === 'number' ? budget : 0;
  } catch {
    return 0;
  }
}

export class DailyRunner {
  private readonly engine: EvolutionEngine;
  private readonly configPath: string;

  constructor(options: DailyRunnerOptions) {
    this.engine = options.engine;
    this.configPath =
      options.configPath ??
      join(process.cwd(), 'config', 'platform_config.json');
  }

  /**
   * Execute the daily cycle.
   * Reads budget first; skips run if budget is 0.
   */
  async run(): Promise<CycleSummary | null> {
    const budget = readDailyBudget(this.configPath);
    if (budget <= 0) {
      return null;
    }
    return this.engine.run();
  }

  /**
   * Budget check exposed for testing.
   * Returns the current daily budget from config.
   */
  budgetCheck(): number {
    return readDailyBudget(this.configPath);
  }
}
