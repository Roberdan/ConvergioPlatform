import { homedir } from 'os';
import { join } from 'path';
import { spawnSync } from 'child_process';

export interface ValidationResult {
  valid: boolean;
  warnings: string[];
}

export interface AgentValidatorOptions {
  dbPath?: string;
}

export class AgentValidator {
  private readonly dbPath: string;

  constructor(options: AgentValidatorOptions = {}) {
    this.dbPath = options.dbPath ?? join(homedir(), '.claude', 'data', 'dashboard.db');
  }

  validateModelChange(from: string, to: string): ValidationResult {
    const warnings: string[] = [];

    if (from === to) {
      warnings.push('No model change requested.');
    }

    const modelExists = this.queryNumber(
      `SELECT COUNT(*) FROM ipc_model_registry WHERE model_id='${escapeSql(to)}' OR id='${escapeSql(to)}';`,
    );

    if (modelExists === 0) {
      warnings.push(`Target model '${to}' was not found in ipc_model_registry.`);
    }

    const runningExperiments = this.queryNumber(
      "SELECT COUNT(*) FROM evolution_experiments WHERE status='running';",
    );

    if (runningExperiments > 0) {
      warnings.push('Active experiments are running; model changes are currently blocked.');
    }

    return {
      valid: warnings.length === 0,
      warnings,
    };
  }

  private queryNumber(sql: string): number {
    const result = spawnSync('sqlite3', ['-separator', '|', this.dbPath, sql], { encoding: 'utf8' });
    if (result.status !== 0) {
      return 0;
    }
    const value = (result.stdout ?? '').trim().split('\n')[0] ?? '0';
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : 0;
  }
}

function escapeSql(value: string): string {
  return value.replaceAll("'", "''");
}
