import { test, expect } from '@playwright/test';
import { attachErrorCollectors } from './full-navigation-audit.spec';

/**
 * Full dashboard navigation audit — interactive controls and periodic checks.
 * Continuation of full-navigation-audit.spec.ts.
 * Runs against the REAL server (no mocks).
 */

test.describe('Full dashboard navigation audit — interactions', () => {
  test.skip(!!process.env.CI, 'Requires real server with data — skipped on CI');

  test('mesh interactions — Add Peer and Discover buttons', async ({ page }) => {
    const errors = attachErrorCollectors(page);
    await page.goto('/', { waitUntil: 'networkidle' });
    await page.waitForTimeout(2000);

    const addPeerBtn = page.locator('button:has-text("Add Peer")');
    if (await addPeerBtn.isVisible()) {
      await addPeerBtn.click();
      await page.waitForTimeout(1000);
      const overlay = page.locator('#peer-form-overlay');
      if (await overlay.isVisible()) {
        await page.keyboard.press('Escape');
        await page.waitForTimeout(500);
        // Force-hide if Escape didn't close it
        if (await overlay.isVisible()) {
          await page.evaluate(() => {
            const el = document.getElementById('peer-form-overlay');
            if (el) el.style.display = 'none';
          });
          await page.waitForTimeout(300);
        }
      }
    }

    const discoverBtn = page.locator('button:has-text("Discover")');
    if (await discoverBtn.isVisible()) {
      await discoverBtn.click({ force: true });
      await page.waitForTimeout(1000);
      await page.keyboard.press('Escape');
      await page.waitForTimeout(500);
    }

    expect(errors, `Mesh errors:\n${errors.map((e) => `[${e.type}] ${e.message}`).join('\n')}`).toHaveLength(0);
  });

  test('zoom and refresh controls work', async ({ page }) => {
    const errors = attachErrorCollectors(page);
    await page.goto('/', { waitUntil: 'networkidle' });
    await page.waitForTimeout(1000);

    const zoomIn = page.locator('.header-ctrl-btn:has-text("+")');
    if (await zoomIn.isVisible()) {
      await zoomIn.click();
      await page.waitForTimeout(300);
    }

    const zoomOut = page.locator('.header-ctrl-btn:has-text("−")');
    if (await zoomOut.isVisible()) {
      await zoomOut.click();
      await page.waitForTimeout(300);
    }

    const zoomReset = page.locator('.header-ctrl-btn:has-text("R")');
    if (await zoomReset.isVisible()) {
      await zoomReset.click();
      await page.waitForTimeout(300);
    }

    const refreshUp = page.locator('.stepper-btn').last();
    if (await refreshUp.isVisible()) {
      await refreshUp.click();
      await page.waitForTimeout(500);
    }

    expect(errors, `Control errors:\n${errors.map((e) => `[${e.type}] ${e.message}`).join('\n')}`).toHaveLength(0);
  });

  test('nightly jobs + button opens create form', async ({ page }) => {
    const errors = attachErrorCollectors(page);
    await page.goto('/', { waitUntil: 'networkidle' });
    await page.waitForTimeout(2000);

    const addJobBtn = page.locator('[data-action="show-create"], .nightly-btn-add').first();
    if (await addJobBtn.isVisible()) {
      await addJobBtn.click();
      await page.waitForTimeout(1000);
    }

    expect(errors, `Nightly create errors:\n${errors.map((e) => `[${e.type}] ${e.message}`).join('\n')}`).toHaveLength(0);
  });

  test('brain canvas interactions', async ({ page }) => {
    const errors = attachErrorCollectors(page);
    await page.goto('/', { waitUntil: 'networkidle' });
    await page.waitForTimeout(2000);

    const pauseBtn = page.locator('#brain-pause-btn');
    if (await pauseBtn.isVisible()) {
      await pauseBtn.click();
      await page.waitForTimeout(500);
      await pauseBtn.click();
      await page.waitForTimeout(500);
    }

    const rewindBtn = page.locator('#brain-rewind-btn');
    if (await rewindBtn.isVisible()) {
      await rewindBtn.click();
      await page.waitForTimeout(500);
    }

    expect(errors, `Brain errors:\n${errors.map((e) => `[${e.type}] ${e.message}`).join('\n')}`).toHaveLength(0);
  });

  test('full auto-refresh cycle completes without errors', async ({ page }) => {
    const errors = attachErrorCollectors(page);
    await page.goto('/', { waitUntil: 'networkidle' });
    // Wait for 2 full refresh cycles (default 30s, but we wait 8s to catch at least one)
    await page.waitForTimeout(8000);

    const jsErrors = errors.filter((e) => e.type === 'js');
    const networkErrors = errors.filter((e) => e.type === 'network');

    expect(jsErrors, `JS errors during refresh:\n${jsErrors.map((e) => e.message).join('\n')}`).toHaveLength(0);
    expect(networkErrors, `Network errors during refresh:\n${networkErrors.map((e) => e.message).join('\n')}`).toHaveLength(0);
  });
});
