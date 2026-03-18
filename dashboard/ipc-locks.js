/* ipc-locks.js — Locks, worktrees, conflicts display */
/* global dashlog */

(function () {
  'use strict';

  function esc(str) {
    if (!str) return '';
    return String(str).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
  }

  function renderIpcLocks(locks, worktrees, conflicts) {
    const el = document.getElementById('ipc-locks-container');
    if (!el) return;

    const conflictHtml = conflicts.length
      ? `<div class="ipc-conflicts">
           <h4 class="ipc-sub-header">⚠ Conflicts (${conflicts.length})</h4>
           ${conflicts.map(c => `
             <div class="ipc-conflict-item">
               <code>${esc(c.file_pattern)}</code>
               <span class="ipc-conflict-agents">${esc(c.agents)}</span>
             </div>`).join('')}
         </div>`
      : '';

    const lockRows = locks.length
      ? locks.map(l => `
          <tr>
            <td><code>${esc(l.file_pattern)}</code></td>
            <td>${esc(l.agent)}</td>
            <td>${esc(l.host)}</td>
            <td class="ipc-mono">${l.pid || '—'}</td>
          </tr>`).join('')
      : '<tr><td colspan="4" class="ipc-empty">No active locks</td></tr>';

    const wtRows = worktrees.length
      ? worktrees.map(w => `
          <tr>
            <td>${esc(w.agent)}</td>
            <td>${esc(w.host)}</td>
            <td><code>${esc(w.branch)}</code></td>
            <td class="ipc-mono" title="${esc(w.path)}">${esc(w.path?.split('/').pop() || w.path)}</td>
          </tr>`).join('')
      : '<tr><td colspan="4" class="ipc-empty">No worktrees registered</td></tr>';

    el.innerHTML = `
      <div class="ipc-widget-header">
        <span class="header-icon" data-icon="lock"></span>
        <span>Coordination</span>
      </div>
      ${conflictHtml}
      <h4 class="ipc-sub-header" id="ipc-locks-heading">File Locks (${locks.length})</h4>
      <table class="ipc-table" role="table" aria-labelledby="ipc-locks-heading">
        <thead><tr><th>Pattern</th><th>Agent</th><th>Host</th><th>PID</th></tr></thead>
        <tbody>${lockRows}</tbody>
      </table>
      <h4 class="ipc-sub-header" id="ipc-wt-heading">Worktrees (${worktrees.length})</h4>
      <table class="ipc-table" role="table" aria-labelledby="ipc-wt-heading">
        <thead><tr><th>Agent</th><th>Host</th><th>Branch</th><th>Path</th></tr></thead>
        <tbody>${wtRows}</tbody>
      </table>`;
  }

  window.renderIpcLocks = renderIpcLocks;
})();
