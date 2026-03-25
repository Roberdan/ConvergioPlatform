/**
 * Channel monitoring panel for the Convergio dashboard.
 * Renders channel health, message volume, and escalation data
 * by querying the daemon REST API.
 */

/**
 * Creates a card element for a single channel.
 * @param {object} channel
 * @param {string} channel.id
 * @param {string} channel.name
 * @param {boolean} channel.connected
 * @param {number} channel.messageCount
 * @param {number} channel.responseTimeMs
 * @returns {HTMLElement}
 */
export function renderChannelCard(channel) {
  const card = document.createElement('article');
  card.className = 'mn-card channel-card';
  card.setAttribute('aria-label', `Channel: ${channel.name}`);

  const health = document.createElement('span');
  const status = channel.connected ? 'connected' : 'disconnected';
  health.setAttribute('data-health', status);
  health.className = `channel-health channel-health--${status}`;
  health.setAttribute('aria-label', status);
  health.setAttribute('role', 'img');

  const name = document.createElement('h3');
  name.className = 'mn-title channel-name';
  name.textContent = channel.name;

  const meta = document.createElement('dl');
  meta.className = 'channel-meta mn-body';

  const msgTerm = document.createElement('dt');
  msgTerm.textContent = 'Messages';
  const msgVal = document.createElement('dd');
  msgVal.textContent = String(channel.messageCount);

  const rtTerm = document.createElement('dt');
  rtTerm.textContent = 'Response (ms)';
  const rtVal = document.createElement('dd');
  rtVal.textContent = String(channel.responseTimeMs);

  meta.append(msgTerm, msgVal, rtTerm, rtVal);
  card.append(health, name, meta);
  return card;
}

/**
 * Renders a table of agent escalation counts.
 * @param {Array<{agent: string, escalations: number}>} data
 * @returns {HTMLTableElement}
 */
export function renderEscalationTable(data) {
  const table = document.createElement('table');
  table.className = 'channel-escalations mn-body';
  table.setAttribute('aria-label', 'Agent escalation counts');

  const thead = document.createElement('thead');
  const headerRow = document.createElement('tr');
  ['Agent', 'Escalations'].forEach((col) => {
    const th = document.createElement('th');
    th.scope = 'col';
    th.textContent = col;
    headerRow.appendChild(th);
  });
  thead.appendChild(headerRow);
  table.appendChild(thead);

  const tbody = document.createElement('tbody');
  for (const entry of data) {
    const row = document.createElement('tr');
    const agentCell = document.createElement('td');
    agentCell.textContent = entry.agent;
    const countCell = document.createElement('td');
    countCell.textContent = String(entry.escalations);
    row.append(agentCell, countCell);
    tbody.appendChild(row);
  }
  table.appendChild(tbody);
  return table;
}

/**
 * Fetches all channels from the daemon API.
 * @returns {Promise<object[]>}
 */
export async function fetchChannels() {
  const response = await fetch('/api/channels');
  if (!response.ok) {
    console.warn(`[channel-panel] GET /api/channels returned ${response.status}`);
    return [];
  }
  return response.json();
}

/**
 * Renders the full channel monitoring panel into a container element.
 * @param {HTMLElement} container
 */
export function createChannelPanel(container) {
  const panel = document.createElement('section');
  panel.setAttribute('data-panel', 'channels');
  panel.className = 'mn-section-dark channel-panel';
  panel.setAttribute('aria-label', 'Channel monitoring');

  const heading = document.createElement('h2');
  heading.className = 'mn-title';
  heading.textContent = 'Channels';

  const grid = document.createElement('div');
  grid.className = 'channel-grid';
  grid.setAttribute('role', 'list');

  const escalationSection = document.createElement('div');
  escalationSection.className = 'channel-escalation-section';

  const escalationHeading = document.createElement('h3');
  escalationHeading.className = 'mn-body';
  escalationHeading.textContent = 'Escalations';
  escalationSection.appendChild(escalationHeading);

  panel.append(heading, grid, escalationSection);
  container.appendChild(panel);

  // Async data load — failures warned, never silent
  fetchChannels()
    .then((channels) => {
      if (!Array.isArray(channels) || channels.length === 0) {
        console.warn('[channel-panel] No channels returned from API');
        grid.textContent = 'No channels configured.';
        return;
      }
      for (const ch of channels) {
        const card = renderChannelCard(ch);
        grid.appendChild(card);
      }
    })
    .catch((err) => {
      console.warn('[channel-panel] Failed to load channels:', err);
      grid.textContent = 'Channel data unavailable.';
    });
}
