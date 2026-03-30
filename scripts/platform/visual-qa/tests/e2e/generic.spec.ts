import { expect, test } from '@playwright/test';

const route = process.env.VQA_ROUTE ?? '/';
const readySelector = process.env.VQA_READY_SELECTOR;
const expectedTitle = process.env.VQA_EXPECTED_TITLE;
const snapshotName = (() => {
  const raw = process.env.VQA_SNAPSHOT_NAME ?? 'visual-qa-page';
  return raw.endsWith('.png') ? raw : `${raw}.png`;
})();

const waitForReady = async (page: Parameters<typeof test>[0]['page']) => {
  await page.goto(route);
  if (readySelector) {
    await expect(page.locator(readySelector)).toBeVisible();
    return;
  }
  await page.waitForLoadState('networkidle');
};

test.describe('generic visual QA', () => {
  test('loads without console errors', async ({ page }) => {
    const consoleErrors: string[] = [];
    page.on('console', (message) => {
      if (message.type() === 'error') {
        consoleErrors.push(message.text());
      }
    });

    await waitForReady(page);

    if (expectedTitle) {
      await expect(page).toHaveTitle(expectedTitle);
    }

    expect(consoleErrors).toEqual([]);
  });

  test('@visual captures a stable page snapshot', async ({ page }) => {
    await waitForReady(page);
    await expect(page).toHaveScreenshot(snapshotName, {
      animations: 'disabled',
      fullPage: true,
    });
  });
});
