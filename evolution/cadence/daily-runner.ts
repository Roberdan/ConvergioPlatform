import { writeFileSync } from 'node:fs';
import type { CycleSummary } from '../core/engine.js';
import { ensureCadenceSchema, escapeSql, sqliteExec, sqliteQuery } from './sqlite.js';

export interface DailyRunnerOptions {
  dbPath?: string;
  statusFilePath?: string;
  now?: () => number;
}

export class DailyRunner {
  private readonly dbPath: string;
  private readonly statusFilePath: string;
  private readonly now: () => number;

  constructor(options: DailyRunnerOptions = {}) {
    this.dbPath = options.dbPath ?? '.evolution-cadence.db';
    this.statusFilePath = options.statusFilePath ?? 'data/cadence-status.json';
    this.now = options.now ?? (() => Date.now());
    ensureCadenceSchema(this.dbPath);
  }

  async run(engine: { run(): Promise<CycleSummary> }): Promise<CycleSummary> {
    if (!this.isBudgetAvailable()) {
      return this.skippedSummary();
    }

    const lastRun = this.lastDailyRunAt();
    if (lastRun > 0 && this.now() - lastRun < 20 * 60 * 60 * 1000) {
      return this.skippedSummary();
    }

    const dailyMicroLoop = await engine.run();
    const lightweightEval = averageScore(dailyMicroLoop);
    const deltaScore = (lightweightEval - 100) / 100;

    sqliteExec(
      this.dbPath,
      `INSERT INTO evolution_cycle_log(cadence, cycle_id, started_at, completed_at, delta_score, summary_json, report_md)
       VALUES('daily', ${dailyMicroLoop.cycleId}, ${dailyMicroLoop.startedAt}, ${dailyMicroLoop.completedAt}, ${deltaScore},
       '${escapeSql(JSON.stringify(dailyMicroLoop))}', NULL);`,
    );

    writeFileSync(
      this.statusFilePath,
      JSON.stringify({ lastRunTimestamp: dailyMicroLoop.completedAt, lastDeltaScore: deltaScore }, null, 2),
      'utf8',
    );

    return dailyMicroLoop;
  }

  private isBudgetAvailable(): boolean {
    const raw = sqliteQuery(
      this.dbPath,
      "SELECT value FROM platform_config WHERE key='evolution.daily_budget_usd' LIMIT 1;",
    );
    if (!raw) {
      return true;
    }
    return Number(raw) > 0;
  }

  private lastDailyRunAt(): number {
    const raw = sqliteQuery(
      this.dbPath,
      "SELECT completed_at FROM evolution_cycle_log WHERE cadence='daily' ORDER BY completed_at DESC LIMIT 1;",
    );
    return raw ? Number(raw) : 0;
  }

  private skippedSummary(): CycleSummary {
    const now = this.now();
    return {
      cycleId: 0,
      startedAt: now,
      completedAt: now,
      metricsCollected: 0,
      evaluations: [{ domain: 'cadence', anomalies: [], opportunities: [], score: 0 }],
      proposalsGenerated: 0,
      experimentsRun: 0,
      experiments: [],
    };
  }
}

function averageScore(summary: CycleSummary): number {
  if (summary.evaluations.length === 0) {
    return 0;
  }
  return summary.evaluations.reduce((sum, evalResult) => sum + evalResult.score, 0) / summary.evaluations.length;
}
