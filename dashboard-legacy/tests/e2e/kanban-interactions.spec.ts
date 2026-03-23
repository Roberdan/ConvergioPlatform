import { test, expect } from './fixtures';
import { KANBAN_MISSION } from './kanban.spec';

test.describe('Plan Kanban Board — Drag & Drop and Cancel', () => {
  test.beforeEach(async ({ page, mockApis }) => {
    await mockApis({ mission: KANBAN_MISSION });
    await page.route('**/api/plan-status', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ ok: true }) }),
    );
    await page.goto('/');
    await page.waitForSelector('#plan-kanban-widget', { timeout: 5000 });
    await page.waitForTimeout(1000);
  });

  // --- Drag & Drop ---

  test('drag card from Pipeline to Executing shows start dialog (not direct API)', async ({ page }) => {
    const todoCard = page.locator('#kanban-todo .kanban-card').first();
    const doingCol = page.locator('.kanban-col[data-status="doing"]');

    // No API should fire — drag to doing opens start dialog
    let apiCalled = false;
    page.on('request', (req) => {
      if (req.url().includes('/api/plan-status') && req.method() === 'POST') apiCalled = true;
    });

    await todoCard.dragTo(doingCol);
    await page.waitForTimeout(400);

    await expect(page.locator('.modal-overlay')).toBeVisible();
    await expect(page.locator('.modal-title')).toContainText('Start #301');
    expect(apiCalled).toBe(false);

    await page.locator('.modal-close').click();
  });

  test('drag card from Executing to Pipeline triggers confirm and API', async ({ page }) => {
    page.on('dialog', (d) => d.accept());
    const doingCard = page.locator('#kanban-doing .kanban-card').first();
    const todoCol = page.locator('.kanban-col[data-status="todo"]');

    const [request] = await Promise.all([
      page.waitForRequest((req) => req.url().includes('/api/plan-status') && req.method() === 'POST'),
      doingCard.dragTo(todoCol),
    ]);
    const body = JSON.parse(request.postData()!);
    expect(body.status).toBe('todo');
  });

  test('API POST body has correct shape (doing→todo drag)', async ({ page }) => {
    page.on('dialog', (d) => d.accept());
    const doingCard = page.locator('#kanban-doing .kanban-card').first();
    const todoCol = page.locator('.kanban-col[data-status="todo"]');

    const [request] = await Promise.all([
      page.waitForRequest((req) => req.url().includes('/api/plan-status')),
      doingCard.dragTo(todoCol),
    ]);
    const body = JSON.parse(request.postData()!);
    expect(body).toHaveProperty('plan_id');
    expect(body).toHaveProperty('status');
    expect(typeof body.plan_id).toBe('number');
  });

  // --- Empty State ---

  test('empty columns show "No plans" message', async ({ page, mockApis }) => {
    await mockApis({ mission: { plans: [] } });
    await page.goto('/');
    await page.waitForSelector('#plan-kanban-widget', { timeout: 5000 });
    await page.waitForTimeout(1000);

    const empties = page.locator('.kanban-empty');
    await expect(empties).toHaveCount(4, { timeout: 10000 });
    for (let i = 0; i < 4; i++) {
      await expect(empties.nth(i)).toContainText('No plans');
    }
  });

  test('drag-over class applied on dragover', async ({ page }) => {
    const col = page.locator('.kanban-col[data-status="doing"]');
    await col.evaluate((el) => {
      el.dispatchEvent(new DragEvent('dragover', { bubbles: true }));
    });
    await expect(col).toHaveClass(/drag-over/);
  });

  // --- Cancel / Trash Buttons ---

  test('cancel/trash button exists on todo card', async ({ page }) => {
    const todoCard = page.locator('#kanban-todo .kanban-card').first();
    await expect(todoCard.locator('.kanban-trash-btn')).toHaveCount(1);
    await expect(todoCard.locator('.kanban-trash-btn svg')).toBeAttached();
  });

  test('cancel/trash button exists on doing card', async ({ page }) => {
    const doingCard = page.locator('#kanban-doing .kanban-card').first();
    await expect(doingCard.locator('.kanban-trash-btn')).toHaveCount(1);
  });

  test('cancel/trash button does NOT exist on done card', async ({ page }) => {
    const doneCard = page.locator('#kanban-done .kanban-card').first();
    await expect(doneCard.locator('.kanban-trash-btn')).toHaveCount(0);
  });

  test('clicking cancel button opens confirmation modal (not native dialog)', async ({ page }) => {
    const trashBtn = page.locator('#kanban-todo .kanban-trash-btn').first();
    await trashBtn.click();
    await expect(page.locator('.modal-overlay')).toBeVisible({ timeout: 2000 });
    await expect(page.locator('.modal-title')).toContainText('Cancel Plan');
    await page.locator('.modal-close').click();
  });
});
