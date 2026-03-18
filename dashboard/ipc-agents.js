/* ipc-agents.js — Agent grid component for IPC panel */
/* global dashlog */

(function () {
  'use strict';

  function esc(str) {
    if (!str) return '';
    return String(str).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
  }

  function timeAgo(isoStr) {
    if (!isoStr) return 'never';
    const diff = (Date.now() - new Date(isoStr + 'Z').getTime()) / 1000;
    if (diff < 60) return `${Math.floor(diff)}s ago`;
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    return `${Math.floor(diff / 86400)}d ago`;
  }

  function statusDot(status) {
    const cls = status === 'active' ? 'ipc-dot-active' : 'ipc-dot-idle';
    const label = status === 'active' ? 'Active' : 'Idle';
    return `<span class="ipc-status-dot ${cls}" aria-label="Agent ${label}" title="${label}"></span>`;
  }

  function renderAgentGrid(agents, worktrees) {
    const el = document.getElementById('ipc-agents-container');
    if (!el) return;

    const wtMap = {};
    for (const wt of worktrees) {
      wtMap[`${wt.agent}@${wt.host}`] = wt;
    }

    if (!agents.length) {
      el.innerHTML = `
        <div class="ipc-widget-header">
          <span class="header-icon" data-icon="worker"></span>
          <span>Agents</span>
          <span class="ipc-badge">0</span>
        </div>
        <div class="ipc-empty">No agents registered</div>`;
      return;
    }

    const cards = agents.map(a => {
      const wt = wtMap[`${a.agent_id}@${a.host}`];
      const branch = wt ? esc(wt.branch) : (a.branch ? esc(a.branch) : '—');
      return `
        <div class="ipc-agent-card">
          <div class="ipc-agent-header">
            ${statusDot(a.status)}
            <strong>${esc(a.agent_id)}</strong>
            <span class="ipc-agent-host">${esc(a.host)}</span>
          </div>
          <div class="ipc-agent-details">
            <span class="ipc-tag">${branch}</span>
            ${a.current_task ? `<span class="ipc-tag ipc-tag-task">${esc(a.current_task)}</span>` : ''}
          </div>
          <div class="ipc-agent-meta">
            <span title="Last heartbeat">${timeAgo(a.last_heartbeat)}</span>
            ${a.pid ? `<span class="ipc-pid">PID ${a.pid}</span>` : ''}
          </div>
        </div>`;
    }).join('');

    el.innerHTML = `
      <div class="ipc-widget-header">
        <span class="header-icon" data-icon="worker"></span>
        <span>Agents</span>
        <span class="ipc-badge">${agents.length}</span>
      </div>
      <div class="ipc-agent-grid">${cards}</div>`;
  }

  window.renderAgentGrid = renderAgentGrid;
})();
