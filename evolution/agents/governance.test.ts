import { describe, it, expect } from 'vitest';
import { mkdtempSync, rmSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { spawnSync } from 'child_process';
import { AgentGovernance } from './governance.js';
import { AgentValidator } from './validator.js';
import type { Proposal } from '../core/types/index.js';

function sqlite(db: string, sql: string): void {
  const result = spawnSync('sqlite3', [db, sql], { encoding: 'utf8' });
  if (result.status !== 0) throw new Error(result.stderr || 'sqlite3 failed');
}

function proposal(): Proposal {
  return {
    id: 'EVO-20260318-0001',
    hypothesis: 'Route simple work to cheaper model',
    targetMetric: 'agent.cost_per_task',
    successCriteria: 'cost per task decreases',
    failureCriteria: 'quality drops',
    blastRadius: 'SingleRepo',
    sourceType: 'Internal',
    status: 'Draft',
    expectedDelta: { min: -0.3, max: -0.25 },
  };
}

describe('AgentGovernance', () => {
  it('requires human approver for high cost delta changes', () => {
    const result = new AgentGovernance().validateProposal(proposal(), []);
    expect(result.prRequired).toBe(true);
    expect(result.approved).toBe(false);
  });
});

describe('AgentValidator', () => {
  it('validates model existence and running experiment constraints', () => {
    const dir = mkdtempSync(join(tmpdir(), 'agent-validator-'));
    const dbPath = join(dir, 'dashboard.db');
    sqlite(
      dbPath,
      "CREATE TABLE ipc_model_registry (id TEXT, model_id TEXT);" +
        "CREATE TABLE evolution_experiments (status TEXT);" +
        "INSERT INTO ipc_model_registry VALUES ('1','gpt-5');" +
        "INSERT INTO evolution_experiments VALUES ('running');",
    );
    const result = new AgentValidator({ dbPath }).validateModelChange('claude-opus', 'gpt-5');
    expect(result.valid).toBe(false);
    expect(result.warnings.some((warning) => warning.includes('Active experiments'))).toBe(true);
    rmSync(dir, { recursive: true, force: true });
  });
});
