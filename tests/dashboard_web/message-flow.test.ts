import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

describe('message-flow dashboard', () => {
  it('exists and is valid html', () => {
    const html = readFileSync('scripts/dashboard_web/message-flow.html', 'utf8');
    expect(html.toLowerCase()).toContain('<!doctype html>');
    expect(html.toLowerCase()).toContain('<html');
  });
});
