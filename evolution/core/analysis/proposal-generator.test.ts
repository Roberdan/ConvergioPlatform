import { describe, expect, it } from 'vitest';
import { mkdtempSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { ProposalGenerator } from './proposal-generator.js';
import type { EvaluationResult } from '../types/index.js';

function evaluation(domain: string, score: number, title: string): EvaluationResult {
  return {
    domain,
    anomalies: [],
    score,
    opportunities: [
      {
        title,
        description: `${title} description`,
        estimatedGain: '-20%',
        domain,
        suggestedBlastRadius: 'SingleRepo',
      },
    ],
  };
}

describe('ProposalGenerator', () => {
  it('generates score-ranked proposals with confidence', () => {
    const dbDir = mkdtempSync(join(tmpdir(), 'proposal-gen-'));
    const generator = new ProposalGenerator({ maxProposalsPerCycle: 5, dbPath: join(dbDir, 'outcomes.db') });

    const proposals = generator.generate([
      evaluation('bundle', 40, 'Enable code splitting'),
      evaluation('latency', 70, 'Enable HTTP/2'),
    ], ['mld', 'dashboard']);

    expect(proposals.length).toBe(2);
    expect(proposals[0]?.confidence).toBeGreaterThanOrEqual(proposals[1]?.confidence ?? 0);
    rmSync(dbDir, { recursive: true, force: true });
  });

  it('scores proposals based on domain score and blast radius', () => {
    const dbDir = mkdtempSync(join(tmpdir(), 'proposal-score-'));
    const generator = new ProposalGenerator({ dbPath: join(dbDir, 'outcomes.db') });
    const score = generator.scoreProposal(
      {
        title: 'Tree-shake imports',
        description: 'desc',
        estimatedGain: '-15%',
        domain: 'bundle',
        suggestedBlastRadius: 'SingleFile',
      },
      20,
    );

    expect(score).toBe(0.8);
    rmSync(dbDir, { recursive: true, force: true });
  });
});
