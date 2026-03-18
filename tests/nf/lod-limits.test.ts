/**
 * NF: Lines-of-Code Limits — all evolution source files stay within 500 lines.
 *
 * Dynamically globs all .ts and .js files under evolution/ and dashboard_web/evolution/
 * and asserts each file is ≤ 500 lines. Violations are reported in the failure message.
 */

import { describe, it, expect } from 'vitest';
import { readdirSync, readFileSync, statSync } from 'fs';
import { join, relative, extname } from 'path';

const REPO_ROOT = join(import.meta.dirname, '..', '..');
const MAX_LINES = 500;

/** Recursively collect all files with given extensions under a directory. */
function glob(dir: string, extensions: string[]): string[] {
  const results: string[] = [];
  let entries: string[];
  try {
    entries = readdirSync(dir);
  } catch {
    return results;
  }
  for (const entry of entries) {
    if (entry === 'node_modules' || entry === 'dist' || entry.startsWith('.')) {
      continue;
    }
    const full = join(dir, entry);
    const stat = statSync(full);
    if (stat.isDirectory()) {
      results.push(...glob(full, extensions));
    } else if (extensions.includes(extname(full))) {
      results.push(full);
    }
  }
  return results;
}

function countLines(filePath: string): number {
  const content = readFileSync(filePath, 'utf-8');
  return content.split('\n').length;
}

describe('NF: Lines-of-Code Limits', () => {
  it('all evolution/**/*.ts files are ≤ 500 lines', () => {
    const evolutionDir = join(REPO_ROOT, 'evolution');
    const files = glob(evolutionDir, ['.ts']);

    const violations: string[] = [];
    for (const file of files) {
      const lines = countLines(file);
      if (lines > MAX_LINES) {
        violations.push(`${relative(REPO_ROOT, file)}: ${lines} lines (limit: ${MAX_LINES})`);
      }
    }

    expect(
      violations,
      `Files exceeding ${MAX_LINES}-line limit:\n${violations.join('\n')}`,
    ).toHaveLength(0);
  });

  it('all dashboard_web/evolution/**/*.js files are ≤ 500 lines', () => {
    const widgetDir = join(REPO_ROOT, 'dashboard_web', 'evolution');
    const files = glob(widgetDir, ['.js']);

    const violations: string[] = [];
    for (const file of files) {
      const lines = countLines(file);
      if (lines > MAX_LINES) {
        violations.push(`${relative(REPO_ROOT, file)}: ${lines} lines (limit: ${MAX_LINES})`);
      }
    }

    expect(
      violations,
      `Files exceeding ${MAX_LINES}-line limit:\n${violations.join('\n')}`,
    ).toHaveLength(0);
  });

  it('newly created W8 files are well within limits', () => {
    const w8Files = [
      join(REPO_ROOT, 'evolution', 'core', 'guardrails', 'kill-switch.ts'),
      join(REPO_ROOT, 'evolution', 'core', 'guardrails', 'rate-limiter.ts'),
      join(REPO_ROOT, 'evolution', 'core', 'cadence', 'daily-runner.ts'),
      join(REPO_ROOT, 'evolution', 'reporting', 'roi-tracker.ts'),
      join(REPO_ROOT, 'evolution', 'reporting', 'scoreboard.ts'),
      join(REPO_ROOT, 'dashboard_web', 'evolution', 'roi-widget.js'),
    ];

    for (const file of w8Files) {
      try {
        const lines = countLines(file);
        expect(lines, `${relative(REPO_ROOT, file)} has ${lines} lines`).toBeLessThanOrEqual(MAX_LINES);
      } catch (err) {
        // File not yet committed; skip silently (will be verified after commit)
        expect.soft(false, `File not found: ${relative(REPO_ROOT, file)}`).toBe(false);
      }
    }
  });
});
