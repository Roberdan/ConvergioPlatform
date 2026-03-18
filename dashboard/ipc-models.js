// ipc-models.js — Model registry + auth status panel for dashboard
// Uses Maranello DS components (mn-widget, mn-badge, mn-status-dot)

async function renderIpcModels(container) {
  if (!container) return;
  const [modelsData, authData] = await Promise.all([
    fetchJson('/api/ipc/models'),
    fetchJson('/api/ipc/auth-status'),
  ]);

  let html = '';

  // Model registry table
  const models = modelsData?.models || [];
  if (models.length) {
    const grouped = {};
    models.forEach(m => {
      const key = `${m.host}/${m.provider}`;
      if (!grouped[key]) grouped[key] = [];
      grouped[key].push(m);
    });
    const rows = Object.entries(grouped).map(([key, ms]) => {
      const [host, provider] = key.split('/');
      return ms.map((m, i) => `
        <tr>
          ${i === 0 ? `<td rowspan="${ms.length}" style="font-weight:600">${host}</td><td rowspan="${ms.length}"><mn-badge variant="info">${provider}</mn-badge></td>` : ''}
          <td>${m.model}</td>
          <td>${m.size_gb > 0 ? m.size_gb.toFixed(1) + ' GB' : '-'}</td>
          <td>${m.quantization || '-'}</td>
          <td><mn-status-dot variant="success"></mn-status-dot> ${m.last_seen || ''}</td>
        </tr>`).join('');
    }).join('');
    html += `
      <mn-widget title="Model Registry">
        <table class="mn-table" style="width:100%;font-size:12px;border-collapse:collapse">
          <tr><th>Host</th><th>Provider</th><th>Model</th><th>Size</th><th>Quant</th><th>Status</th></tr>
          ${rows}
        </table>
        <div style="font-size:11px;color:#64748b;margin-top:4px">
          ${models.length} model(s) across ${Object.keys(grouped).length} provider(s)
        </div>
      </mn-widget>`;
  } else {
    html += '<mn-widget title="Model Registry"><p>No models detected. Start ollama or LMStudio.</p></mn-widget>';
  }

  // Auth status indicator
  const tokens = authData?.tokens || [];
  const health = authData?.health || {};
  const services = ['claude', 'gh', 'opencode'];
  const auth_status = services.map(svc => {
    const tok = tokens.find(t => t.service === svc);
    const variant = tok ? 'valid' : 'missing';
    const color = tok ? '#22c55e' : tok === undefined ? '#6b7280' : '#ef4444';
    const label = tok ? 'valid' : 'missing';
    return `<span style="margin-right:12px"><mn-status-dot style="color:${color}"></mn-status-dot> ${svc}: <mn-badge variant="${variant === 'valid' ? 'success' : 'warning'}">${label}</mn-badge></span>`;
  }).join('');
  html += `
    <mn-widget title="Auth Status" class="mn-card">
      <div style="display:flex;align-items:center;flex-wrap:wrap;gap:8px">${auth_status}</div>
      <div style="font-size:11px;color:#64748b;margin-top:4px">
        ${health.total_tokens || 0} token(s) across ${health.hosts_with_tokens || 0} host(s)
        ${health.services?.length ? ` · Services: ${health.services.join(', ')}` : ''}
      </div>
    </mn-widget>`;

  container.innerHTML = html;
}

window.renderIpcModels = renderIpcModels;
