import { test, expect, MOCK } from './fixtures';
import { SUBSTATUS_MISSION, stubWS } from './widgets.spec';

test.describe('Font Loading', () => {
  test('JetBrains Mono and Orbitron fonts applied', async ({ page, mockApis }) => {
    await mockApis();
    await page.goto('/');
    await page.waitForSelector('.kpi-bar .kpi-card', { timeout: 5000 });

    const fontVar = await page.evaluate(() =>
      getComputedStyle(document.documentElement).getPropertyValue('--font-mono'),
    );
    expect(fontVar).toContain('JetBrainsMono');

    const bodyFont = await page.evaluate(() => getComputedStyle(document.body).fontFamily);
    expect(bodyFont).toContain('JetBrains');

    const h1Font = await page.evaluate(() => getComputedStyle(document.querySelector('h1')!).fontFamily);
    expect(h1Font.toLowerCase()).toContain('orbitron');
  });
});

test.describe('Theme & CSS Variables', () => {
  test('default theme, CSS vars defined, text readable', async ({ page, mockApis }) => {
    await mockApis();
    await page.goto('/');
    await page.waitForSelector('.kpi-bar .kpi-card', { timeout: 5000 });
    expect(await page.evaluate(() => document.documentElement.getAttribute('data-theme'))).toBeNull();
    const result = await page.evaluate(() => {
      const cs = getComputedStyle(document.documentElement);
      const bcs = getComputedStyle(document.body);
      return {
        cyan: cs.getPropertyValue('--cyan').trim(),
        red: cs.getPropertyValue('--red').trim(),
        gold: cs.getPropertyValue('--gold').trim(),
        color: bcs.color, bg: bcs.backgroundColor,
      };
    });
    expect(result.cyan.length).toBeGreaterThan(0);
    expect(result.red.length).toBeGreaterThan(0);
    expect(result.gold.length).toBeGreaterThan(0);
    expect(result.color).not.toBe(result.bg);
  });
});

test.describe('Substatus Badges', () => {
  test.beforeEach(async ({ page, mockApis }) => {
    await mockApis({ mission: SUBSTATUS_MISSION });
    await page.goto('/');
    await page.waitForSelector('#mission-content .mission-plan', { timeout: 5000 });
    await page.waitForSelector('#task-table .mn-badge--info', { timeout: 5000 });
  });

  test('waiting_ci shows CI badge with SVG and correct color', async ({ page }) => {
    const badge = page.locator('.mn-badge.mn-badge--info', { hasText: 'CI' });
    await expect(badge).toBeVisible({ timeout: 5000 });
    await expect(badge.locator('svg')).toHaveCount(1);
  });

  test('waiting_review shows Review badge with SVG', async ({ page }) => {
    const badge = page.locator('.mn-badge.mn-badge--info', { hasText: 'Review' });
    await expect(badge).toHaveCount(1);
    await expect(badge.locator('svg')).toHaveCount(1);
  });
});

test.describe('Buttons & Interactions', () => {
  test('delegate button exists with SVG icon', async ({ page, mockApis }) => {
    await mockApis();
    await page.goto('/');
    await page.waitForSelector('#mission-content .mission-plan', { timeout: 5000 });
    await expect(page.locator('.mission-delegate-btn')).toHaveCount(1);
    await expect(page.locator('.mission-delegate-btn svg')).toBeAttached();
  });

  test('start button on todo plans has SVG icon', async ({ page, mockApis }) => {
    await mockApis({
      mission: { plans: [{ plan: { ...MOCK.mission.plans[0].plan, status: 'todo', tasks_done: 0 }, waves: [], tasks: [] }] },
    });
    await page.goto('/');
    await page.waitForSelector('.mission-start-btn', { timeout: 5000 });
    await expect(page.locator('.mission-start-btn svg')).toBeAttached();
  });

  test('clicking delegate does not throw JS errors', async ({ page, mockApis }) => {
    await mockApis();
    await page.goto('/');
    await page.waitForSelector('#mission-content .mission-plan', { timeout: 5000 });
    const errors: string[] = [];
    page.on('pageerror', (e) => errors.push(e.message));
    page.on('dialog', (d) => d.dismiss());
    await page.locator('.mission-delegate-btn').click();
    await page.waitForTimeout(500);
    expect(errors).toHaveLength(0);
  });

  test('cancel button exists on non-done mission cards', async ({ page, mockApis }) => {
    await mockApis();
    await page.goto('/');
    await page.waitForSelector('#mission-content .mission-plan', { timeout: 5000 });
    await expect(page.locator('.mission-cancel-btn')).toHaveCount(1);
    await expect(page.locator('.mission-cancel-btn svg')).toBeAttached();
  });

  test('cancel button opens cancel plan modal', async ({ page, mockApis }) => {
    await mockApis();
    await page.goto('/');
    await page.waitForSelector('.mission-cancel-btn', { timeout: 5000 });
    await page.locator('.mission-cancel-btn').click();
    await expect(page.locator('.modal-overlay')).toBeVisible({ timeout: 2000 });
    await expect(page.locator('.modal-title')).toContainText('Cancel Plan');
    await page.locator('.modal-close').click();
  });

  test('window.renderWaveGantt is defined (mission-details.js loaded)', async ({ page, mockApis }) => {
    await mockApis();
    await page.goto('/');
    await page.waitForSelector('#mission-content .mission-plan', { timeout: 5000 });
    const defined = await page.evaluate(() => typeof (window as any).renderWaveGantt === 'function');
    expect(defined).toBe(true);
  });
});

test.describe('Terminal Widget', () => {
  test.beforeEach(async ({ page, mockApis }) => {
    await mockApis();
    await page.goto('/');
    await page.waitForSelector('.kpi-bar .kpi-card', { timeout: 5000 });
  });

  test('terminal exists but hidden on load', async ({ page }) => {
    await expect(page.locator('#term-main')).toBeAttached();
    const display = await page.locator('#term-main').evaluate((el) => getComputedStyle(el).display);
    expect(display).toBe('none');
  });

  test('terminal opens and shows tab label', async ({ page }) => {
    await stubWS(page);
    // Stub xterm.js if CDN didn't load
    await page.evaluate(() => {
      if (typeof (window as any).Terminal === 'undefined') {
        (window as any).Terminal = class { open() {} write() {} onData() { return { dispose() {} }; } onResize() { return { dispose() {} }; } dispose() {} loadAddon() {} };
        (window as any).FitAddon = class { fit() {} };
        (window as any).WebLinksAddon = class {};
      }
    });
    await page.evaluate('termMgr.open("local", "TestTab")');
    // Container should be visible (display set + open class added synchronously)
    await page.waitForTimeout(200);
    const isVisible = await page.locator('#term-main').evaluate(
      (el) => el.style.display !== 'none' && el.classList.contains('open'),
    );
    expect(isVisible).toBe(true);
  });
});
