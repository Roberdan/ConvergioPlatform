import { afterEach, describe, expect, it } from 'vitest';
import { mkdtempSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { OutcomeTracker } from './outcome-tracker.js';
import type { EvaluationResult } from '../types/index.js';

function evalResult(score: number): EvaluationResult {
  return {
    domain: 'latency',
    anomalies: [],
    opportunities: [],
    score,
  };
}

describe('OutcomeTracker', () => {
  const tempPaths: string[] = [];

  afterEach(() => {
    for (const path of tempPaths.splice(0)) {
      rmSync(path, { recursive: true, force: true });
    }
  });

  it('records outcomes and computes ROI delta', () => {
    const dir = mkdtempSync(join(tmpdir(), 'outcome-'));
    tempPaths.push(dir);

    const tracker = new OutcomeTracker({ dbPath: join(dir, 'outcomes.db') });
    tracker.record('proposal-1', evalResult(40), evalResult(70));

    const roi = tracker.getROI('proposal-1');
    expect(roi).toEqual({ deltaScore: 30, improved: true });
  });
});
