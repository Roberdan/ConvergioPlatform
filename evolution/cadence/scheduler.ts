import type { CycleSummary } from '../core/engine.js';
import { DailyRunner } from './daily-runner.js';
import { WeeklyRunner } from './weekly-runner.js';
import { CronConfig } from './cron-config.js';
import { ensureCadenceSchema, sqliteQuery } from './sqlite.js';

export class CadenceScheduler {
  private readonly dbPath: string;
  private readonly engine: { run(): Promise<CycleSummary> };
  private readonly dailyRunner: DailyRunner;
  private readonly weeklyRunner: WeeklyRunner;
  private readonly cron = new CronConfig();
  private timer?: ReturnType<typeof setInterval>;

  constructor(options: {
    engine: { run(): Promise<CycleSummary> };
    dailyRunner: DailyRunner;
    weeklyRunner: WeeklyRunner;
    dbPath?: string;
  }) {
    this.dbPath = options.dbPath ?? '.evolution-cadence.db';
    this.engine = options.engine;
    this.dailyRunner = options.dailyRunner;
    this.weeklyRunner = options.weeklyRunner;
    ensureCadenceSchema(this.dbPath);
  }

  start(): void {
    this.stop();
    this.timer = setInterval(() => {
      void this.tick();
    }, 60_000);
  }

  stop(): void {
    if (this.timer) {
      clearInterval(this.timer);
      this.timer = undefined;
    }
  }

  async triggerManual(mode: 'daily' | 'weekly'): Promise<CycleSummary> {
    if (mode === 'daily') return this.dailyRunner.run(this.engine);
    const retrospective = await this.weeklyRunner.run(this.engine);
    return retrospective[retrospective.length - 1] ?? {
      cycleId: 0,
      startedAt: Date.now(),
      completedAt: Date.now(),
      metricsCollected: 0,
      evaluations: [],
      proposalsGenerated: 0,
      experimentsRun: 0,
      experiments: [],
    };
  }

  private async tick(): Promise<void> {
    const now = new Date();
    const dailyExpr = this.cron.parse(this.readCron('evolution.daily_cron', '0 6 * * *'));
    const weeklyExpr = this.cron.parse(this.readCron('evolution.weekly_cron', '0 4 * * 1'));

    if (this.cron.matches(dailyExpr, now)) {
      await this.dailyRunner.run(this.engine);
    }
    if (this.cron.matches(weeklyExpr, now)) {
      await this.weeklyRunner.run(this.engine);
    }
  }

  private readCron(key: string, fallback: string): string {
    const escaped = key.replace(/'/g, "''");
    const value = sqliteQuery(this.dbPath, `SELECT value FROM platform_config WHERE key='${escaped}' LIMIT 1;`);
    return value || fallback;
  }
}
