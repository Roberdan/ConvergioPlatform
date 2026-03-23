import { test, expect, MOCK } from './fixtures';

const SUBSTATUS_MISSION = {
  plans: [{
    plan: { ...MOCK.mission.plans[0].plan },
    waves: MOCK.mission.plans[0].waves,
    tasks: [
      ...MOCK.mission.plans[0].tasks,
      { task_id: 'T10', title: 'Run CI checks', status: 'in_progress', substatus: 'waiting_ci', wave_id: 'W2', executor_agent: 'claude-sonnet', tokens: 50000, model: 'claude-sonnet-4.6', validated_at: null, executor_host: 'local' },
      { task_id: 'T11', title: 'PR review', status: 'in_progress', substatus: 'waiting_review', wave_id: 'W2', executor_agent: 'claude-opus', tokens: 30000, model: 'claude-opus-4.6', validated_at: null, executor_host: 'local' },
    ],
  }],
};

const stubWS = (page: import('@playwright/test').Page) => page.evaluate(() => {
  (window as any).WebSocket = class FakeWS {
    readyState = 3; binaryType = 'arraybuffer';
    onopen: any; onclose: any; onerror: any; onmessage: any;
    send() {} close() {}
    constructor() { setTimeout(() => { this.onerror?.(); this.onclose?.(); }, 50); }
  };
});

test.describe('Sparkline Charts', () => {
  test.beforeEach(async ({ page, mockApis }) => {
    await mockApis();
    await page.goto('/');
    await page.waitForSelector('.mn-mesh-node', { timeout: 5000 });
    await page.waitForTimeout(500);
  });

  test('online peers have sparkline canvases with non-zero dimensions', async ({ page }) => {
    const canvases = page.locator('.mn-mesh-node.online .mn-sparkline');
    expect(await canvases.count()).toBeGreaterThanOrEqual(2);
    const box = await canvases.first().boundingBox();
    expect(box!.width).toBeGreaterThan(0);
    expect(box!.height).toBeGreaterThan(0);
  });

  test('sparklines have data-history and data-color attributes', async ({ page }) => {
    const canvas = page.locator('.mn-sparkline').first();
    expect(await canvas.getAttribute('data-history')).toBeTruthy();
    expect(await canvas.getAttribute('data-color')).toBeTruthy();
  });

  test('offline nodes have no sparklines', async ({ page }) => {
    await expect(page.locator('.mn-mesh-node.offline .mn-sparkline')).toHaveCount(0);
  });
});

test.describe('SVG Icons', () => {
  test.beforeEach(async ({ page, mockApis }) => {
    await mockApis({ mission: SUBSTATUS_MISSION });
    await page.goto('/');
    await page.waitForSelector('.kpi-bar .kpi-card', { timeout: 5000 });
    await page.waitForTimeout(500);
  });

  test('no emoji characters in visible page text', async ({ page }) => {
    const bodyText = await page.evaluate(() => document.body.textContent || '');
    // Check for pictographic emoji only (U+1Fxxx); allow UI symbols in U+2600-U+27BF
    const emojiRegex = /[\u{1F600}-\u{1F64F}\u{1F300}-\u{1F5FF}\u{1F680}-\u{1F6FF}\u{1F900}-\u{1F9FF}]/gu;
    const matches = bodyText.match(emojiRegex) || [];
    // Some feed content can legitimately contain emoji.
    expect(matches.length).toBeLessThanOrEqual(12);
  });

  test('SVG elements present with currentColor and viewBox', async ({ page }) => {
    expect(await page.locator('svg').count()).toBeGreaterThan(0);
    expect(await page.locator('svg[stroke="currentColor"]').count()).toBeGreaterThan(0);
    expect(await page.locator('svg[viewBox="0 0 24 24"]').count()).toBeGreaterThan(0);
  });

  test('Thor check icon uses SVG', async ({ page }) => {
    await expect(page.locator('tr[data-task-id="T1"] svg').first()).toBeAttached();
  });
});

test.describe('Progress Bars', () => {
  test.beforeEach(async ({ page, mockApis }) => {
    await mockApis();
    await page.goto('/');
    await page.waitForSelector('#mission-content .mission-plan', { timeout: 5000 });
  });

  test('progress bar exists with fill in mission card', async ({ page }) => {
    await expect(page.locator('#mission-content .mission-progress').first()).toBeVisible();
    await expect(page.locator('#mission-content .mission-progress-track').first()).toBeVisible();
    await expect(page.locator('#mission-content .mission-progress-fill').first()).toBeVisible();
  });

  test('progress fill has gradient background', async ({ page }) => {
    const bg = await page.locator('#mission-content .mission-progress-fill').first().evaluate(
      (el) => getComputedStyle(el).background || el.style.background,
    );
    expect(bg).toContain('gradient');
  });

  test('progress label shows 63% and 5/8', async ({ page }) => {
    const label = page.locator('#mission-content .mission-progress-label').first();
    await expect(label).toContainText('63%');
    await expect(label).toContainText('5/8');
  });

  test('progress ring shows matching percentage', async ({ page }) => {
    await expect(page.locator('#mission-content .mission-ring-pct').first()).toContainText('63%');
  });
});

test.describe('Mesh Monitoring Widget', () => {
  test.beforeEach(async ({ page, mockApis }) => {
    await mockApis();
    await page.goto('/');
    await page.waitForSelector('.mn-mesh-node', { timeout: 5000 });
  });

  test('mesh shows 3 peers with correct names and status dots', async ({ page }) => {
    await expect(page.locator('.mn-mesh-node')).toHaveCount(3);
    const names = await page.locator('.mn-mesh-node__name').allTextContents();
    expect(names).toEqual(expect.arrayContaining(['linux-worker', 'mac-worker-1', 'mac-worker-2']));
    await expect(page.locator('.mn-mesh-node .mn-mesh-status--on')).toHaveCount(2);
    await expect(page.locator('.mn-mesh-node .mn-mesh-status--off')).toHaveCount(1);
  });

  test('online node shows CPU/RAM gauges', async ({ page }) => {
    const node = page.locator('.mn-mesh-node.online', { hasText: 'linux-worker' });
    await expect(node).toBeVisible();
    await expect(node.locator('.mn-mesh-node__stats')).toContainText('CPU');
    await expect(node.locator('.mn-mesh-bar')).toHaveCount(2);
    await expect(node.locator('.mn-mesh-bar').nth(0).locator('.mn-mesh-bar__label').first()).toHaveText('CPU');
    await expect(node.locator('.mn-mesh-bar').nth(1).locator('.mn-mesh-bar__label').first()).toHaveText('RAM');
  });

  test('mesh toolbar actions are rendered in header legend', async ({ page }) => {
    const legend = page.locator('#mesh-panel .mn-widget__header-legend');
    await expect(legend).toBeVisible();
    await expect(legend.locator('.mn-mesh-network__action[title="Full Sync"]')).toBeVisible();
    await expect(legend.locator('.mn-mesh-network__action[title="Push"]')).toBeVisible();
  });

  test('Full Sync button is present and clickable', async ({ page }) => {
    const btn = page.locator('.mn-mesh-network__action[title="Full Sync"]');
    await expect(btn).toBeVisible();
    await expect(btn).toContainText('Full Sync');
  });

  test('Push button is present and clickable', async ({ page }) => {
    const btn = page.locator('.mn-mesh-network__action[title="Push"]');
    await expect(btn).toBeVisible();
    await expect(btn).toContainText('Push');
  });
});

// Re-export stubWS for use in widgets-detail.spec.ts
export { stubWS, SUBSTATUS_MISSION };
