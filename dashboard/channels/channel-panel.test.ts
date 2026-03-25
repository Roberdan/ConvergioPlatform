// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// Import after setting up vitest-environment jsdom
const MODULE_PATH = './channel-panel.js';

let renderChannelCard: (ch: unknown) => HTMLElement;
let renderEscalationTable: (data: unknown[]) => HTMLElement;
let fetchChannels: () => Promise<unknown[]>;
let createChannelPanel: (container: HTMLElement) => void;

beforeEach(async () => {
  const mod = await import(MODULE_PATH);
  renderChannelCard = mod.renderChannelCard;
  renderEscalationTable = mod.renderEscalationTable;
  fetchChannels = mod.fetchChannels;
  createChannelPanel = mod.createChannelPanel;
});

afterEach(() => {
  vi.restoreAllMocks();
  document.body.innerHTML = '';
});

describe('renderChannelCard', () => {
  it('produces an element with the channel name', () => {
    const card = renderChannelCard({
      id: 'slack',
      name: 'Slack',
      connected: true,
      messageCount: 42,
      responseTimeMs: 120,
    });
    expect(card.textContent).toContain('Slack');
  });

  it('shows green health indicator when connected=true', () => {
    const card = renderChannelCard({
      id: 'email',
      name: 'Email',
      connected: true,
      messageCount: 10,
      responseTimeMs: 80,
    });
    const indicator = card.querySelector('[data-health]');
    expect(indicator).not.toBeNull();
    expect(indicator!.getAttribute('data-health')).toBe('connected');
  });

  it('shows red health indicator when connected=false', () => {
    const card = renderChannelCard({
      id: 'webhook',
      name: 'Webhook',
      connected: false,
      messageCount: 0,
      responseTimeMs: 0,
    });
    const indicator = card.querySelector('[data-health]');
    expect(indicator).not.toBeNull();
    expect(indicator!.getAttribute('data-health')).toBe('disconnected');
  });

  it('displays message count and response time', () => {
    const card = renderChannelCard({
      id: 'sms',
      name: 'SMS',
      connected: true,
      messageCount: 99,
      responseTimeMs: 55,
    });
    expect(card.textContent).toContain('99');
    expect(card.textContent).toContain('55');
  });
});

describe('renderEscalationTable', () => {
  it('produces table rows for each escalation entry', () => {
    const data = [
      { agent: 'marco', escalations: 3 },
      { agent: 'otto', escalations: 1 },
    ];
    const table = renderEscalationTable(data);
    const rows = table.querySelectorAll('tr');
    // header + 2 data rows
    expect(rows.length).toBeGreaterThanOrEqual(2);
    expect(table.textContent).toContain('marco');
    expect(table.textContent).toContain('otto');
  });

  it('renders empty table without error when data is empty', () => {
    const table = renderEscalationTable([]);
    expect(table).toBeInstanceOf(HTMLElement);
  });
});

describe('fetchChannels', () => {
  it('calls GET /api/channels and returns parsed JSON', async () => {
    const mockChannels = [{ id: 'slack', name: 'Slack', connected: true }];
    const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce({
      ok: true,
      json: async () => mockChannels,
    } as Response);

    const result = await fetchChannels();

    expect(fetchSpy).toHaveBeenCalledWith('/api/channels');
    expect(result).toEqual(mockChannels);
  });
});

describe('createChannelPanel', () => {
  it('renders panel into the provided container without errors', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue({
      ok: true,
      json: async () => [],
    } as Response);

    const container = document.createElement('div');
    document.body.appendChild(container);

    // Should not throw
    expect(() => createChannelPanel(container)).not.toThrow();
    expect(container.querySelector('[data-panel="channels"]')).not.toBeNull();
  });
});
