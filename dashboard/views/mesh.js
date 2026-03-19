/**
 * Mesh view: 3-tab layout (Status, Network, Actions) using mn-tabs.
 * Consumes api.fetchMesh() and api.fetchMeshSyncStats().
 */

import { createPeerCard } from '../widgets/peer-card.js';

const STYLE_ID = 'mn-mesh-view-style';

function injectStyles() {
  if (document.getElementById(STYLE_ID)) return;
  const style = document.createElement('style');
  style.id = STYLE_ID;
  style.textContent = `
    .mn-mesh-grid {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
      gap: 1rem;
      padding: 1rem 0;
    }
    .mn-mesh-actions {
      display: flex;
      gap: 0.75rem;
      flex-wrap: wrap;
      padding: 1rem 0;
    }
    .mn-mesh-error {
      color: var(--signal-danger);
      padding: 0.5rem;
    }
    .mn-mesh-loading {
      color: var(--mn-text-muted);
      padding: 1rem;
    }
  `;
  document.head.appendChild(style);
}

function esc(s) {
  const d = document.createElement('div');
  d.textContent = s;
  return d.innerHTML;
}

// -- Tab 1: Status --

function renderStatusTab(tab, api) {
  tab.innerHTML = '<div class="mn-mesh-loading">Loading peers...</div>';

  api.fetchMesh().then(peers => {
    tab.innerHTML = '';
    if (!Array.isArray(peers) || peers.length === 0) {
      tab.innerHTML = '<p style="color:var(--mn-text-muted)">No peers found.</p>';
      return;
    }
    const grid = document.createElement('div');
    grid.className = 'mn-mesh-grid';
    peers.forEach(peer => grid.appendChild(createPeerCard(peer)));
    tab.appendChild(grid);
  }).catch(err => {
    tab.innerHTML = `<div class="mn-mesh-error">Failed to load mesh: ${esc(err.message)}</div>`;
  });
}

// -- Tab 2: Network --

const SYNC_COLUMNS = [
  { key: 'peer_name', label: 'Peer' },
  { key: 'last_sync', label: 'Last Sync' },
  { key: 'sync_count', label: 'Syncs' },
  { key: 'avg_latency_ms', label: 'Avg Latency (ms)' },
  { key: 'errors', label: 'Errors' },
  { key: 'status', label: 'Status' },
];

function buildTable(data) {
  const table = document.createElement('mn-data-table');
  table.setAttribute('columns', JSON.stringify(SYNC_COLUMNS));
  table.setAttribute('rows', JSON.stringify(data));
  return table;
}

function renderNetworkTab(tab, api) {
  tab.innerHTML = '<div class="mn-mesh-loading">Loading sync stats...</div>';

  api.fetchMeshSyncStats().then(stats => {
    tab.innerHTML = '';
    if (!Array.isArray(stats) || stats.length === 0) {
      tab.innerHTML = '<p style="color:var(--mn-text-muted)">No sync data available.</p>';
      return;
    }
    tab.appendChild(buildTable(stats));
  }).catch(err => {
    tab.innerHTML = `<div class="mn-mesh-error">Failed to load sync stats: ${esc(err.message)}</div>`;
  });
}

// -- Tab 3: Actions --

const ACTIONS = [
  { id: 'sync-all', label: 'Sync All Peers', variant: 'primary', action: 'syncAll' },
  { id: 'heartbeat', label: 'Send Heartbeat', variant: 'secondary', action: 'heartbeat' },
  { id: 'refresh', label: 'Refresh Status', variant: 'ghost', action: 'refresh' },
];

function renderActionsTab(tab, api, store, reloadView) {
  tab.innerHTML = '';
  const wrapper = document.createElement('div');
  wrapper.className = 'mn-mesh-actions';

  ACTIONS.forEach(({ id, label, variant, action }) => {
    const btn = document.createElement('button');
    btn.id = `mesh-action-${id}`;
    btn.className = `mn-btn mn-btn--${variant}`;
    btn.textContent = label;
    btn.addEventListener('click', () => handleAction(btn, api, action, reloadView));
    wrapper.appendChild(btn);
  });

  tab.appendChild(wrapper);
}

async function handleAction(btn, api, action, reloadView) {
  btn.disabled = true;
  btn.textContent += ' ...';

  try {
    if (action === 'syncAll' && api.syncAllPeers) {
      await api.syncAllPeers();
    } else if (action === 'heartbeat' && api.sendHeartbeat) {
      await api.sendHeartbeat();
    } else if (action === 'refresh') {
      reloadView();
      return;
    }
  } catch (err) {
    console.warn(`Mesh action "${action}" failed:`, err);
  } finally {
    btn.disabled = false;
    btn.textContent = btn.textContent.replace(' ...', '');
  }
}

// -- Tab 4: Topology --

/**
 * Render topology map tab. Uses mn-map if available, otherwise mn-placeholder.
 * Builds a network graph of peers from mesh data.
 */
function renderTopologyTab(tab, api) {
  const mapAvailable = !!customElements.get('mn-map');

  if (!mapAvailable) {
    console.warn('[mesh] mn-map not registered — showing topology placeholder');
    const placeholder = document.createElement('mn-placeholder');
    placeholder.setAttribute('label', 'Mesh topology map — awaiting MLD delivery');
    tab.appendChild(placeholder);
    return;
  }

  tab.innerHTML = '<div class="mn-mesh-loading">Loading topology...</div>';

  api.fetchMesh().then(peers => {
    tab.innerHTML = '';
    if (!Array.isArray(peers) || peers.length === 0) {
      tab.innerHTML = '<p style="color:var(--mn-text-muted)">No peers for topology.</p>';
      return;
    }
    const map = document.createElement('mn-map');
    const nodes = peers.map(p => ({
      id: p.peer_id || p.name,
      label: p.name || p.peer_id,
      status: p.status || 'unknown',
      role: p.role || 'worker',
    }));
    // WHY: coordinator connects to all workers; workers connect only to coordinator
    const edges = [];
    const coordinator = nodes.find(n => n.role === 'coordinator');
    if (coordinator) {
      for (const node of nodes) {
        if (node.id !== coordinator.id) {
          edges.push({ from: coordinator.id, to: node.id });
        }
      }
    }
    map.setAttribute('nodes', JSON.stringify(nodes));
    map.setAttribute('edges', JSON.stringify(edges));
    tab.appendChild(map);
  }).catch(err => {
    tab.innerHTML = `<div class="mn-mesh-error">Topology load failed: ${esc(err.message)}</div>`;
  });
}

// -- Main view --

/**
 * Mount the mesh view into the given container.
 * @param {HTMLElement} container
 * @param {{api: object, store: object}} deps
 * @returns {Function} cleanup function
 */
export default function mesh(container, { api, store }) {
  injectStyles();
  container.innerHTML = '';

  const tabs = document.createElement('mn-tabs');

  // Tab 1: Status
  const statusTab = document.createElement('mn-tab');
  statusTab.setAttribute('label', 'Status');
  renderStatusTab(statusTab, api);

  // Tab 2: Network
  const networkTab = document.createElement('mn-tab');
  networkTab.setAttribute('label', 'Network');
  renderNetworkTab(networkTab, api);

  // Tab 3: Actions
  const actionsTab = document.createElement('mn-tab');
  actionsTab.setAttribute('label', 'Actions');

  const reloadView = () => {
    renderStatusTab(statusTab, api);
    renderNetworkTab(networkTab, api);
  };

  renderActionsTab(actionsTab, api, store, reloadView);

  // Tab 4: Topology
  const topologyTab = document.createElement('mn-tab');
  topologyTab.setAttribute('label', 'Topology');
  renderTopologyTab(topologyTab, api);

  tabs.append(statusTab, networkTab, actionsTab, topologyTab);
  container.appendChild(tabs);

  // Cleanup
  return () => {
    container.innerHTML = '';
  };
}
