// Playwright smoke test for dashboard 3-zone layout restructure.
// Verifies zones, brain strip, terminal drawer, chat drawer, and evolution.

import { test, expect, mockApis } from './e2e/fixtures.ts';

test.describe('Dashboard Restructure', () => {
  test.beforeEach(async ({ mockApis }) => {
    await mockApis();
  });

  test('3-zone layout renders: command strip, main area, brain strip', async ({ page }) => {
    await page.goto('/');

    // Zone 1: Command Strip (KPI bar / header)
    const commandStrip = page.locator('.cr-command-strip');
    await expect(commandStrip).toBeVisible();
    await expect(commandStrip.locator('#command-strip-kpi')).toBeAttached();

    // Zone 2: Main view container
    const mainArea = page.locator('#view-container');
    await expect(mainArea).toBeVisible();

    // Zone 3: Brain strip
    const brainStrip = page.locator('#brain-strip');
    await expect(brainStrip).toBeVisible();
    await expect(brainStrip.locator('#brain-strip-content')).toBeAttached();
  });

  test('brain strip toggle collapses and expands', async ({ page }) => {
    await page.goto('/');

    const brainStrip = page.locator('#brain-strip');
    const toggleBtn = page.locator('#brain-strip-toggle');

    // Initially expanded
    await expect(brainStrip).toHaveAttribute('data-brain-strip', 'expanded');

    // Click toggle to collapse
    await toggleBtn.click();
    await expect(brainStrip).toHaveAttribute('data-brain-strip', 'collapsed');

    // Click again to expand
    await toggleBtn.click();
    await expect(brainStrip).toHaveAttribute('data-brain-strip', 'expanded');
  });

  test('terminal drawer toggles open and closed', async ({ page }) => {
    await page.goto('/');

    // Terminal drawer starts closed (no .drawer-bottom--open)
    const drawer = page.locator('.drawer-bottom');
    // Drawer element should exist in DOM
    await expect(drawer).toBeAttached();

    // Ctrl+` toggles the terminal drawer
    await page.keyboard.press('Control+Backquote');
    await expect(drawer).toHaveClass(/drawer-bottom--open/);

    // Toggle again to close
    await page.keyboard.press('Control+Backquote');
    await expect(drawer).not.toHaveClass(/drawer-bottom--open/);
  });

  test('chat drawer opens via toggle button', async ({ page }) => {
    await page.goto('/');

    // Chat toggle button should be in the DOM
    const chatToggle = page.locator('[aria-label="Toggle chat drawer"]');
    await expect(chatToggle).toBeAttached();

    // Click to open chat drawer
    await chatToggle.click();
    const chatDrawer = page.locator('[aria-label="Chat drawer"]');
    await expect(chatDrawer).toBeVisible();
  });

  test('evolution section renders when navigated to', async ({ page }) => {
    await page.goto('/');

    // Navigate to evolution view via sidebar
    const evolutionLink = page.locator('a[data-view="evolution"]');
    await evolutionLink.click();

    // Wait for evolution view to load in the main container
    const viewContainer = page.locator('#view-container');
    await expect(viewContainer).not.toBeEmpty();
  });

  test('sidebar navigation contains all expected sections', async ({ page }) => {
    await page.goto('/');

    const sidebar = page.locator('nav[aria-label="Main navigation"]');
    await expect(sidebar).toBeVisible();

    // All nav links present
    for (const view of ['overview', 'plans', 'mesh', 'brain', 'agents', 'evolution', 'admin']) {
      await expect(sidebar.locator(`a[data-view="${view}"]`)).toBeAttached();
    }
  });
});
