// ipc-router.js — Route visualization panel for dashboard
// Uses Maranello DS components (mn-widget, mn-card)

async function renderIpcRouter(container) {
  if (!container) return;
  const data = await fetchJson('/api/ipc/route-history');
  if (!data || !data.history || !data.history.length) {
    container.innerHTML = '<mn-widget title="Route History"><p>No route history</p></mn-widget>';
    return;
  }
  const cards = data.history.map(entry => {
    const confidence = ((entry.cost || 0) > 0) ? 'cloud' : 'local';
    const reason = entry.model ? `→ ${entry.model}` : '';
    return `
      <mn-card class="mn-card route-card" style="margin-bottom:8px;padding:8px">
        <div style="display:flex;justify-content:space-between;align-items:center">
          <div>
            <strong style="font-size:12px">${entry.task || 'task'}</strong>
            <span class="mn-badge" style="margin-left:6px;font-size:10px">${entry.model || '?'}</span>
          </div>
          <mn-badge variant="${confidence}">${confidence}</mn-badge>
        </div>
        <div style="font-size:11px;color:#94a3b8;margin-top:4px">
          <span>Tokens: ${entry.tokens_in || 0}→${entry.tokens_out || 0}</span>
          <span style="margin-left:12px">Cost: $${(entry.cost || 0).toFixed(4)}</span>
          <span style="margin-left:12px">Sub: ${entry.subscription || '-'}</span>
        </div>
        <div style="font-size:10px;color:#64748b;margin-top:2px">${entry.date || ''} ${reason}</div>
      </mn-card>`;
  }).join('');
  container.innerHTML = `<mn-widget title="Route History">${cards}</mn-widget>`;
}

window.renderIpcRouter = renderIpcRouter;
