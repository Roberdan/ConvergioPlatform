import { test, expect } from './fixtures';

test.describe('Plan Move', () => {
  test.beforeEach(async ({ page, mockApis }) => {
    await mockApis();
    await page.route('**/api/plan/move*', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ ok: true, plan_id: 300, target: 'linux-worker' }),
      }),
    );
    await page.goto('/');
    await page.waitForSelector('.mn-mesh-node', { timeout: 5000 });
  });

  test('move-here button exists on online worker nodes', async ({ page }) => {
    const linuxWorker = page.locator('.mn-mesh-node.online', { hasText: 'linux-worker' });
    const moveBtn = linuxWorker.locator('.mn-mesh-action[data-action="movehere"]');
    await expect(moveBtn).toHaveCount(1);
  });

  test('move-here button triggers move dialog', async ({ page }) => {
    page.on('dialog', (dialog) => dialog.accept());
    const linuxWorker = page.locator('.mn-mesh-node.online', { hasText: 'linux-worker' });
    const moveBtn = linuxWorker.locator('.mn-mesh-action[data-action="movehere"]');
    await moveBtn.click();
    await page.waitForTimeout(500);
  });
});
