import { describe, expect, it } from 'vitest';
import { HypothesisTagger } from '../../../src/ts/evolution/research/hypothesis-tagger.ts';

describe('research hypothesis tagging', () => {
  it('marks web knowledge as HYPOTHESIS with source metadata', () => {
    const tagger = new HypothesisTagger();
    const tags = tagger.tag('latency improved', 'latency');
    const meta = tagger.createMetadata('https://example.com');

    expect(tags).toContain('performance');
    expect(meta.status).toBe('HYPOTHESIS');
    expect(meta.sourceUrl).toContain('example.com');
    expect(meta.retrievalDate).toContain('T');
    expect(meta.confidence).toBeGreaterThan(0);
  });
});
