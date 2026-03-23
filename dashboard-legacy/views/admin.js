/**
 * Admin view — 3 tabs: Nightly Jobs, System, Chat.
 * REST from api-core.js, SSE streaming from lib/ws.js.
 */
import { streamSSE } from '../lib/ws.js';
import { postChatMessage } from '../lib/api-ipc.js';

const STYLE_ID = 'mn-admin-view-style';
function injectStyles() {
  if (document.getElementById(STYLE_ID)) return;
  const s = document.createElement('style');
  s.id = STYLE_ID;
  s.textContent = `
    .admin-cards{display:grid;grid-template-columns:repeat(auto-fill,minmax(180px,1fr));gap:1rem;padding:1rem 0}
    .admin-card{background:var(--mn-surface-raised);border-radius:8px;padding:1rem;text-align:center}
    .admin-card__val{font-size:1.5rem;font-weight:600;color:var(--mn-text)}
    .admin-card__lbl{color:var(--mn-text-muted);font-size:.85rem}
    .admin-events{max-height:300px;overflow-y:auto;padding:.5rem 0}
    .admin-evt{padding:.35rem .5rem;border-bottom:1px solid var(--mn-border);font-size:.85rem}
    .admin-chat{display:flex;flex-direction:column;height:100%;min-height:400px}
    .admin-chat__msgs{flex:1;overflow-y:auto;padding:.5rem;background:var(--mn-surface);border-radius:8px;margin-bottom:.75rem}
    .admin-chat__msg{padding:.4rem .6rem;margin-bottom:.4rem;border-radius:6px;font-size:.9rem}
    .admin-chat__msg--user{background:var(--mn-accent);color:var(--mn-text-on-accent,#fff);margin-left:20%;text-align:right}
    .admin-chat__msg--assistant{background:var(--mn-surface-raised);color:var(--mn-text);margin-right:20%}
    .admin-chat__row{display:flex;gap:.5rem}
    .admin-chat__in{flex:1;padding:.5rem .75rem;border:1px solid var(--mn-border);border-radius:6px;background:var(--mn-surface);color:var(--mn-text);font-size:.9rem}
    .admin-loading{color:var(--mn-text-muted);padding:1rem}
    .admin-error{color:var(--signal-danger);padding:.5rem}`;
  document.head.appendChild(s);
}

function esc(v) { const d = document.createElement('div'); d.textContent = v; return d.innerHTML; }

// -- Tab 1: Nightly Jobs --

const JOB_COLS = [
  { key: 'job_name', label: 'Job' }, { key: 'status', label: 'Status' },
  { key: 'host', label: 'Host' }, { key: 'duration_sec', label: 'Duration (s)' },
  { key: 'processed_items', label: 'Processed' }, { key: 'fixed_items', label: 'Fixed' },
  { key: 'started_at', label: 'Started At' },
];

function renderNightlyTab(tab, api) {
  tab.innerHTML = '<div class="admin-loading">Loading jobs...</div>';
  api.fetchNightlyJobs().then(result => {
    if (result instanceof Error || !result?.jobs) {
      tab.innerHTML = '<div class="admin-error">Failed to load nightly jobs.</div>';
      console.warn('[admin] fetchNightlyJobs failed', result);
      return;
    }
    tab.innerHTML = '';
    const table = document.createElement('mn-data-table');
    table.setAttribute('columns', JSON.stringify(JOB_COLS));
    table.setAttribute('rows', JSON.stringify(result.jobs));
    table.setAttribute('selectable', '');
    table.addEventListener('mn-row-click', (e) => showJobDetail(api, e.detail.row));
    tab.appendChild(table);
  }).catch(err => { tab.innerHTML = `<div class="admin-error">${esc(err.message)}</div>`; });
}

async function showJobDetail(api, job) {
  const result = await api.fetchNightlyJobDetail(job.id || job.job_name);
  const modal = document.createElement('mn-modal');
  modal.setAttribute('heading', `Job: ${job.job_name}`);
  modal.setAttribute('open', '');
  const content = document.createElement('div');
  if (result instanceof Error || !result) {
    content.innerHTML = '<div class="admin-error">Failed to load job detail.</div>';
  } else {
    const d = result.job || result;
    const pre = (label, text, color) => text
      ? `<h4>${label}</h4><pre style="max-height:200px;overflow:auto;background:var(--mn-surface);padding:.5rem;border-radius:4px${color ? ';color:var(--signal-danger)' : ''}">${esc(text)}</pre>`
      : '';
    content.innerHTML = `
      <p><strong>Status:</strong> ${esc(d.status || job.status)}</p>
      <p><strong>Host:</strong> ${esc(d.host || job.host || '')}</p>
      <p><strong>Duration:</strong> ${d.duration_sec ?? ''}s</p>
      ${pre('stdout', d.stdout, false)}${pre('stderr', d.stderr, true)}`;
  }
  modal.appendChild(content);
  document.body.appendChild(modal);
  modal.addEventListener('mn-close', () => modal.remove());
}

// -- Tab 2: System --

function renderSystemTab(tab, api) {
  tab.innerHTML = '<div class="admin-loading">Loading system status...</div>';
  Promise.allSettled([api.fetchCoordinatorStatus(), api.fetchEvents()]).then(([coordRes, evtRes]) => {
    tab.innerHTML = '';
    // Status cards
    if (coordRes.status === 'fulfilled' && !(coordRes.value instanceof Error)) {
      const c = coordRes.value;
      const cards = document.createElement('div');
      cards.className = 'admin-cards';
      for (const [label, value] of [
        ['Coordinator', c.running ? 'Running' : 'Stopped'],
        ['PID', c.pid ?? 'N/A'],
        ['Pending Events', c.pending_events ?? 0],
      ]) {
        const card = document.createElement('div');
        card.className = 'admin-card';
        card.innerHTML = `<div class="admin-card__val">${esc(String(value))}</div><div class="admin-card__lbl">${esc(label)}</div>`;
        cards.appendChild(card);
      }
      tab.appendChild(cards);
    } else {
      tab.innerHTML += '<div class="admin-error">Failed to load coordinator status.</div>';
    }
    // Events feed
    if (evtRes.status === 'fulfilled' && !(evtRes.value instanceof Error)) {
      const events = evtRes.value?.events || evtRes.value || [];
      if (Array.isArray(events) && events.length > 0) {
        const h = document.createElement('h4');
        h.textContent = 'Recent Events';
        h.style.margin = '1rem 0 .5rem';
        const list = document.createElement('div');
        list.className = 'admin-events';
        for (const evt of events.slice(0, 50)) {
          const row = document.createElement('div');
          row.className = 'admin-evt';
          row.textContent = typeof evt === 'string' ? evt : `${evt.timestamp || ''} ${evt.type || ''} ${evt.message || ''}`.trim();
          list.appendChild(row);
        }
        tab.append(h, list);
      }
    }
  });
}

// -- Tab 3: Chat --

function renderChatTab(tab) {
  tab.innerHTML = '';
  const wrap = document.createElement('div');
  wrap.className = 'admin-chat';
  const msgs = document.createElement('div');
  msgs.className = 'admin-chat__msgs';
  const row = document.createElement('div');
  row.className = 'admin-chat__row';
  const input = document.createElement('input');
  input.className = 'admin-chat__in';
  input.type = 'text';
  input.placeholder = 'Type a message...';
  const btn = document.createElement('button');
  btn.className = 'mn-btn mn-btn--primary';
  btn.textContent = 'Send';
  row.append(input, btn);
  wrap.append(msgs, row);
  tab.appendChild(wrap);

  const sid = `admin-${Date.now()}`;
  let sseHandle = null;

  function appendMsg(text, role) {
    const div = document.createElement('div');
    div.className = `admin-chat__msg admin-chat__msg--${role}`;
    div.textContent = text;
    msgs.appendChild(div);
    msgs.scrollTop = msgs.scrollHeight;
  }

  async function send() {
    const text = input.value.trim();
    if (!text) return;
    input.value = '';
    appendMsg(text, 'user');
    if (sseHandle) { sseHandle.close(); sseHandle = null; }

    const result = await postChatMessage(sid, text);
    if (result instanceof Error) { appendMsg('Error: ' + result.message, 'assistant'); return; }

    let buffer = '';
    sseHandle = streamSSE(`/api/chat/stream/${encodeURIComponent(sid)}`, {
      onEvent(_type, data) { buffer += (typeof data === 'string' ? data : data?.text || ''); },
      onDone() { if (buffer) appendMsg(buffer, 'assistant'); buffer = ''; sseHandle = null; },
      onError(err) { appendMsg('Stream error: ' + err.message, 'assistant'); sseHandle = null; },
    });
  }

  btn.addEventListener('click', send);
  input.addEventListener('keydown', (e) => { if (e.key === 'Enter') send(); });
  tab._chatCleanup = () => { if (sseHandle) sseHandle.close(); };
}

// -- Main view factory --

/** @param {HTMLElement} container @param {{api: object, store: object}} deps @returns {Function} */
export default function admin(container, { api, store }) {
  injectStyles();
  container.innerHTML = '';
  const tabs = document.createElement('mn-tabs');

  const nightlyTab = document.createElement('mn-tab');
  nightlyTab.setAttribute('label', 'Nightly Jobs');
  renderNightlyTab(nightlyTab, api);

  const systemTab = document.createElement('mn-tab');
  systemTab.setAttribute('label', 'System');
  renderSystemTab(systemTab, api);

  const chatTab = document.createElement('mn-tab');
  chatTab.setAttribute('label', 'Chat');
  renderChatTab(chatTab);

  tabs.append(nightlyTab, systemTab, chatTab);
  container.appendChild(tabs);

  return () => {
    if (chatTab._chatCleanup) chatTab._chatCleanup();
    container.innerHTML = '';
  };
}
