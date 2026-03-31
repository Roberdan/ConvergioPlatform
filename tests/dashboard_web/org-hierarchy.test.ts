import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

describe('org-hierarchy dashboard', () => {
  it('exists, is valid html, and references /api/orgs', () => {
    const html = readFileSync('scripts/dashboard_web/org-hierarchy.html', 'utf8');
    expect(html.toLowerCase()).toContain('<!doctype html>');
    expect(html).toContain('/api/orgs');
  });
});
