import { test, expect } from './fixtures';

const MESH_NODE = '.mn-mesh' + '-node';
const MESH_NAME = '.mn-mesh' + '-node__name';
const MESH_STATS = '.mn-mesh-node__stats';
const MESH_ACTION = '.mn-mesh-action';
const MESH_HEADER_ACTION = '.mn-mesh-network__action';

test.describe('Mesh Network', () => {
  test.beforeEach(async ({ page, mockApis }) => {
    await mockApis();
    await page.goto('/');
    await page.waitForSelector(`${MESH_NODE}`, { timeout: 5000 });
  });

  test('renders all mesh nodes', async ({ page }) => {
    const nodes = page.locator(`${MESH_NODE}`);
    await expect(nodes).toHaveCount(3);
  });

  test('online nodes have .online class', async ({ page }) => {
    await expect(page.locator(`${MESH_NODE}.online`)).toHaveCount(2);
    await expect(page.locator(`${MESH_NODE}.offline`)).toHaveCount(1);
  });

  test('coordinator node has .coordinator class', async ({ page }) => {
    await expect(page.locator(`${MESH_NODE}.coordinator`)).toHaveCount(1);
    const coord = page.locator(`${MESH_NODE}.coordinator`);
    await expect(coord).toContainText('mac-worker-2');
    await expect(coord).toContainText('COORDINATOR');
  });

  test('node names are displayed', async ({ page }) => {
    const names = await page.locator(MESH_NAME).allTextContents();
    expect(names).toEqual(expect.arrayContaining(['linux-worker', 'mac-worker-1', 'mac-worker-2']));
  });

  test('online nodes show CPU stats', async ({ page }) => {
    const onlineNode = page.locator(`${MESH_NODE}.online`, { hasText: 'linux-worker' });
    await expect(onlineNode.locator(MESH_STATS)).toContainText('CPU 72%');
  });

  test('offline node shows "No heartbeat"', async ({ page }) => {
    const offNode = page.locator(`${MESH_NODE}.offline`);
    await expect(offNode.locator(MESH_STATS)).toContainText('No heartbeat');
  });

  test('online nodes show action buttons', async ({ page }) => {
    const onlineNode = page.locator(`${MESH_NODE}.online`, { hasText: 'linux-worker' });
    const actions = onlineNode.locator(MESH_ACTION);
    // terminal, sync, heartbeat, auth, status, movehere, reboot = 7
    await expect(actions).toHaveCount(7);
  });

  test('offline nodes show wake button only', async ({ page }) => {
    const offNode = page.locator(`${MESH_NODE}.offline`);
    const actions = offNode.locator(MESH_ACTION);
    await expect(actions).toHaveCount(1);
    await expect(actions.first()).toHaveAttribute('data-action', 'wake');
  });

  test('capabilities are shown as badges', async ({ page }) => {
    const coord = page.locator(`${MESH_NODE}.coordinator`);
    await expect(coord.locator('.mn-mesh-badge')).toHaveCount(3);
    await expect(coord.locator('.mn-mesh-badge', { hasText: 'ollama' })).toHaveClass(/mn-mesh-badge--ollama/);
  });

  test('mesh header shows online count and action buttons', async ({ page }) => {
    await expect(page.locator('#mesh-online-count')).toContainText('2/3 online');
    await expect(page.locator(`${MESH_HEADER_ACTION}[title="Full Sync"]`)).toBeVisible();
    await expect(page.locator(`${MESH_HEADER_ACTION}[title="Push"]`)).toBeVisible();
  });

  test('coordinator node shows active plan', async ({ page }) => {
    const coord = page.locator(`${MESH_NODE}.coordinator`);
    await expect(coord.locator('.mn-plan')).toHaveCount(1);
    await expect(coord.locator('.mn-plan')).toContainText('#300');
    await expect(coord.locator('.mn-plan')).toContainText('Auth refactor');
  });

  test('hub layout with spokes container exists', async ({ page }) => {
    await expect(page.locator('.mn-mesh-network')).toBeVisible();
    await expect(page.locator('.mn-mesh-network__coord')).toBeVisible();
    await expect(page.locator('#mesh-flow-cvs')).toBeAttached();
  });

  test('sync badges are applied after load', async ({ page }) => {
    await page.waitForSelector(`${MESH_NODE}.coordinator .mn-sync-dot`, { state: 'attached', timeout: 5000 });
    // mac-worker-2 should have green sync dot
    const coord = page.locator(`${MESH_NODE}.coordinator`);
    expect(await coord.locator('.mn-sync-green').count()).toBeGreaterThanOrEqual(1);
    // linux-worker should have yellow (out of sync)
    const linux-worker = page.locator(`${MESH_NODE}`, { hasText: 'linux-worker' });
    expect(await linux-worker.locator('.mn-sync-yellow').count()).toBeGreaterThanOrEqual(1);
  });

  test('terminal action button triggers terminal open', async ({ page }) => {
    // Stub WebSocket
    await page.evaluate(() => {
      (window as any).WebSocket = class FakeWS {
        readyState = 3; binaryType = 'arraybuffer';
        onopen: any; onclose: any; onerror: any; onmessage: any;
        send() {} close() {}
        constructor() { setTimeout(() => { this.onerror?.(); this.onclose?.(); }, 50); }
      };
    });

    const termBtn = page.locator(`${MESH_NODE}.online`, { hasText: 'linux-worker' })
      .locator(`${MESH_ACTION}[data-action="terminal"]`);
    await termBtn.click();

    await page.waitForSelector('#term-main.open', { timeout: 3000 });
    await expect(page.locator('.term-tab')).toContainText('linux-worker');
  });
});
