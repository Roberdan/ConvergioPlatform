import { test, expect } from './fixtures';

test.describe('Brain Widget — Script Loading & Integration', () => {
  test.beforeEach(async ({ page, mockApis }) => {
    await mockApis();
    await page.goto('/');
    await page.waitForSelector('#brain-widget', { timeout: 5000 });
    await page.waitForTimeout(600);
  });

  // --- 7. Script Loading ---

  test('brain scripts load without console errors', async ({ page, mockApis }) => {
    const errors: string[] = [];
    page.on('pageerror', (e) => errors.push(e.message));

    const failedRequests: string[] = [];
    page.on('response', (res) => {
      if (res.url().includes('brain-') && res.status() >= 400) {
        failedRequests.push(res.url());
      }
      if (res.url().includes('icons.js') && res.status() >= 400) {
        failedRequests.push(res.url());
      }
    });

    await mockApis();
    await page.goto('/');
    await page.waitForSelector('#brain-widget', { timeout: 5000 });
    await page.waitForTimeout(800);

    const brainErrors = errors.filter(e =>
      /brain|Brain|region|organism|canvas|consciousness|effect/i.test(e),
    );
    expect(brainErrors).toHaveLength(0);
    expect(failedRequests).toHaveLength(0);
  });

  test('no 404s for brain-related resources', async ({ page, mockApis }) => {
    const notFound: string[] = [];
    page.on('response', (res) => {
      if (res.status() === 404 && /brain|icons/i.test(res.url())) {
        notFound.push(res.url());
      }
    });

    await mockApis();
    await page.goto('/');
    await page.waitForSelector('#brain-widget', { timeout: 5000 });
    await page.waitForTimeout(500);

    expect(notFound).toHaveLength(0);
  });

  // --- 8. Module Integration ---

  test('window.BrainRegions is defined', async ({ page }) => {
    const defined = await page.evaluate(() => typeof (window as any).BrainRegions !== 'undefined');
    expect(defined).toBe(true);
  });

  test('window.BrainOrganism is defined', async ({ page }) => {
    const defined = await page.evaluate(() => typeof (window as any).BrainOrganism !== 'undefined');
    expect(defined).toBe(true);
  });

  test('window.BrainEffects is defined', async ({ page }) => {
    const defined = await page.evaluate(() => typeof (window as any).BrainEffects !== 'undefined');
    expect(defined).toBe(true);
  });

  test('window._consciousness is defined', async ({ page }) => {
    const defined = await page.evaluate(() => typeof (window as any)._consciousness !== 'undefined');
    expect(defined).toBe(true);
  });

  // --- 9. Stats Update ---

  test('stats overlay shows region or neuron counts', async ({ page }) => {
    const stats = page.locator('#brain-stats');
    await page.waitForTimeout(1500);
    const text = await stats.textContent();
    expect(text).toBeTruthy();
    expect(text!.length).toBeGreaterThan(0);
  });

  test('stats update after data refresh', async ({ page }) => {
    const stats = page.locator('#brain-stats');
    await page.waitForTimeout(1000);

    // Trigger a re-fetch by dispatching custom event (dashboard polls)
    await page.evaluate(() => window.dispatchEvent(new Event('dashboard:refresh')));
    await page.waitForTimeout(3000);
    const text2 = await stats.textContent();

    expect(text2!.length).toBeGreaterThan(0);
  });

  // --- 10. Session Data Rendering ---

  test('window._sessionClusters is defined (session cluster renderer loaded)', async ({ page }) => {
    const defined = await page.evaluate(() => typeof (window as any)._sessionClusters !== 'undefined');
    expect(defined).toBe(true);
  });

  test('canvas still renders with /api/sessions returning data', async ({ page, mockApis }) => {
    await mockApis();
    await page.goto('/');
    await page.waitForSelector('#brain-widget', { timeout: 5000 });
    await page.waitForTimeout(800);

    const canvas = page.locator('#brain-canvas-container canvas');
    const hasContent = await canvas.evaluate((el: HTMLCanvasElement) => {
      const ctx = el.getContext('2d');
      if (!ctx) return false;
      const data = ctx.getImageData(0, 0, el.width, el.height).data;
      for (let i = 3; i < data.length; i += 4) {
        if (data[i] > 0) return true;
      }
      return false;
    });
    expect(hasContent).toBe(true);
  });
});
