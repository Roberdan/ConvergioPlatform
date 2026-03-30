import { test, expect, Page } from '@playwright/test';
import path from 'path';

const SCREENSHOT_DIR = path.join('test-results', 'screenshots');

async function capturePageErrors(page: Page): Promise<string[]> {
  const errors: string[] = [];
  page.on('pageerror', (err) => errors.push(err.message));
  return errors;
}

test.describe('Dashboard main page', () => {
  test('loads without JS errors', async ({ page }) => {
    const errors = await capturePageErrors(page);

    await page.goto('/');
    await page.waitForLoadState('networkidle');

    expect(errors).toHaveLength(0);
  });

  test('page title is set', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('domcontentloaded');

    const title = await page.title();
    expect(title.length).toBeGreaterThan(0);
  });

  test('renders a root container element', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('domcontentloaded');

    // The daemon serves a single-page app — verify a top-level container exists.
    const root = page.locator('body > *').first();
    await expect(root).toBeAttached();
  });

  test('navigation or sidebar exists', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Accept any standard nav landmark: <nav>, role=navigation, or common class names.
    const nav = page
      .locator('nav, [role="navigation"], .sidebar, .nav, #nav, #sidebar')
      .first();
    await expect(nav).toBeAttached();
  });

  test('no 4xx/5xx responses on initial load', async ({ page }) => {
    const failedRequests: string[] = [];

    page.on('requestfailed', (req) => {
      failedRequests.push(`${req.failure()?.errorText ?? 'unknown'} — ${req.url()}`);
    });

    page.on('response', (res) => {
      if (res.status() >= 400) {
        failedRequests.push(`HTTP ${res.status()} — ${res.url()}`);
      }
    });

    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Filter noise from browser extension injections or preflight OPTIONS.
    const relevant = failedRequests.filter(
      (r) => !r.includes('chrome-extension') && !r.includes('moz-extension'),
    );
    expect(relevant).toHaveLength(0);
  });

  test('visual snapshot — dashboard home', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    await page.screenshot({
      path: `${SCREENSHOT_DIR}/dashboard-home.png`,
      fullPage: true,
    });

    // Snapshot assertion: first run creates the baseline; subsequent runs compare.
    await expect(page).toHaveScreenshot('dashboard-home.png', {
      maxDiffPixelRatio: 0.05,
    });
  });
});
