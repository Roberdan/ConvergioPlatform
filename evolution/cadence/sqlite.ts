import { spawnSync } from 'node:child_process';

export function sqliteExec(dbPath: string, sql: string): void {
  const result = spawnSync('sqlite3', [dbPath, sql], { encoding: 'utf8' });
  if (result.status !== 0) {
    throw new Error(result.stderr || 'sqlite3 execution failed');
  }
}

export function sqliteQuery(dbPath: string, sql: string): string {
  const result = spawnSync('sqlite3', ['-separator', '|', dbPath, sql], { encoding: 'utf8' });
  if (result.status !== 0) {
    throw new Error(result.stderr || 'sqlite3 query failed');
  }
  return result.stdout.trim();
}

export function ensureCadenceSchema(dbPath: string): void {
  sqliteExec(
    dbPath,
    `CREATE TABLE IF NOT EXISTS platform_config (
      key TEXT PRIMARY KEY,
      value TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS evolution_cycle_log (
      id INTEGER PRIMARY KEY,
      cadence TEXT NOT NULL,
      cycle_id INTEGER NOT NULL,
      started_at INTEGER NOT NULL,
      completed_at INTEGER NOT NULL,
      delta_score REAL NOT NULL,
      summary_json TEXT NOT NULL,
      report_md TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_cycle_log_cadence_time
      ON evolution_cycle_log(cadence, completed_at DESC);`,
  );
}

export function escapeSql(value: string): string {
  return value.replace(/'/g, "''");
}
