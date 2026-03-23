import { test } from '@playwright/test';

test('Maranello visual screenshot', async ({ page }) => {
  await page.goto('http://localhost:8420', { waitUntil: 'networkidle' });
  await page.waitForTimeout(500);
  await page.evaluate(() => (window as any).setTheme?.('maranello'));
  await page.waitForTimeout(3000);

  // Check for JS errors
  const errors: string[] = [];
  page.on('pageerror', err => errors.push(err.message));

  await page.screenshot({ path: '/tmp/maranello-full.png', fullPage: true });
  await page.screenshot({ path: '/tmp/maranello-viewport.png' });

  const kpi = page.locator('#kpi-bar');
  if (await kpi.count() > 0) {
    await kpi.screenshot({ path: '/tmp/maranello-kpi.png' });
  }

  // Scroll to charts area
  await page.evaluate(() => {
    const el = document.querySelector('.chart-container');
    if (el) el.scrollIntoView();
  });
  await page.waitForTimeout(500);
  await page.screenshot({ path: '/tmp/maranello-charts.png' });

  if (errors.length) console.log('JS ERRORS:', errors);
});
