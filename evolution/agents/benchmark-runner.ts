import { homedir } from 'os';
import { join } from 'path';
import { spawnSync } from 'child_process';

export interface BenchmarkResult {
  modelId: string;
  taskType: string;
  avgTokens: number;
  avgDurationMin: number;
  costPer100Tasks: number;
  sampleSize: number;
}

export interface BenchmarkRunnerOptions {
  dbPath?: string;
}

export class BenchmarkRunner {
  private readonly dbPath: string;

  constructor(options: BenchmarkRunnerOptions = {}) {
    this.dbPath = options.dbPath ?? join(homedir(), '.claude', 'data', 'dashboard.db');
  }

  run(modelId: string, taskType: string): BenchmarkResult {
    const sql = `SELECT
      COALESCE(AVG(COALESCE(tu.input_tokens,0)+COALESCE(tu.output_tokens,0)),0),
      COALESCE(AVG((julianday(COALESCE(t.completed_at, datetime('now'))) - julianday(COALESCE(t.started_at, t.created_at, datetime('now'))))*24*60),0),
      COALESCE(SUM(COALESCE(tu.cost_usd,0)),0),
      COUNT(*)
      FROM (
        SELECT id, created_at, started_at, completed_at
        FROM tasks
        WHERE status='done' AND task_type='${escapeSql(taskType)}'
        ORDER BY COALESCE(completed_at, created_at) DESC
        LIMIT 10
      ) t
      JOIN token_usage tu ON tu.task_id=t.id
      WHERE tu.model='${escapeSql(modelId)}';`;

    const row = queryRow(this.dbPath, sql);
    const sampleSize = Math.round(parseNumber(row[3]));
    const costTotal = parseNumber(row[2]);

    return {
      modelId,
      taskType,
      avgTokens: parseNumber(row[0]),
      avgDurationMin: parseNumber(row[1]),
      costPer100Tasks: sampleSize > 0 ? (costTotal / sampleSize) * 100 : 0,
      sampleSize,
    };
  }
}

function queryRow(dbPath: string, sql: string): string[] {
  const result = spawnSync('sqlite3', ['-separator', '|', dbPath, sql], { encoding: 'utf8' });
  if (result.status !== 0) {
    return [];
  }
  const line = (result.stdout ?? '').trim().split('\n')[0] ?? '';
  return line ? line.split('|') : [];
}

function parseNumber(value: string | undefined): number {
  const parsed = Number(value ?? 0);
  return Number.isFinite(parsed) ? parsed : 0;
}

function escapeSql(value: string): string {
  return value.replaceAll("'", "''");
}
