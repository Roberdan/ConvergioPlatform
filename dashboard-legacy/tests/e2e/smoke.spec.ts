import { test, expect } from '@playwright/test';

test.describe('Smoke', () => {
  test('all five dashboard sections exist and app chrome renders', async ({ page }) => {
    await page.goto('/');

    await expect(page.locator('#dashboard-main-section')).toBeVisible();
    await expect(page.locator('#dashboard-admin-section')).toHaveCount(1);
    await expect(page.locator('#dashboard-chat-section')).toHaveCount(1);
    await expect(page.locator('#dashboard-ideajar-section')).toHaveCount(1);
    await expect(page.locator('#dashboard-brain-section')).toHaveCount(1);

    await expect(page.locator('#dashboard-nav')).toBeVisible();
    await expect(page.locator('#kpi-bar')).toBeVisible();
    await expect(page.locator('#mission-panel')).toBeVisible();
    await expect(page.locator('#mesh-panel')).toBeVisible();
    await expect(page.locator('#brain-widget')).toBeVisible();
  });
});
