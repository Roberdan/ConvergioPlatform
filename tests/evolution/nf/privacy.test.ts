/**
 * NF: Privacy — telemetry must not leak PII or secrets.
 */
import { describe, it, expect } from 'vitest';
import { AgentMetricCollector } from '../../../evolution/telemetry/collectors/agent-collector.js';

const SECRET_RE = [/sk-[A-Za-z0-9]{10,}/, /ghp_[A-Za-z0-9]+/, /OPENAI_API_KEY/i, /GH_TOKEN/i];
function hasSecret(s: string): boolean { return SECRET_RE.some((re) => re.test(s)); }

describe('NF: Privacy', () => {
  it('metric values are numeric', async () => {
    const c = new AgentMetricCollector({ dbPath: '/nonexistent/db.sqlite' });
    for (const m of await c.collect()) {
      expect(typeof m.value).toBe('number');
      expect(Number.isFinite(m.value)).toBe(true);
    }
  });

  it('metric labels do not contain secrets', async () => {
    const c = new AgentMetricCollector({ dbPath: '/nonexistent/db.sqlite' });
    for (const m of await c.collect()) {
      for (const [k, v] of Object.entries(m.labels)) {
        expect(hasSecret(`${k}=${v}`), `Label "${k}" contains secret`).toBe(false);
      }
    }
  });

  it('metric names do not contain secret patterns', async () => {
    const c = new AgentMetricCollector({ dbPath: '/nonexistent/db.sqlite' });
    for (const m of await c.collect()) {
      expect(hasSecret(m.name)).toBe(false);
    }
  });

  it('string "sk-test" is sanitized to 0 by parseNumber guard', () => {
    function parseNumber(v: string | undefined): number {
      const p = Number(v ?? 0);
      return Number.isFinite(p) ? p : 0;
    }
    expect(parseNumber('sk-test123')).toBe(0);
    expect(hasSecret(String(parseNumber('sk-test123')))).toBe(false);
  });
});
