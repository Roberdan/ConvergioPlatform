import { test, expect } from './fixtures';

const MOCK_IDEAS = [
  { id: 1, title: 'Add dark mode', description: 'Support system-level dark mode', status: 'active', priority: 'P0', project: 'VirtualBPM', project_id: 'proj-1', tags: 'ui,theme', created_at: '2026-03-01T10:00:00Z', updated_at: '2026-03-01T10:00:00Z' },
  { id: 2, title: 'API rate limiting', description: 'Prevent abuse on public endpoints', status: 'draft', priority: 'P1', project: '', project_id: '', tags: 'security', created_at: '2026-03-02T10:00:00Z', updated_at: '2026-03-02T10:00:00Z' },
  { id: 3, title: 'Mobile responsive', description: 'Make dashboard work on tablets', status: 'active', priority: 'P2', project: 'MyConvergio', project_id: 'proj-2', tags: 'ui', created_at: '2026-03-03T10:00:00Z', updated_at: '2026-03-03T10:00:00Z' },
];

const MOCK_PROJECTS = [
  { id: 'proj-1', name: 'VirtualBPM' },
  { id: 'proj-2', name: 'MyConvergio' },
];

function filterMockIdeas(url: URL) {
  const search = url.searchParams.get('search')?.toLowerCase() || '';
  const status = url.searchParams.get('status') || '';
  const priority = url.searchParams.get('priority') || '';

  return MOCK_IDEAS.filter((idea) => {
    const matchesSearch = !search || [idea.title, idea.description, idea.tags].some((value) => value.toLowerCase().includes(search));
    const matchesStatus = !status || idea.status === status;
    const matchesPriority = !priority || idea.priority === priority;
    return matchesSearch && matchesStatus && matchesPriority;
  });
}

/** Setup: mock APIs and navigate to Idea Jar */
async function goToIdeaJar(page: import('@playwright/test').Page) {
  await page.route('**/api/projects', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ projects: MOCK_PROJECTS }),
    }),
  );
  await page.route('**/api/ideas**', (route) => {
    const url = new URL(route.request().url());
    const ideaId = url.pathname.match(/\/api\/ideas\/(\d+)$/)?.[1];

    if (url.pathname.match(/\/api\/ideas\/\d+\/notes$/)) {
      return route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
    }

    if (route.request().method() === 'GET') {
      if (ideaId) {
        const idea = MOCK_IDEAS.find(({ id }) => id === Number(ideaId));
        return route.fulfill({
          status: idea ? 200 : 404,
          contentType: 'application/json',
          body: JSON.stringify(idea ?? { error: 'Idea not found' }),
        });
      }

      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(filterMockIdeas(url)),
      });
    }

    return route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ ok: true, id: ideaId ? Number(ideaId) : 99 }),
    });
  });
  await page.locator('.mn-convergio-pill', { hasText: 'Idea Jar' }).click();
  await expect(page.locator('#idea-list .idea-item')).toHaveCount(3);
}

test.describe('Idea Jar', () => {
  test.beforeEach(async ({ page, mockApis }) => {
    await mockApis();
    await page.goto('/');
    await page.waitForSelector('.kpi-bar .kpi-card', { timeout: 5000 });
  });

  test('idea jar section renders with idea cards', async ({ page }) => {
    await goToIdeaJar(page);
    await expect(page.locator('#dashboard-ideajar-section')).toBeVisible();
    const cards = page.locator('#dashboard-ideajar-section .mission-plan');
    await expect(cards).toHaveCount(3);
  });

  test('idea cards display title and priority badge', async ({ page }) => {
    await goToIdeaJar(page);
    const cards = page.locator('#dashboard-ideajar-section .mission-plan');
    await expect(cards.first()).toContainText('Add dark mode');
    await expect(cards.first()).toContainText('P0');
  });

  test('filter buttons are present', async ({ page }) => {
    await goToIdeaJar(page);
    const filters = page.locator('#ideajar-header-actions button, #ideajar-header-actions select');
    const count = await filters.count();
    expect(count).toBeGreaterThanOrEqual(3);
  });

  test('idea cards show tags', async ({ page }) => {
    await goToIdeaJar(page);
    const firstCard = page.locator('#dashboard-ideajar-section .mission-plan').first();
    await expect(firstCard).toContainText('ui');
  });

  test('clicking idea card does not crash', async ({ page }) => {
    await goToIdeaJar(page);
    const firstCard = page.locator('#dashboard-ideajar-section .mission-plan').first();
    await firstCard.click();
    await page.waitForTimeout(300);
    await expect(page.locator('#dashboard-ideajar-section')).toBeVisible();
  });

  test('status and priority filters narrow the idea list', async ({ page }) => {
    await goToIdeaJar(page);
    const cards = page.locator('#idea-list .idea-item');

    await page.selectOption('#idea-filter-status', 'active');
    await expect(cards).toHaveCount(2);

    await page.selectOption('#idea-filter-priority', 'P2');
    await expect(cards).toHaveCount(1);
    await expect(cards.first()).toContainText('Mobile responsive');
  });

  test('search filters ideas by title and tags', async ({ page }) => {
    await goToIdeaJar(page);
    const cards = page.locator('#idea-list .idea-item');

    await page.locator('#idea-search').fill('security');
    await expect(cards).toHaveCount(1);
    await expect(cards.first()).toContainText('API rate limiting');
  });

  test('idea detail panel opens on click', async ({ page }) => {
    await goToIdeaJar(page);

    await page.locator('#idea-list .idea-item').first().click();

    const modal = page.locator('#idea-modal-overlay');
    await expect(modal).toBeVisible();
    await expect(modal.locator('input[name="title"]')).toHaveValue('Add dark mode');
    await expect(modal.locator('textarea[name="description"]')).toHaveValue('Support system-level dark mode');
  });
});
