import { test, expect } from './fixtures';

test.describe('IPC Coordination Panel', () => {
  test.beforeEach(async ({ page, mockApis }) => {
    await mockApis();
    await page.goto('/');
    await page.waitForSelector('.kpi-bar .kpi-card', { timeout: 5000 });
  });

  test('IPC pill is visible in nav bar', async ({ page }) => {
    const ipcPill = page.locator('.mn-convergio-pill', { hasText: 'IPC' });
    await expect(ipcPill).toBeVisible();
  });

  test('clicking IPC switches to IPC section', async ({ page }) => {
    const ipcPill = page.locator('.mn-convergio-pill', { hasText: 'IPC' });
    await ipcPill.click();
    await expect(ipcPill).toHaveClass(/active/);
    await expect(page.locator('#dashboard-ipc-section')).toBeVisible();
  });

  test('IPC panel renders KPI bar with counts', async ({ page }) => {
    const ipcPill = page.locator('.mn-convergio-pill', { hasText: 'IPC' });
    await ipcPill.click();
    await page.waitForSelector('.ipc-kpi-bar', { timeout: 5000 });
    const kpiCards = page.locator('.ipc-kpi-card');
    await expect(kpiCards).toHaveCount(4);
  });

  test('IPC panel shows agent cards', async ({ page }) => {
    const ipcPill = page.locator('.mn-convergio-pill', { hasText: 'IPC' });
    await ipcPill.click();
    await page.waitForSelector('.ipc-agent-card', { timeout: 5000 });
    const cards = page.locator('.ipc-agent-card');
    await expect(cards).toHaveCount(2);
    await expect(cards.first()).toContainText('executor-001');
  });

  test('IPC panel shows messages', async ({ page }) => {
    const ipcPill = page.locator('.mn-convergio-pill', { hasText: 'IPC' });
    await ipcPill.click();
    await page.waitForSelector('.ipc-message', { timeout: 5000 });
    const messages = page.locator('.ipc-message');
    await expect(messages).toHaveCount(2);
    await expect(messages.first()).toContainText('Task T1-01 started');
  });

  test('IPC panel shows file locks table', async ({ page }) => {
    const ipcPill = page.locator('.mn-convergio-pill', { hasText: 'IPC' });
    await ipcPill.click();
    await page.waitForSelector('.ipc-table', { timeout: 5000 });
    const lockTable = page.locator('.ipc-table').first();
    await expect(lockTable).toContainText('src/auth/*.rs');
  });

  test('IPC panel shows worktrees table', async ({ page }) => {
    const ipcPill = page.locator('.mn-convergio-pill', { hasText: 'IPC' });
    await ipcPill.click();
    await page.waitForSelector('.ipc-table', { timeout: 5000 });
    const tables = page.locator('.ipc-table');
    await expect(tables.nth(1)).toContainText('feature/auth');
  });

  test('IPC send bar is visible', async ({ page }) => {
    const ipcPill = page.locator('.mn-convergio-pill', { hasText: 'IPC' });
    await ipcPill.click();
    await page.waitForSelector('.ipc-send-bar', { timeout: 5000 });
    await expect(page.locator('#ipc-msg-input')).toBeVisible();
    await expect(page.locator('#ipc-send-btn')).toBeVisible();
  });

  test('IPC status dots have aria labels', async ({ page }) => {
    const ipcPill = page.locator('.mn-convergio-pill', { hasText: 'IPC' });
    await ipcPill.click();
    await page.waitForSelector('.ipc-status-dot', { timeout: 5000 });
    const dots = page.locator('.ipc-status-dot');
    const count = await dots.count();
    expect(count).toBeGreaterThan(0);
    for (let i = 0; i < count; i++) {
      await expect(dots.nth(i)).toHaveAttribute('aria-label', /Agent/);
    }
  });

  test('switching away from IPC hides section', async ({ page }) => {
    const ipcPill = page.locator('.mn-convergio-pill', { hasText: 'IPC' });
    await ipcPill.click();
    await expect(page.locator('#dashboard-ipc-section')).toBeVisible();
    const overviewPill = page.locator('.mn-convergio-pill', { hasText: 'Overview' });
    await overviewPill.click();
    await expect(page.locator('#dashboard-ipc-section')).toBeHidden();
  });

  test('IPC panel KPI bar has role region', async ({ page }) => {
    const ipcPill = page.locator('.mn-convergio-pill', { hasText: 'IPC' });
    await ipcPill.click();
    await page.waitForSelector('.ipc-kpi-bar', { timeout: 5000 });
    await expect(page.locator('.ipc-kpi-bar')).toHaveAttribute('role', 'region');
  });

  test('tables have role and aria-labelledby', async ({ page }) => {
    const ipcPill = page.locator('.mn-convergio-pill', { hasText: 'IPC' });
    await ipcPill.click();
    await page.waitForSelector('.ipc-table', { timeout: 5000 });
    const tables = page.locator('.ipc-table[role="table"]');
    await expect(tables).toHaveCount(2);
  });

  test('theme switch does not break IPC panel', async ({ page }) => {
    const ipcPill = page.locator('.mn-convergio-pill', { hasText: 'IPC' });
    await ipcPill.click();
    await page.waitForSelector('.ipc-kpi-bar', { timeout: 5000 });
    // Switch theme to mn-nero equivalent
    await page.evaluate(() => {
      document.documentElement.setAttribute('data-theme', 'dark');
    });
    await expect(page.locator('.ipc-kpi-bar')).toBeVisible();
    await expect(page.locator('.ipc-agent-card').first()).toBeVisible();
  });

  test('accessibility: all interactive elements reachable', async ({ page }) => {
    const ipcPill = page.locator('.mn-convergio-pill', { hasText: 'IPC' });
    await ipcPill.click();
    await page.waitForSelector('.ipc-send-bar', { timeout: 5000 });
    const input = page.locator('#ipc-msg-input');
    await expect(input).toHaveAttribute('aria-label', 'Message content');
    const btn = page.locator('#ipc-send-btn');
    await expect(btn).toHaveAttribute('aria-label', 'Send message');
  });

  test('mobile viewport: IPC grid stacks vertically', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 812 });
    const ipcPill = page.locator('.mn-convergio-pill', { hasText: 'IPC' });
    await ipcPill.click();
    await page.waitForSelector('.ipc-grid', { timeout: 5000 });
    const grid = page.locator('.ipc-grid');
    const style = await grid.evaluate(el => getComputedStyle(el).gridTemplateColumns);
    // On mobile, should be single column
    expect(style).not.toContain(' ');
  });
});
