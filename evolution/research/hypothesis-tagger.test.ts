import { describe, expect, it } from 'vitest';
import { HypothesisTagger } from './hypothesis-tagger.js';

describe('HypothesisTagger fallback', () => {
  it('returns generic domain tag when keyword map missing', () => {
    const tagger = new HypothesisTagger();
    expect(tagger.tag('custom topic', 'foo')).toEqual(['foo']);
  });
});
