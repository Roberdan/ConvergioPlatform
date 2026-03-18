/* ipc-panel.js — IPC Coordination Panel orchestrator */
/* global dashlog, renderAgentGrid, renderMessageFeed, renderIpcLocks, renderIpcWorktrees */

(function () {
  'use strict';

  let _ipcData = null;
  let _refreshTimer = null;

  async function fetchIpcData() {
    try {
      const [agents, messages, locks, worktrees, channels, status, conflicts] = await Promise.all([
        fetch('/api/ipc/agents').then(r => r.json()),
        fetch('/api/ipc/messages?limit=100').then(r => r.json()),
        fetch('/api/ipc/locks').then(r => r.json()),
        fetch('/api/ipc/worktrees').then(r => r.json()),
        fetch('/api/ipc/channels').then(r => r.json()),
        fetch('/api/ipc/status').then(r => r.json()),
        fetch('/api/ipc/conflicts').then(r => r.json()),
      ]);
      _ipcData = { agents, messages, locks, worktrees, channels, status, conflicts };
      return _ipcData;
    } catch (err) {
      if (typeof dashlog !== 'undefined') dashlog.error('ipc-panel', 'fetchIpcData failed', err);
      return null;
    }
  }

  function renderIpcPanel() {
    const container = document.getElementById('ipc-panel-root');
    if (!container) return;

    const data = _ipcData;
    if (!data) {
      container.innerHTML = '<div class="ipc-loading">Loading IPC data…</div>';
      return;
    }

    const st = data.status || {};
    container.innerHTML = `
      <div class="ipc-kpi-bar" role="region" aria-label="IPC status summary">
        <div class="ipc-kpi-card">
          <span class="ipc-kpi-value">${st.agents_active || 0}</span>
          <span class="ipc-kpi-label">Active Agents</span>
        </div>
        <div class="ipc-kpi-card">
          <span class="ipc-kpi-value">${st.locks_active || 0}</span>
          <span class="ipc-kpi-label">File Locks</span>
        </div>
        <div class="ipc-kpi-card">
          <span class="ipc-kpi-value">${st.messages_total || 0}</span>
          <span class="ipc-kpi-label">Messages</span>
        </div>
        <div class="ipc-kpi-card ${(st.conflicts || 0) > 0 ? 'ipc-kpi-warning' : ''}">
          <span class="ipc-kpi-value">${st.conflicts || 0}</span>
          <span class="ipc-kpi-label">Conflicts</span>
        </div>
      </div>
      <div class="ipc-grid">
        <div class="ipc-col-left">
          <div id="ipc-agents-container" class="ipc-widget"></div>
          <div id="ipc-locks-container" class="ipc-widget"></div>
        </div>
        <div class="ipc-col-right">
          <div id="ipc-messages-container" class="ipc-widget"></div>
        </div>
      </div>
    `;

    if (typeof renderAgentGrid === 'function') {
      renderAgentGrid(data.agents?.agents || [], data.worktrees?.worktrees || []);
    }
    if (typeof renderMessageFeed === 'function') {
      renderMessageFeed(data.messages?.messages || [], data.channels?.channels || []);
    }
    if (typeof renderIpcLocks === 'function') {
      renderIpcLocks(
        data.locks?.locks || [],
        data.worktrees?.worktrees || [],
        data.conflicts?.conflicts || []
      );
    }
  }

  async function refreshIpc() {
    await fetchIpcData();
    renderIpcPanel();
  }

  function startIpcRefresh(intervalMs) {
    stopIpcRefresh();
    refreshIpc();
    _refreshTimer = setInterval(refreshIpc, intervalMs || 10000);
  }

  function stopIpcRefresh() {
    if (_refreshTimer) {
      clearInterval(_refreshTimer);
      _refreshTimer = null;
    }
  }

  function handleIpcWsEvent(event) {
    if (!_ipcData) return;
    const type = event.type || event.kind;
    if (type === 'ipc_message' || type === 'ipc_agent_register' ||
        type === 'ipc_lock_change' || type === 'ipc_worktree_change') {
      refreshIpc();
    }
  }

  window.startIpcRefresh = startIpcRefresh;
  window.stopIpcRefresh = stopIpcRefresh;
  window.handleIpcWsEvent = handleIpcWsEvent;
  window.refreshIpc = refreshIpc;
})();
