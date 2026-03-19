import type { CycleSummary } from '../core/engine.js';
import { ReportGenerator } from '../experiments/report-generator.js';
import { ensureCadenceSchema, escapeSql, sqliteExec, sqliteQuery } from './sqlite.js';

export interface WeeklyRunnerOptions {
  dbPath?: string;
  now?: () => number;
}

export class WeeklyRunner {
  private readonly dbPath: string;
  private readonly now: () => number;

  constructor(options: WeeklyRunnerOptions = {}) {
    this.dbPath = options.dbPath ?? '.evolution-cadence.db';
    this.now = options.now ?? (() => Date.now());
    ensureCadenceSchema(this.dbPath);
  }

  async run(_engine: { run(): Promise<CycleSummary> }): Promise<CycleSummary[]> {
    const since = this.now() - 7 * 24 * 60 * 60 * 1000;
    const rows = sqliteQuery(
      this.dbPath,
      `SELECT cycle_id, summary_json, delta_score
       FROM evolution_cycle_log
       WHERE cadence='daily' AND completed_at >= ${since}
       ORDER BY completed_at ASC;`,
    );

    const deepAnalysis = rows
      .split('\n')
      .filter(Boolean)
      .map((line) => JSON.parse(line.split('|')[1] ?? '{}') as CycleSummary);

    const crossDomainCorrelation = computeWeeklyDelta(rows);
    const report = new ReportGenerator().generate({
      experimentId: `weekly-${new Date(this.now()).toISOString().slice(0, 10)}`,
      mode: 'Full',
      proposalTitle: 'Weekly deep analysis retrospective',
      durationMs: 7 * 24 * 60 * 60 * 1000,
      beforeAfter: {
        before: [{ name: 'weekly_delta_before', value: 0 }],
        after: [{ name: 'weekly_delta_after', value: crossDomainCorrelation }],
      },
      confidenceInterval: [crossDomainCorrelation - 0.05, crossDomainCorrelation + 0.05],
      pValue: 0.1,
      recommendation: crossDomainCorrelation < 0 ? 'Apply' : 'Inconclusive',
      anomaliesResolved: [],
      deltaScore: crossDomainCorrelation,
    });

    if (deepAnalysis.length > 0) {
      const latest = deepAnalysis[deepAnalysis.length - 1];
      sqliteExec(
        this.dbPath,
        `INSERT INTO evolution_cycle_log(cadence, cycle_id, started_at, completed_at, delta_score, summary_json, report_md)
         VALUES('weekly', ${latest?.cycleId ?? 0}, ${this.now()}, ${this.now()}, ${crossDomainCorrelation},
         '${escapeSql(JSON.stringify(latest))}', '${escapeSql(report)}');`,
      );
    }

    return deepAnalysis;
  }
}

function computeWeeklyDelta(rows: string): number {
  const deltas = rows
    .split('\n')
    .filter(Boolean)
    .map((line) => Number(line.split('|')[2] ?? 0));

  if (deltas.length === 0) {
    return 0;
  }

  return deltas.reduce((sum, value) => sum + value, 0) / deltas.length;
}
