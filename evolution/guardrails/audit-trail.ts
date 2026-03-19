import * as childProcess from 'child_process';

export interface AuditEntry {
  id: string;
  timestamp: number;
  actor: string;
  action: string;
  proposalId: string;
  detail: string;
  blastRadius: string;
}

export interface AuditQueryOptions {
  proposalId?: string;
  action?: string;
  from?: number;
}

export class AuditTrail {
  constructor(private readonly dbPath = `${process.env.HOME ?? ''}/.claude/data/dashboard.db`) {
    this.ensureSchema();
  }

  log(action: string, proposalId: string, actor: string, detail: string, blastRadius: string): void {
    const now = Date.now();
    const id = `${proposalId}-${now}`;
    const sql =
      `INSERT INTO evolution_audit(id, ts, action, proposal_id, actor, detail, blast_radius) VALUES(` +
      `'${escapeSql(id)}',${now},'${escapeSql(action)}','${escapeSql(proposalId)}','${escapeSql(actor)}','${escapeSql(detail)}','${escapeSql(blastRadius)}');`;
    this.exec(sql);
  }

  query(options: AuditQueryOptions): AuditEntry[] {
    const clauses = ['1=1'];
    if (options.proposalId) clauses.push(`proposal_id='${escapeSql(options.proposalId)}'`);
    if (options.action) clauses.push(`action='${escapeSql(options.action)}'`);
    if (options.from) clauses.push(`ts>=${options.from}`);

    const sql = `SELECT id, ts, action, proposal_id, actor, detail, blast_radius FROM evolution_audit WHERE ${clauses.join(' AND ')} ORDER BY ts DESC;`;
    const out = this.queryRaw(sql);
    if (!out) return [];
    return out.split('\n').filter(Boolean).map((line) => {
      const [id, ts, action, proposalId, actor, detail, blastRadius] = line.split('|');
      return { id, timestamp: Number(ts), action, proposalId, actor, detail, blastRadius };
    });
  }

  private ensureSchema(): void {
    this.exec(
      'CREATE TABLE IF NOT EXISTS evolution_audit (' +
        'id TEXT PRIMARY KEY, ts INTEGER, action TEXT, proposal_id TEXT, actor TEXT, detail TEXT, blast_radius TEXT);',
    );
  }

  private exec(sql: string): void {
    const res = childProcess.spawnSync('sqlite3', [this.dbPath, sql], { encoding: 'utf8' });
    if (res.status !== 0) throw new Error(res.stderr || 'audit trail exec failed');
  }

  private queryRaw(sql: string): string {
    const res = childProcess.spawnSync('sqlite3', ['-separator', '|', this.dbPath, sql], { encoding: 'utf8' });
    if (res.status !== 0) throw new Error(res.stderr || 'audit trail query failed');
    return res.stdout.trim();
  }
}

function escapeSql(value: string): string {
  return value.replace(/'/g, "''");
}
