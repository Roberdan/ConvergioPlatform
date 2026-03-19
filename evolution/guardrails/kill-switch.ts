import * as childProcess from 'child_process';

const ENABLED_KEY = 'evolution.enabled';
const CACHE_MS = 30_000;

export class KillSwitch {
  private enabledCache: { value: boolean; expiresAt: number } | null = null;

  constructor(private readonly dbPath = `${process.env.HOME ?? ''}/.claude/data/dashboard.db`, private readonly now = () => Date.now()) {}

  isEnabled(): boolean {
    const current = this.now();
    if (this.enabledCache && this.enabledCache.expiresAt > current) {
      return this.enabledCache.value;
    }

    const value = this.queryValue(`SELECT value FROM platform_config WHERE key='${ENABLED_KEY}' LIMIT 1;`);
    const enabled = value !== 'false';
    this.enabledCache = { value: enabled, expiresAt: current + CACHE_MS };
    return enabled;
  }

  disable(reason: string): void {
    // KILLED flag + immediatHalt with rollback signal in audit detail.
    this.exec(
      `INSERT OR REPLACE INTO platform_config(key,value) VALUES('${ENABLED_KEY}','false');` +
        `INSERT INTO evolution_audit(id, ts, action, proposal_id, actor, detail, blast_radius) VALUES(` +
        `'kill-${this.now()}',${this.now()},'KILLED','','system','immediatHalt: ${escapeSql(reason)}; rollback advised','SingleRepo');`,
    );
    this.enabledCache = { value: false, expiresAt: this.now() + CACHE_MS };
  }

  enable(): void {
    this.exec(`INSERT OR REPLACE INTO platform_config(key,value) VALUES('${ENABLED_KEY}','true');`);
    this.enabledCache = { value: true, expiresAt: this.now() + CACHE_MS };
  }

  private exec(sql: string): void {
    const res = childProcess.spawnSync('sqlite3', [this.dbPath, sql], { encoding: 'utf8' });
    if (res.status !== 0) throw new Error(res.stderr || 'kill-switch sqlite exec failed');
  }

  private queryValue(sql: string): string {
    const res = childProcess.spawnSync('sqlite3', [this.dbPath, sql], { encoding: 'utf8' });
    if (res.status !== 0) throw new Error(res.stderr || 'kill-switch sqlite query failed');
    return res.stdout.trim();
  }
}

function escapeSql(input: string): string {
  return input.replace(/'/g, "''");
}
