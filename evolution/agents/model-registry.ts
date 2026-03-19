import { homedir } from 'os';
import { join } from 'path';
import { spawnSync } from 'child_process';

export interface ModelRecord {
  id: string;
  model_id: string;
  version: string;
  cost_per_1k: number;
}

export interface UpgradeAvailable {
  currentId: string;
  latestVersion: string;
  estimatedSavings: string;
}

export interface ModelIntelligenceOptions {
  dbPath?: string;
}

const KNOWN_LATEST: Record<string, string> = {
  'claude-opus': '4.6',
  'claude-sonnet': '4.6',
  'claude-haiku': '4.5',
  'gpt-5': '5.4',
  'gpt-5.3-codex': '5.3-codex',
};

export class ModelIntelligence {
  private readonly dbPath: string;

  constructor(options: ModelIntelligenceOptions = {}) {
    this.dbPath = options.dbPath ?? join(homedir(), '.claude', 'data', 'dashboard.db');
  }

  getCurrentModels(): ModelRecord[] {
    const rows = queryRows(this.dbPath, 'SELECT id, model_id, version, cost_per_1k FROM ipc_model_registry;');
    return rows.map((row) => ({
      id: row[0] ?? '',
      model_id: row[1] ?? '',
      version: row[2] ?? '',
      cost_per_1k: parseNumber(row[3]),
    }));
  }

  checkForUpgrades(): UpgradeAvailable[] {
    return this.getCurrentModels()
      .map((model) => {
        const latestVersion = KNOWN_LATEST[model.model_id];
        if (!latestVersion || latestVersion === model.version) {
          return null;
        }
        const savings = model.cost_per_1k > 0 ? (model.cost_per_1k * 0.1).toFixed(4) : '0.0000';
        return {
          currentId: model.model_id,
          latestVersion,
          estimatedSavings: `~${savings} USD/1k`,
        };
      })
      .filter((item): item is UpgradeAvailable => item !== null);
  }
}

function queryRows(dbPath: string, sql: string): string[][] {
  const result = spawnSync('sqlite3', ['-separator', '|', dbPath, sql], { encoding: 'utf8' });
  if (result.status !== 0) {
    return [];
  }
  const output = (result.stdout ?? '').trim();
  return output ? output.split('\n').map((line) => line.split('|')) : [];
}

function parseNumber(value: string | undefined): number {
  const parsed = Number(value ?? 0);
  return Number.isFinite(parsed) ? parsed : 0;
}
