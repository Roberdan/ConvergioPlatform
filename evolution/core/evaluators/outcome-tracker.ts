import { spawnSync } from 'node:child_process';
import type { EvaluationResult } from '../types/index.js';

export interface OutcomeTrackerOptions {
  dbPath?: string;
}

export class OutcomeTracker {
  private readonly dbPath: string;

  constructor(options: OutcomeTrackerOptions = {}) {
    this.dbPath = options.dbPath ?? '.evolution-outcomes.db';
    this.ensureSchema();
  }

  record(proposalId: string, before: EvaluationResult, after: EvaluationResult): void {
    const delta = after.score - before.score;
    const improved = delta > 0 ? 1 : 0;
    const sql = `INSERT INTO evolution_outcomes(
      proposal_id, domain, before_score, after_score, delta_score, improved, created_at
    ) VALUES(
      '${escapeSql(proposalId)}',
      '${escapeSql(before.domain)}',
      ${before.score},
      ${after.score},
      ${delta},
      ${improved},
      ${Date.now()}
    );`;
    this.exec(sql);
  }

  getROI(proposalId: string): { deltaScore: number; improved: boolean } | null {
    const sql = `SELECT delta_score, improved FROM evolution_outcomes WHERE proposal_id = '${escapeSql(
      proposalId,
    )}' ORDER BY created_at DESC LIMIT 1;`;
    const result = this.query(sql);
    if (!result) {
      return null;
    }

    const [deltaRaw, improvedRaw] = result.split('|');
    return {
      deltaScore: Number(deltaRaw),
      improved: Number(improvedRaw) === 1,
    };
  }

  private ensureSchema(): void {
    this.exec(`CREATE TABLE IF NOT EXISTS evolution_outcomes (
      id INTEGER PRIMARY KEY,
      proposal_id TEXT NOT NULL,
      domain TEXT NOT NULL,
      before_score REAL NOT NULL,
      after_score REAL NOT NULL,
      delta_score REAL NOT NULL,
      improved INTEGER NOT NULL,
      created_at INTEGER NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_evolution_outcomes_proposal ON evolution_outcomes(proposal_id, created_at DESC);`);
  }

  private exec(sql: string): void {
    const result = spawnSync('sqlite3', [this.dbPath, sql], { encoding: 'utf8' });
    if (result.status !== 0) {
      throw new Error(result.stderr || 'sqlite3 execution failed');
    }
  }

  private query(sql: string): string | null {
    const result = spawnSync('sqlite3', ['-separator', '|', this.dbPath, sql], { encoding: 'utf8' });
    if (result.status !== 0) {
      throw new Error(result.stderr || 'sqlite3 query failed');
    }
    const line = result.stdout.trim();
    return line.length ? line : null;
  }
}

function escapeSql(value: string): string {
  return value.replace(/'/g, "''");
}
