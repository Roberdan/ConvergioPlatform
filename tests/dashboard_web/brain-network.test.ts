import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

describe('brain-network dashboard', () => {
  it('exists, is valid html, and references /ws/brain', () => {
    const html = readFileSync('scripts/dashboard_web/brain-network.html', 'utf8');
    expect(html.toLowerCase()).toContain('<!doctype html>');
    expect(html).toContain('/ws/brain');
  });
});
