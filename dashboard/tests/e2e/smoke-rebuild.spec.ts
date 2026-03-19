import { test, expect } from '@playwright/test';

const BASE = 'http://localhost:8420';

test.describe('Dashboard Rebuild Smoke Tests', () => {
  test('loads and shows mn-app-shell', async ({ page }) => {
    await page.goto(BASE);
    await expect(page.locator('mn-app-shell')).toBeVisible();
  });

  test('navigate to each section', async ({ page }) => {
    await page.goto(BASE);
    const sections = ['overview', 'plans', 'mesh', 'brain', 'ideas', 'ipc', 'admin', 'terminal'];
    for (const section of sections) {
      await page.click(`[data-view="${section}"]`);
      await expect(page.locator('#view-container')).not.toBeEmpty();
    }
  });

  test('overview shows KPI strip', async ({ page }) => {
    await page.goto(BASE);
    // Wait for KPI strip to render
    await page.waitForSelector('.mn-kpi-strip', { timeout: 5000 });
    const kpis = await page.locator('.mn-kpi-strip .mn-card').count();
    expect(kpis).toBeGreaterThanOrEqual(1);
  });

  test('plans table loads rows', async ({ page }) => {
    await page.goto(BASE + '#plans');
    await page.click('[data-view="plans"]');
    await page.waitForSelector('mn-data-table', { timeout: 5000 });
  });

  test('mesh shows peer cards', async ({ page }) => {
    await page.goto(BASE + '#mesh');
    await page.click('[data-view="mesh"]');
    await page.waitForSelector('mn-tabs', { timeout: 5000 });
  });

  test('theme toggle switches themes', async ({ page }) => {
    await page.goto(BASE);
    const initialTheme = await page.getAttribute('html', 'data-theme');
    // Click theme rotary
    await page.click('mn-theme-rotary');
    const newTheme = await page.getAttribute('html', 'data-theme');
    expect(newTheme).not.toBe(initialTheme);
  });

  test('command palette opens on Cmd+K', async ({ page }) => {
    await page.goto(BASE);
    await page.keyboard.press('Meta+k');
    await expect(page.locator('mn-command-palette')).toBeVisible();
  });
});
