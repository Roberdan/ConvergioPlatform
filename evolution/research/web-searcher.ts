import * as childProcess from 'child_process';
import { createHash } from 'crypto';

const DEFAULT_DB_PATH = `${process.env.HOME ?? ''}/.claude/data/dashboard.db`;
const WEEK_MS = 7 * 24 * 60 * 60 * 1000;
const MINUTE_MS = 60 * 1000;
const RATE_LIMIT_PER_MINUTE = 3;

interface CacheRow {
  resultJson: string;
  cachedAt: number;
}

interface SearcherOptions {
  dbPath?: string;
  now?: () => number;
}

export interface ResearchResult {
  query: string;
  title: string;
  snippet: string;
  tags: string[];
  cachedAt: number;
  hypothesisRef?: string;
}

export class WebSearcher {
  private readonly dbPath: string;
  private readonly now: () => number;
  private readonly requestHistory: number[] = [];

  constructor(options: SearcherOptions = {}) {
    this.dbPath = options.dbPath ?? DEFAULT_DB_PATH;
    this.now = options.now ?? (() => Date.now());
    this.ensureSchema();
  }

  async search(query: string, tags: string[]): Promise<ResearchResult[]> {
    this.guardRateLimit();
    const now = this.now();
    const queryHash = sha256(query.trim().toLowerCase());

    const cached = this.readCache(queryHash, now);
    if (cached) {
      const parsed = JSON.parse(cached.resultJson) as Array<{ title: string; snippet: string }>;
      return parsed.map((item) => ({
        query,
        title: item.title,
        snippet: item.snippet,
        tags: [...tags],
        cachedAt: cached.cachedAt,
      }));
    }

    const response = childProcess.spawnSync(
      'curl',
      ['-sS', `https://api.duckduckgo.com/?q=${encodeURIComponent(query)}&format=json&no_html=1`],
      { encoding: 'utf8' },
    );

    if (response.status !== 0) {
      throw new Error(response.stderr || 'DuckDuckGo request failed');
    }

    const payload = JSON.parse(response.stdout || '{}') as {
      AbstractText?: string;
      RelatedTopics?: Array<{ Text?: string }>;
    };

    const rows: Array<{ title: string; snippet: string }> = [];
    if (payload.AbstractText?.trim()) {
      rows.push({ title: 'DuckDuckGo Abstract', snippet: payload.AbstractText.trim() });
    }

    const related = payload.RelatedTopics ?? [];
    for (const topic of related.slice(0, 3)) {
      if (topic.Text?.trim()) {
        rows.push({ title: 'Related Topic', snippet: topic.Text.trim() });
      }
    }

    if (rows.length === 0) {
      rows.push({ title: 'No Result', snippet: 'No instant answer content was returned.' });
    }

    this.writeCache(queryHash, JSON.stringify(rows), now, now + WEEK_MS);

    return rows.map((item) => ({
      query,
      title: item.title,
      snippet: item.snippet,
      tags: [...tags],
      cachedAt: now,
    }));
  }

  private guardRateLimit(): void {
    const now = this.now();
    const windowStart = now - MINUTE_MS;

    while (this.requestHistory.length > 0 && this.requestHistory[0] < windowStart) {
      this.requestHistory.shift();
    }

    if (this.requestHistory.length >= RATE_LIMIT_PER_MINUTE) {
      throw new Error('WebSearcher rate limit exceeded (3 requests/minute)');
    }

    this.requestHistory.push(now);
  }

  private ensureSchema(): void {
    this.sqliteExec(`
      CREATE TABLE IF NOT EXISTS research_cache (
        query_hash TEXT PRIMARY KEY,
        result_json TEXT NOT NULL,
        cached_at INTEGER NOT NULL,
        expires_at INTEGER NOT NULL
      );
      CREATE INDEX IF NOT EXISTS idx_research_cache_expires ON research_cache(expires_at);
    `);
  }

  private readCache(queryHash: string, now: number): CacheRow | null {
    const sql = [
      'SELECT result_json, cached_at',
      'FROM research_cache',
      `WHERE query_hash = '${escapeSql(queryHash)}' AND expires_at > ${now}`,
      'LIMIT 1;',
    ].join(' ');

    const row = this.sqliteQuery(sql);
    if (!row) return null;
    const [resultJson, cachedAtRaw] = row.split('|');

    return {
      resultJson,
      cachedAt: Number(cachedAtRaw),
    };
  }

  private writeCache(queryHash: string, resultJson: string, cachedAt: number, expiresAt: number): void {
    this.sqliteExec(
      `INSERT OR REPLACE INTO research_cache(query_hash, result_json, cached_at, expires_at) VALUES (` +
        `'${escapeSql(queryHash)}','${escapeSql(resultJson)}',${cachedAt},${expiresAt});`,
    );
  }

  private sqliteExec(sql: string): void {
    const result = childProcess.spawnSync('sqlite3', [this.dbPath, sql], { encoding: 'utf8' });
    if (result.status !== 0) {
      throw new Error(result.stderr || 'sqlite exec failed');
    }
  }

  private sqliteQuery(sql: string): string {
    const result = childProcess.spawnSync('sqlite3', ['-separator', '|', this.dbPath, sql], {
      encoding: 'utf8',
    });
    if (result.status !== 0) {
      throw new Error(result.stderr || 'sqlite query failed');
    }
    return result.stdout.trim();
  }
}

function sha256(input: string): string {
  return createHash('sha256').update(input).digest('hex');
}

function escapeSql(value: string): string {
  return value.replace(/'/g, "''");
}
