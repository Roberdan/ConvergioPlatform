import { beforeEach, describe, expect, it, vi } from 'vitest';

const { spawnSyncMock } = vi.hoisted(() => ({
  spawnSyncMock: vi.fn(),
}));

vi.mock('child_process', () => ({
  spawnSync: spawnSyncMock,
}));

import { WebSearcher } from './web-searcher.js';
import { HypothesisTagger } from './hypothesis-tagger.js';

const DB_PATH = '/tmp/evolution-research-test.db';

describe('HypothesisTagger', () => {
  it('maps known domain keywords to tags', () => {
    const tagger = new HypothesisTagger();
    expect(tagger.tag('latency got worse', 'latency')).toContain('performance');
    expect(tagger.tag('bundle grew', 'bundle')).toContain('frontend');
    expect(tagger.tag('agent tokens rose', 'agent')).toContain('llm');
  });
});

describe('WebSearcher', () => {
  beforeEach(() => {
    spawnSyncMock.mockReset();
  });

  it('returns parsed research results with tags', async () => {
    spawnSyncMock.mockImplementation((cmd: string) => {
      if (cmd === 'sqlite3') {
        return { status: 0, stdout: '', stderr: '' };
      }
      return {
        status: 0,
        stdout: JSON.stringify({
          AbstractText: 'Edge caching helps latency.',
          RelatedTopics: [
            { Text: 'CDN can reduce origin traffic.' },
            { Text: 'Use compression for payload size.' },
            { Text: 'Cache headers improve hit ratio.' },
          ],
        }),
        stderr: '',
      };
    });

    const searcher = new WebSearcher({ dbPath: DB_PATH, now: () => 1000 });
    const rows = await searcher.search('edge caching latency', ['performance']);

    expect(rows).toHaveLength(4);
    expect(rows[0]).toMatchObject({
      query: 'edge caching latency',
      title: 'DuckDuckGo Abstract',
      tags: ['performance'],
      cachedAt: 1000,
    });
  });

  it('blocks when exceeding 3 requests per minute', async () => {
    spawnSyncMock.mockImplementation((cmd: string) => {
      if (cmd === 'sqlite3') {
        return { status: 0, stdout: '', stderr: '' };
      }
      return {
        status: 0,
        stdout: JSON.stringify({ AbstractText: 'ok', RelatedTopics: [] }),
        stderr: '',
      };
    });

    let now = 0;
    const searcher = new WebSearcher({ dbPath: DB_PATH, now: () => now });

    await searcher.search('q1', ['a']);
    await searcher.search('q2', ['a']);
    await searcher.search('q3', ['a']);
    await expect(searcher.search('q4', ['a'])).rejects.toThrow('rate limit');

    now = 70_000;
    await expect(searcher.search('q5', ['a'])).resolves.toHaveLength(1);
  });
});
