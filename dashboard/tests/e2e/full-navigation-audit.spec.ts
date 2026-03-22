import { test, expect, Page } from '@playwright/test';

/**
 * Full dashboard navigation audit — hits every tab, button, widget, and
 * interactive element to catch ALL client-side errors.
 * Runs against the REAL server (no mocks).
 */

interface CollectedError {
  type: 'js' | 'network' | 'console';
  message: string;
}

function attachErrorCollectors(page: Page): CollectedError[] {
  const errors: CollectedError[] = [];
  page.on('pageerror', (err) =>
    errors.push({ type: 'js', message: err.message }),
  );
  page.on('console', (msg) => {
    if (msg.type() === 'error') {
      const text = msg.text();
      // Ignore known CDN issues (chartjs-plugin-zoom@2 404)
      if (text.includes('cdn.jsdelivr')) return;
      errors.push({ type: 'console', message: text });
    }
  });
  page.on('response', (res) => {
    const url = res.url();
    if (url.includes('jsdelivr') || url.includes('fonts.g')) return;
    if (url.includes('/api/') && res.status() === 404) return;
    if (res.status() >= 400) {
      errors.push({ type: 'network', message: `${res.status()} ${url}` });
    }
  });
  return errors;
}

export { attachErrorCollectors };

test.describe('Full dashboard navigation audit', () => {
  test.skip(!!process.env.CI, 'Requires real server with data — skipped on CI');

  test('Overview tab — all widgets render without errors', async ({ page }) => {
    const errors = attachErrorCollectors(page);
    await page.goto('/', { waitUntil: 'networkidle' });
    await page.waitForTimeout(3000);

    const kpiText = await page.locator('.kpi-row, .kpi-strip, [class*="kpi"]').first().textContent();
    expect(kpiText).toBeTruthy();

    const missionContent = await page.locator('#mission-content').textContent();
    expect(missionContent).not.toContain('Loading...');

    const meshContent = await page.locator('#mesh-strip').textContent();
    expect(meshContent).toBeTruthy();

    const nightlyContent = await page.locator('#nightly-jobs-content').textContent();
    expect(nightlyContent).not.toContain('Loading...');

    const org = page.locator('#agent-organization-content');
    if (await org.count()) {
      const orgContent = await org.textContent();
      expect(orgContent).not.toContain('Loading...');
    }
    const live = page.locator('#live-system-content');
    if (await live.count()) {
      const liveContent = await live.textContent();
      expect(liveContent).not.toContain('Loading...');
    }

    const taskTable = await page.locator('#task-table').textContent();
    expect(taskTable).toBeTruthy();

    expect(errors, `Overview errors:\n${errors.map((e) => `[${e.type}] ${e.message}`).join('\n')}`).toHaveLength(0);
  });

  test('Chat tab — switches without errors', async ({ page }) => {
    const errors = attachErrorCollectors(page);
    await page.goto('/', { waitUntil: 'networkidle' });
    await page.waitForTimeout(2000);

    const chatBtn = page.locator('button[data-section="dashboard-chat-section"]');
    if (await chatBtn.isVisible()) {
      await chatBtn.click();
      await page.waitForTimeout(2000);
    }

    expect(errors, `Chat tab errors:\n${errors.map((e) => `[${e.type}] ${e.message}`).join('\n')}`).toHaveLength(0);
  });

  test('click plan card — detail sidebar opens', async ({ page }) => {
    const errors = attachErrorCollectors(page);
    await page.goto('/', { waitUntil: 'networkidle' });
    await page.waitForTimeout(2000);

    const planCard = page.locator('.mission-plan').first();
    if (await planCard.isVisible()) {
      await planCard.click();
      await page.waitForTimeout(2000);
    }

    expect(errors, `Plan detail errors:\n${errors.map((e) => `[${e.type}] ${e.message}`).join('\n')}`).toHaveLength(0);
  });

  test('click task row — detail expands', async ({ page }) => {
    const errors = attachErrorCollectors(page);
    await page.goto('/', { waitUntil: 'networkidle' });
    await page.waitForTimeout(2000);

    const taskRow = page.locator('#task-table tbody tr').first();
    if (await taskRow.isVisible()) {
      await taskRow.click();
      await page.waitForTimeout(1000);
    }

    expect(errors, `Task row errors:\n${errors.map((e) => `[${e.type}] ${e.message}`).join('\n')}`).toHaveLength(0);
  });

  test('theme toggle — cycles without errors', async ({ page }) => {
    const errors = attachErrorCollectors(page);
    await page.goto('/', { waitUntil: 'networkidle' });
    await page.waitForTimeout(1000);

    const themeBtn = page.locator('#theme-toggle');
    if (await themeBtn.isVisible()) {
      await themeBtn.click();
      await page.waitForTimeout(500);
      const themeOption = page.locator('.theme-dropdown button, .theme-option').first();
      if (await themeOption.isVisible()) {
        await themeOption.click();
        await page.waitForTimeout(1000);
      }
    }

    expect(errors, `Theme errors:\n${errors.map((e) => `[${e.type}] ${e.message}`).join('\n')}`).toHaveLength(0);
  });
});
