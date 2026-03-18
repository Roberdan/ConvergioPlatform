import { createRequire } from 'module';
import { spawnSync } from 'child_process';
import type { Metric, MetricFamily } from '../core/types/index.js';
import { TELEMETRY_DB_PATH } from './config.js';
import { rollup5m, type AggregatedPoint } from './aggregation.js';

export interface QueryOptions {
  name?: string;
  family?: MetricFamily;
  from: number;
  to: number;
}

export interface MetricStoreOptions {
  dbPath?: string;
  host?: string;
}

type BetterSqliteDatabase = {
  exec(sql: string): void;
  prepare(sql: string): {
    run: (...args: unknown[]) => void;
    all: (...args: unknown[]) => Array<Record<string, unknown>>;
  };
};

const CREATE_TABLE_SQL = `
CREATE TABLE IF NOT EXISTS telemetry_metrics (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  value REAL NOT NULL,
  family TEXT NOT NULL,
  labels TEXT,
  ts INTEGER NOT NULL,
  host TEXT NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS idx_tm_name_ts ON telemetry_metrics(name, ts DESC);
CREATE INDEX IF NOT EXISTS idx_tm_family ON telemetry_metrics(family, ts DESC);
`;

export class MetricStore {
  private readonly dbPath: string;
  private readonly host: string;
  private readonly db?: BetterSqliteDatabase;

  constructor(options: MetricStoreOptions = {}) {
    this.dbPath = options.dbPath ?? TELEMETRY_DB_PATH;
    this.host = options.host ?? '';
    this.db = this.tryLoadBetterSqlite();
  }

  persist(metrics: Metric[]): void {
    if (metrics.length === 0) return;
    this.createSchema();

    if (this.db) {
      const stmt = this.db.prepare(
        'INSERT INTO telemetry_metrics(name, value, family, labels, ts, host) VALUES(?, ?, ?, ?, ?, ?)',
      );
      for (const metric of metrics) {
        stmt.run(
          metric.name,
          metric.value,
          metric.family,
          JSON.stringify(metric.labels),
          metric.timestamp,
          this.host,
        );
      }
      return;
    }

    const values = metrics
      .map((metric) =>
        `('${escape(metric.name)}',${metric.value},'${escape(metric.family)}','${escape(
          JSON.stringify(metric.labels),
        )}',${metric.timestamp},'${escape(this.host)}')`,
      )
      .join(',');

    this.sqliteExec(`INSERT INTO telemetry_metrics(name, value, family, labels, ts, host) VALUES ${values};`);
  }

  query(options: QueryOptions): Metric[] {
    this.createSchema();
    const { sql, params } = this.buildWhere(options);

    if (this.db) {
      const rows = this.db.prepare(sql).all(...params);
      return rows.map(toMetric);
    }

    const result = this.sqliteQuery(sql.replace(/\?/g, () => serializeParam(params.shift())));
    return result
      .split('\n')
      .filter(Boolean)
      .map((line) => toMetricFromPipe(line));
  }

  aggregate(options: QueryOptions): AggregatedPoint[] {
    return rollup5m(this.query(options));
  }

  purge(retentionDays: number): void {
    this.createSchema();
    const cutoff = Date.now() - retentionDays * 24 * 60 * 60 * 1000;
    this.sqliteExec(`DELETE FROM telemetry_metrics WHERE ts < ${cutoff};`);
  }

  private createSchema(): void {
    this.sqliteExec('PRAGMA journal_mode=WAL;');
    this.sqliteExec(CREATE_TABLE_SQL);
  }

  private buildWhere(options: QueryOptions): { sql: string; params: Array<string | number> } {
    const clauses = ['ts >= ?', 'ts <= ?'];
    const params: Array<string | number> = [options.from, options.to];

    if (options.name) {
      clauses.push('name = ?');
      params.push(options.name);
    }
    if (options.family) {
      clauses.push('family = ?');
      params.push(options.family);
    }

    return {
      sql: `SELECT name, value, family, labels, ts FROM telemetry_metrics WHERE ${clauses.join(
        ' AND ',
      )} ORDER BY ts ASC;`,
      params,
    };
  }

  private sqliteExec(sql: string): void {
    if (this.db) {
      this.db.exec(sql);
      return;
    }
    const result = spawnSync('sqlite3', [this.dbPath, sql], { encoding: 'utf8' });
    if (result.status !== 0) {
      throw new Error(result.stderr || 'sqlite3 execution failed');
    }
  }

  private sqliteQuery(sql: string): string {
    const result = spawnSync('sqlite3', ['-separator', '|', this.dbPath, sql], { encoding: 'utf8' });
    if (result.status !== 0) {
      throw new Error(result.stderr || 'sqlite3 query failed');
    }
    return result.stdout.trim();
  }

  private tryLoadBetterSqlite(): BetterSqliteDatabase | undefined {
    try {
      const req = createRequire(import.meta.url);
      const BetterSqlite3 = req('better-sqlite3') as new (path: string) => BetterSqliteDatabase;
      return new BetterSqlite3(this.dbPath);
    } catch {
      return undefined;
    }
  }
}

function toMetric(row: Record<string, unknown>): Metric {
  return {
    name: String(row.name ?? ''),
    value: Number(row.value ?? 0),
    family: row.family as MetricFamily,
    timestamp: Number(row.ts ?? 0),
    labels: row.labels ? JSON.parse(String(row.labels)) as Record<string, string> : {},
  };
}

function toMetricFromPipe(line: string): Metric {
  const [name, value, family, labels, ts] = line.split('|');
  return {
    name,
    value: Number(value),
    family: family as MetricFamily,
    labels: labels ? JSON.parse(labels) as Record<string, string> : {},
    timestamp: Number(ts),
  };
}

function escape(value: string): string {
  return value.replace(/'/g, "''");
}

function serializeParam(value: string | number | undefined): string {
  if (typeof value === 'number') return `${value}`;
  return `'${escape(String(value ?? ''))}'`;
}
