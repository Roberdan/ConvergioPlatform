import { test as base, Page, Route } from '@playwright/test';
import { MOCK } from './fixtures-mock';

export { MOCK } from './fixtures-mock';

type MockOverrides = Partial<typeof MOCK>;

/** Install API route mocks on a Playwright page. */
export async function mockAllApis(page: Page, overrides: MockOverrides = {}) {
  const data = { ...MOCK, ...overrides };
  const routes: Record<string, unknown> = {
    '/api/overview': data.overview,
    '/api/mission': data.mission,
    '/api/tokens/daily': data.tokensDaily,
    '/api/tokens/models': data.tokensModels,
    '/api/mesh': data.mesh,
    '/api/mesh/sync-status': data.meshSyncStatus,
    '/api/history': data.history,
    '/api/tasks/distribution': data.taskDist,
    '/api/events': data.events,
    '/api/notifications': data.notifications,
    '/api/sessions': data.sessions,
    '/api/plan/300': data.planDetail,
    '/api/github/stats/300': data.githubStats,
    '/api/github/events/proj-1': data.githubEvents,
    '/api/nightly/jobs': data.nightlyJobs,
    '/api/agents': data.agents,
    '/api/peers/heartbeats': data.peerHeartbeats,
    '/api/ipc/agents': data.ipcAgents,
    '/api/ipc/messages': data.ipcMessages,
    '/api/ipc/locks': data.ipcLocks,
    '/api/ipc/worktrees': data.ipcWorktrees,
    '/api/ipc/channels': data.ipcChannels,
    '/api/ipc/status': data.ipcStatus,
    '/api/ipc/conflicts': data.ipcConflicts,
    '/api/ipc/context': data.ipcContext,
    '/api/mesh/provision': { ok: true, peers: [
      { peer: 'worker-mac-2', ip: '100.64.0.3', user: 'testuser', online: true, ssh_ok: true, tmux_ok: true, session_ok: true, error: null },
      { peer: 'worker-linux-1', ip: '100.64.0.2', user: 'testuser', online: true, ssh_ok: true, tmux_ok: true, session_ok: true, error: null },
      { peer: 'worker-mac-1', ip: '100.64.0.1', user: 'testuser', online: true, ssh_ok: true, tmux_ok: true, session_ok: true, error: null },
    ]},
  };
  for (const [path, body] of Object.entries(routes)) {
    await page.route(`**${path}`, (route: Route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(body) }),
    );
  }
  await page.route('**/api/mesh/pull-db', (route: Route) =>
    route.fulfill({
      status: 200,
      contentType: 'text/event-stream',
      headers: { 'cache-control': 'no-cache', connection: 'keep-alive' },
      body: `event: done\ndata: ${JSON.stringify(data.pullDb)}\n\n`,
    }),
  );
  // Catch any /api/plan/<id> request and return planDetail mock
  await page.route('**/api/plan/*', (route: Route) =>
    route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(data.planDetail) }),
  );
  // Block WS dashboard connection to avoid noise
  await page.route('**/ws/dashboard', (route: Route) => route.abort());
}

/** Extended test fixture with mockApis helper. */
export const test = base.extend<{ mockApis: (overrides?: MockOverrides) => Promise<void> }>({
  mockApis: async ({ page }, use) => {
    await use(async (overrides?: MockOverrides) => {
      await mockAllApis(page, overrides);
    });
  },
});

export { expect } from '@playwright/test';
