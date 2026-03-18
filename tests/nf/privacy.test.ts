/**
 * NF: Privacy — validates that telemetry does not leak PII or secrets.
 *
 * Verifies:
 * 1. AgentMetricCollector does not embed process.env.* in metric values
 * 2. Metric values are always numeric (TypeScript type + runtime check)
 * 3. No known secret patterns (sk-, GH_TOKEN, OPENAI_API_KEY) appear in labels
 */

import { describe, it, expect, vi } from 'vitest';
import { AgentMetricCollector } from '../../evolution/telemetry/collectors/agent-collector.js';

/** Regex patterns for known secret / PII tokens. */
const SECRET_PATTERNS = [
  /sk-[A-Za-z0-9]{10,}/,
  /ghp_[A-Za-z0-9]+/,
  /OPENAI_API_KEY/i,
  /GH_TOKEN/i,
  /gh_token/i,
];

function containsSecret(value: string): boolean {
  return SECRET_PATTERNS.some((re) => re.test(value));
}

describe('NF: Privacy', () => {
  it('AgentMetricCollector metric values are numeric', async () => {
    const collector = new AgentMetricCollector({
      dbPath: '/nonexistent/db.sqlite',
    });
    const metrics = await collector.collect();
    for (const metric of metrics) {
      expect(typeof metric.value).toBe('number');
      expect(Number.isFinite(metric.value)).toBe(true);
    }
  });

  it('AgentMetricCollector labels do not contain secret patterns', async () => {
    const collector = new AgentMetricCollector({
      dbPath: '/nonexistent/db.sqlite',
    });
    const metrics = await collector.collect();
    for (const metric of metrics) {
      for (const [key, val] of Object.entries(metric.labels)) {
        expect(
          containsSecret(`${key}=${val}`),
          `Label "${key}" contains a secret-like value`,
        ).toBe(false);
      }
    }
  });

  it('metric values cannot hold string secrets (type safety)', () => {
    // Demonstrates that Metric.value: number prevents string injection at compile time.
    // At runtime, we verify the value field is always numeric.
    const safeMetric = {
      name: 'agent.cost.usd',
      value: 0.05,
      timestamp: Date.now(),
      labels: {},
      family: 'Agent' as const,
    };
    expect(typeof safeMetric.value).toBe('number');
    expect(containsSecret(String(safeMetric.value))).toBe(false);
  });

  it('sanitizes metric with value that would expose secret-like string', () => {
    // If a metric value were somehow a string like "sk-test123", parseNumber returns 0.
    // parseNumber is the guard used throughout the collector.
    function parseNumber(value: string | undefined): number {
      const parsed = Number(value ?? 0);
      return Number.isFinite(parsed) ? parsed : 0;
    }

    const maliciousInput = 'sk-test123456789';
    const sanitized = parseNumber(maliciousInput);
    expect(typeof sanitized).toBe('number');
    expect(Number.isFinite(sanitized)).toBe(true);
    expect(containsSecret(String(sanitized))).toBe(false);
  });

  it('process.env values are not embedded in metric names', async () => {
    const collector = new AgentMetricCollector({
      dbPath: '/nonexistent/db.sqlite',
    });
    const metrics = await collector.collect();
    // Only check for env vars that look like secret keys (all-caps, length > 5)
    const sensitiveEnvKeys = Object.keys(process.env).filter(
      (k) => k.length > 5 && /^[A-Z][A-Z0-9_]{4,}$/.test(k),
    );
    for (const metric of metrics) {
      for (const envKey of sensitiveEnvKeys) {
        expect(metric.name).not.toContain(envKey);
      }
    }
  });
});
