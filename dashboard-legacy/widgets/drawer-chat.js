// drawer-chat.js — Chat drawer with real LLM streaming via POST + SSE.
import { apiFetch } from '../lib/api-core.js';
import { streamSSE } from '../lib/ws.js';
import { injectChatStyles } from './drawer-chat-styles.js';

let drawerEl = null, overlayEl = null, isOpen = false;
let activeSessionId = null, activeStream = null;
let models = [], sessions = [];

function buildDrawer() {
  overlayEl = document.createElement('div');
  overlayEl.className = 'dc-overlay';
  overlayEl.addEventListener('click', toggleDrawer);
  const drawer = document.createElement('div');
  drawer.className = 'dc-drawer';
  drawer.setAttribute('role', 'complementary');
  drawer.setAttribute('aria-label', 'Chat drawer');
  drawer.innerHTML = `
    <div class="dc-header">
      <span class="dc-header__title">Chat</span>
      <select class="dc-model-select" aria-label="Model selector"></select>
      <button class="dc-close" aria-label="Close chat" title="Close">&times;</button>
    </div>
    <div class="dc-session-bar">
      <select class="dc-session-select" aria-label="Session selector"></select>
      <button data-action="new-session" title="New session">+</button>
      <button data-action="del-session" title="Delete session">&#128465;</button>
    </div>
    <div class="dc-messages" aria-live="polite" aria-relevant="additions"></div>
    <div class="dc-input-bar">
      <textarea class="dc-input" rows="1" placeholder="Type a message..."
        aria-label="Chat message input"></textarea>
      <button class="dc-send" aria-label="Send message">Send</button>
    </div>`;
  document.body.appendChild(overlayEl);
  document.body.appendChild(drawer);
  drawer.querySelector('.dc-close').addEventListener('click', toggleDrawer);
  drawer.querySelector('.dc-send').addEventListener('click', sendMessage);
  drawer.querySelector('[data-action="new-session"]').addEventListener('click', createSession);
  drawer.querySelector('[data-action="del-session"]').addEventListener('click', deleteSession);
  drawer.querySelector('.dc-session-select')
    .addEventListener('change', (e) => switchSession(e.target.value));
  drawer.querySelector('.dc-input').addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); sendMessage(); }
  });
  drawerEl = drawer;
}

export function toggleDrawer() {
  if (!drawerEl) { buildDrawer(); loadModels(); loadSessions(); }
  isOpen = !isOpen;
  drawerEl.classList.toggle('dc-drawer--open', isOpen);
  overlayEl.classList.toggle('dc-overlay--open', isOpen);
  if (isOpen) drawerEl.querySelector('.dc-input').focus();
}

/** Build model option label with capability display (context window, cost, provider). */
function modelOptionLabel(m) {
  if (typeof m === 'string') return m;
  const parts = [m.name || m.id];
  if (m.context_window) parts.push(`${Math.round(m.context_window / 1000)}k`);
  if (m.cost_label) parts.push(m.cost_label);
  if (m.provider) parts.push(`[${m.provider}]`);
  return parts.join(' | ');
}

const optionHtml = (id, label, sel) =>
  `<option value="${id}"${sel ? ' selected' : ''}>${label}</option>`;

async function loadModels() {
  const res = await apiFetch('/api/chat/models');
  if (res instanceof Error) { console.warn('Failed to load chat models:', res.message); return; }
  models = Array.isArray(res) ? res : (res.models || []);
  const sel = drawerEl.querySelector('.dc-model-select');
  sel.innerHTML = models.map((m) => {
    const id = typeof m === 'string' ? m : m.id;
    return optionHtml(id, modelOptionLabel(m), false);
  }).join('');
}

async function loadSessions() {
  const res = await apiFetch('/api/chat/sessions');
  if (res instanceof Error) { console.warn('Failed to load chat sessions:', res.message); return; }
  sessions = Array.isArray(res) ? res : (res.sessions || []);
  const sel = drawerEl.querySelector('.dc-session-select');
  sel.innerHTML = sessions.map((s) => {
    const id = typeof s === 'string' ? s : s.id;
    const label = typeof s === 'string' ? s : (s.name || s.id);
    return optionHtml(id, label, id === activeSessionId);
  }).join('');
  if (sessions.length > 0 && !activeSessionId) {
    const firstId = typeof sessions[0] === 'string' ? sessions[0] : sessions[0].id;
    switchSession(firstId);
  }
}

async function createSession() {
  const model = drawerEl.querySelector('.dc-model-select').value;
  const res = await apiFetch('/api/chat/session', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ name: `Session ${sessions.length + 1}`, model }),
  });
  if (res instanceof Error) { console.warn('Failed to create session:', res.message); return; }
  await loadSessions();
  const newId = res.id || res.session_id;
  if (newId) switchSession(newId);
}

async function deleteSession() {
  if (!activeSessionId) return;
  const res = await apiFetch(
    `/api/chat/sessions/${encodeURIComponent(activeSessionId)}`, { method: 'DELETE' }
  );
  if (res instanceof Error) { console.warn('Failed to delete session:', res.message); return; }
  activeSessionId = null;
  drawerEl.querySelector('.dc-messages').innerHTML = '';
  await loadSessions();
}

async function switchSession(sessionId) {
  if (activeStream) { activeStream.close(); activeStream = null; }
  activeSessionId = sessionId;
  const msgContainer = drawerEl.querySelector('.dc-messages');
  msgContainer.innerHTML = '';
  drawerEl.querySelector('.dc-session-select').value = sessionId;
  const res = await apiFetch(
    `/api/chat/sessions/${encodeURIComponent(sessionId)}/messages`
  );
  if (res instanceof Error) return;
  const msgs = Array.isArray(res) ? res : (res.messages || []);
  msgs.forEach((m) => {
    appendBubble(m.role, m.content);
    if (m.role === 'assistant' && m.usage) appendUsageStats(m.usage);
  });
}

function appendBubble(role, text) {
  const container = drawerEl.querySelector('.dc-messages');
  const div = document.createElement('div');
  div.className = `dc-msg dc-msg--${role === 'user' ? 'user' : 'assistant'}`;
  div.textContent = text;
  container.appendChild(div);
  container.scrollTop = container.scrollHeight;
  return div;
}

/** Append token usage and cost stats below the last assistant bubble. */
function appendUsageStats(usage) {
  const container = drawerEl.querySelector('.dc-messages');
  const div = document.createElement('div');
  div.className = 'dc-usage';
  const parts = [];
  const tokenCount = usage.total_tokens || usage.completion_tokens || 0;
  if (tokenCount) parts.push(`${tokenCount} tokens`);
  if (usage.cost != null) parts.push(`$${Number(usage.cost).toFixed(4)}`);
  if (usage.model) parts.push(usage.model);
  div.textContent = parts.join(' \u00b7 ') || 'usage unavailable';
  container.appendChild(div);
}

/** POST message to API, then stream response via SSE with typing animation. */
async function sendMessage() {
  const input = drawerEl.querySelector('.dc-input');
  const text = input.value.trim();
  if (!text) return;
  input.value = '';
  appendBubble('user', text);
  const model = drawerEl.querySelector('.dc-model-select').value;
  const sendBtn = drawerEl.querySelector('.dc-send');
  sendBtn.disabled = true;

  // Auto-create session if none active
  if (!activeSessionId) {
    const sRes = await apiFetch('/api/chat/session', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'Auto', model }),
    });
    if (sRes instanceof Error) {
      console.warn('Failed to auto-create session:', sRes.message);
      appendBubble('assistant', '[Error: could not create session]');
      sendBtn.disabled = false;
      return;
    }
    activeSessionId = sRes.id || sRes.session_id;
    await loadSessions();
  }
  const msgRes = await apiFetch('/api/chat/message', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ session_id: activeSessionId, model, content: text }),
  });
  if (msgRes instanceof Error) {
    console.warn('Failed to send message:', msgRes.message);
    appendBubble('assistant', '[Error: failed to send message]');
    sendBtn.disabled = false;
    return;
  }

  const streamId = msgRes.stream_id || msgRes.sid || activeSessionId;
  const bubble = appendBubble('assistant', '');
  bubble.classList.add('dc-msg--typing');
  let streamUsage = null;

  // Connect SSE for token-by-token streaming
  activeStream = streamSSE(`/api/chat/stream/${encodeURIComponent(streamId)}`, {
    onEvent(type, data) {
      const token = typeof data === 'string' ? data : (data.token || data.content || '');
      if (type === 'usage' && typeof data === 'object') {
        streamUsage = data;
        return;
      }
      bubble.textContent += token;
      drawerEl.querySelector('.dc-messages').scrollTop =
        drawerEl.querySelector('.dc-messages').scrollHeight;
    },
    onDone(data) {
      bubble.classList.remove('dc-msg--typing');
      const usage = (typeof data === 'object' && data) ? (data.usage || data) : streamUsage;
      if (usage) appendUsageStats(usage);
      sendBtn.disabled = false;
      activeStream = null;
    },
    onError(err) {
      console.warn('Chat SSE error:', err.message);
      bubble.classList.remove('dc-msg--typing');
      if (!bubble.textContent) bubble.textContent = '[Error: streaming failed]';
      sendBtn.disabled = false;
      activeStream = null;
    },
  });
}

/** Initialize drawer-chat: mount toggle button in command strip, bind Ctrl+Shift+C. */
export function initDrawerChat() {
  injectChatStyles();
  const strip = document.querySelector('.cr-command-strip');
  if (strip) {
    const btn = document.createElement('button');
    btn.className = 'dc-toggle';
    btn.textContent = 'Chat';
    btn.title = 'Toggle chat (Ctrl+Shift+C)';
    btn.setAttribute('aria-label', 'Toggle chat drawer');
    btn.addEventListener('click', toggleDrawer);
    strip.appendChild(btn);
  }
  document.addEventListener('keydown', (e) => {
    if (e.ctrlKey && e.shiftKey && e.key === 'C') { e.preventDefault(); toggleDrawer(); }
  });
}
