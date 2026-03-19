import * as childProcess from 'child_process';
import type { PlatformAdapter } from '../core/types/adapter.js';

export class ReversibilityGuarantor {
  constructor(private readonly dbPath = `${process.env.HOME ?? ''}/.claude/data/dashboard.db`) {}

  canRollback(experimentId: string): boolean {
    const rollbackRecipe = this.query(
      `SELECT rollback_info FROM evolution_experiments WHERE id='${escapeSql(experimentId)}' LIMIT 1;`,
    );
    const configSnapshot = this.query(
      `SELECT config_snapshot FROM evolution_experiments WHERE id='${escapeSql(experimentId)}' LIMIT 1;`,
    );
    return Boolean(rollbackRecipe) && Boolean(configSnapshot);
  }

  async executeRollback(experimentId: string, adapter: PlatformAdapter): Promise<void> {
    if (!this.canRollback(experimentId)) {
      throw new Error('rollbackRecipe/configSnapshot not found for experiment');
    }

    await adapter.rollback(experimentId);

    const revertPR =
      `UPDATE evolution_experiments SET status='rolled_back', rolled_back_at=${Date.now()} ` +
      `WHERE id='${escapeSql(experimentId)}';`;
    this.exec(revertPR);
  }

  private exec(sql: string): void {
    const res = childProcess.spawnSync('sqlite3', [this.dbPath, sql], { encoding: 'utf8' });
    if (res.status !== 0) throw new Error(res.stderr || 'reversibility exec failed');
  }

  private query(sql: string): string {
    const res = childProcess.spawnSync('sqlite3', [this.dbPath, sql], { encoding: 'utf8' });
    if (res.status !== 0) throw new Error(res.stderr || 'reversibility query failed');
    return res.stdout.trim();
  }
}

function escapeSql(value: string): string {
  return value.replace(/'/g, "''");
}
