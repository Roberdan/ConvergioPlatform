import { afterEach, describe, expect, it } from 'vitest';
import { mkdtempSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { HypothesisStore } from './hypothesis-store.js';

describe('HypothesisStore', () => {
  const dirs: string[] = [];

  afterEach(() => {
    for (const dir of dirs.splice(0)) {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  it('saves and queries recent hypotheses', () => {
    const dir = mkdtempSync(join(tmpdir(), 'hyp-store-'));
    dirs.push(dir);

    const store = new HypothesisStore({ dbPath: join(dir, 'hyp.db') });
    store.save({
      id: 'h-1',
      proposalId: 'p-1',
      source: 'engine',
      tags: ['latency'],
      confidence: 0.7,
      createdAt: Date.now(),
    });

    const recent = store.findRecent(7);
    expect(recent.length).toBe(1);
    expect(recent[0]?.proposalId).toBe('p-1');

    store.markTested('h-1', 'confirmed');
    const updated = store.findRecent(7)[0];
    expect(updated?.outcome).toBe('confirmed');
  });
});
