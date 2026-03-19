import { spawnSync } from 'node:child_process';

export interface Hypothesis {
  id: string;
  proposalId: string;
  source: string;
  tags: string[];
  confidence: number;
  createdAt: number;
  testedAt?: number;
  outcome?: 'confirmed' | 'rejected' | 'inconclusive';
}

export interface HypothesisStoreOptions {
  dbPath?: string;
}

export class HypothesisStore {
  private readonly dbPath: string;

  constructor(options: HypothesisStoreOptions = {}) {
    this.dbPath = options.dbPath ?? '.evolution-hypotheses.db';
    this.ensureSchema();
  }

  save(hypothesis: Hypothesis): void {
    this.exec(`INSERT OR REPLACE INTO evolution_hypotheses(
      id, proposal_id, source, tags, confidence, created_at, tested_at, outcome
    ) VALUES(
      '${escapeSql(hypothesis.id)}',
      '${escapeSql(hypothesis.proposalId)}',
      '${escapeSql(hypothesis.source)}',
      '${escapeSql(JSON.stringify(hypothesis.tags))}',
      ${hypothesis.confidence},
      ${hypothesis.createdAt},
      ${hypothesis.testedAt ?? 'NULL'},
      ${hypothesis.outcome ? `'${escapeSql(hypothesis.outcome)}'` : 'NULL'}
    );`);
  }

  findRecent(days: number): Hypothesis[] {
    const cutoff = Date.now() - days * 24 * 60 * 60 * 1000;
    const output = this.query(`SELECT id, proposal_id, source, tags, confidence, created_at, tested_at, outcome
      FROM evolution_hypotheses
      WHERE created_at >= ${cutoff}
      ORDER BY created_at DESC;`);

    if (!output) {
      return [];
    }

    return output.split('\n').filter(Boolean).map((line) => {
      const [id, proposalId, source, tags, confidence, createdAt, testedAt, outcome] = line.split('|');
      return {
        id,
        proposalId,
        source,
        tags: tags ? (JSON.parse(tags) as string[]) : [],
        confidence: Number(confidence),
        createdAt: Number(createdAt),
        testedAt: testedAt ? Number(testedAt) : undefined,
        outcome: outcome as Hypothesis['outcome'] | undefined,
      };
    });
  }

  markTested(id: string, outcome: 'confirmed' | 'rejected' | 'inconclusive'): void {
    this.exec(`UPDATE evolution_hypotheses
      SET tested_at = ${Date.now()}, outcome = '${escapeSql(outcome)}'
      WHERE id = '${escapeSql(id)}';`);
  }

  private ensureSchema(): void {
    this.exec(`CREATE TABLE IF NOT EXISTS evolution_hypotheses (
      id TEXT PRIMARY KEY,
      proposal_id TEXT NOT NULL,
      source TEXT NOT NULL,
      tags TEXT NOT NULL,
      confidence REAL NOT NULL,
      created_at INTEGER NOT NULL,
      tested_at INTEGER,
      outcome TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_evolution_hypotheses_created ON evolution_hypotheses(created_at DESC);`);
  }

  private exec(sql: string): void {
    const result = spawnSync('sqlite3', [this.dbPath, sql], { encoding: 'utf8' });
    if (result.status !== 0) {
      throw new Error(result.stderr || 'sqlite3 execution failed');
    }
  }

  private query(sql: string): string {
    const result = spawnSync('sqlite3', ['-separator', '|', this.dbPath, sql], { encoding: 'utf8' });
    if (result.status !== 0) {
      throw new Error(result.stderr || 'sqlite3 query failed');
    }
    return result.stdout.trim();
  }
}

function escapeSql(value: string): string {
  return value.replace(/'/g, "''");
}
