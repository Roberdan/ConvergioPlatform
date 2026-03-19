// drawer-chat.js — Right-side chat drawer with LLM streaming.
// Accessible from any view via toggle button or Ctrl+Shift+C.
// Uses streamSSE from lib/ws.js for token-by-token responses.
import { apiFetch } from '../lib/api-core.js';
import { streamSSE } from '../lib/ws.js';

const STYLE_ID = 'mn-drawer-chat-style';
let drawerEl = null;
let overlayEl = null;
let isOpen = false;
let activeSessionId = null;
let activeStream = null;
let models = [];
let sessions = [];

function injectStyles() {
  if (document.getElementById(STYLE_ID)) return;
  const s = document.createElement('style');
  s.id = STYLE_ID;
  s.textContent = `
    .dc-toggle { background:none; border:1px solid var(--mn-border);
      border-radius:var(--mn-radius,6px); padding:0.25rem 0.5rem;
      cursor:pointer; color:var(--mn-text-muted); font-size:0.8rem; }
    .dc-toggle:hover { color:var(--mn-text); background:var(--mn-surface-raised); }
    .dc-overlay { position:fixed; inset:0; z-index:199;
      background:rgba(0,0,0,0.15); opacity:0;
      transition:opacity 0.25s ease; pointer-events:none; }
    .dc-overlay--open { opacity:1; pointer-events:auto; }
    .dc-drawer { position:fixed; top:0; right:0; bottom:0;
      width:380px; max-width:90vw; z-index:200;
      transform:translateX(100%); transition:transform 0.25s ease;
      background:var(--mn-surface); border-left:1px solid var(--mn-border);
      display:flex; flex-direction:column; }
    .dc-drawer--open { transform:translateX(0); }
    .dc-header { display:flex; align-items:center; gap:0.5rem;
      padding:0.75rem 1rem; border-bottom:1px solid var(--mn-border); flex-shrink:0; }
    .dc-header__title { font-weight:600; font-size:0.95rem; color:var(--mn-text); flex:1; }
    .dc-model-select,.dc-session-select { font-size:0.75rem; padding:0.2rem 0.4rem;
      border:1px solid var(--mn-border); border-radius:var(--mn-radius,4px);
      background:var(--mn-surface-raised); color:var(--mn-text); max-width:120px; }
    .dc-session-bar { display:flex; gap:0.25rem; padding:0.4rem 1rem;
      border-bottom:1px solid var(--mn-border); flex-shrink:0; align-items:center; }
    .dc-session-bar button { font-size:0.7rem; padding:0.15rem 0.4rem;
      border:1px solid var(--mn-border); border-radius:4px;
      background:var(--mn-surface-raised); color:var(--mn-text-muted); cursor:pointer; }
    .dc-session-bar button:hover { color:var(--mn-text); }
    .dc-messages { flex:1; overflow-y:auto; padding:0.75rem 1rem;
      display:flex; flex-direction:column; gap:0.5rem; }
    .dc-msg { padding:0.5rem 0.75rem; border-radius:8px; font-size:0.85rem;
      line-height:1.4; max-width:85%; word-wrap:break-word; white-space:pre-wrap; }
    .dc-msg--user { align-self:flex-end; background:var(--mn-accent); color:#fff; }
    .dc-msg--assistant { align-self:flex-start; background:var(--mn-surface-raised);
      color:var(--mn-text); }
    .dc-input-bar { display:flex; gap:0.5rem; padding:0.75rem 1rem;
      border-top:1px solid var(--mn-border); flex-shrink:0; }
    .dc-input { flex:1; border:1px solid var(--mn-border);
      border-radius:var(--mn-radius,6px); padding:0.5rem 0.75rem; font-size:0.85rem;
      background:var(--mn-surface); color:var(--mn-text); resize:none; font-family:inherit; }
    .dc-input:focus { outline:2px solid var(--mn-accent); outline-offset:-1px; }
    .dc-send { border:none; background:var(--mn-accent); color:#fff;
      border-radius:var(--mn-radius,6px); padding:0.5rem 1rem; cursor:pointer;
      font-size:0.85rem; font-weight:600; }
    .dc-send:disabled { opacity:0.5; cursor:not-allowed; }
    .dc-close { background:none; border:none; cursor:pointer;
      color:var(--mn-text-muted); font-size:1.1rem; padding:0.25rem; }
    .dc-close:hover { color:var(--mn-text); }`;
  document.head.appendChild(s);
}

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

function optionHtml(id, label, selected) {
  return `<option value="${id}"${selected ? ' selected' : ''}>${label}</option>`;
}

function resolveIdLabel(item) {
  const id = typeof item === 'string' ? item : item.id;
  const label = typeof item === 'string' ? item : (item.name || item.id);
  return { id, label };
}

async function loadModels() {
  const res = await apiFetch('/api/chat/models');
  if (res instanceof Error) { console.warn('Failed to load chat models:', res.message); return; }
  models = Array.isArray(res) ? res : (res.models || []);
  const sel = drawerEl.querySelector('.dc-model-select');
  sel.innerHTML = models.map((m) => {
    const { id, label } = resolveIdLabel(m);
    return optionHtml(id, label, false);
  }).join('');
}

async function loadSessions() {
  const res = await apiFetch('/api/chat/sessions');
  if (res instanceof Error) { console.warn('Failed to load chat sessions:', res.message); return; }
  sessions = Array.isArray(res) ? res : (res.sessions || []);
  const sel = drawerEl.querySelector('.dc-session-select');
  sel.innerHTML = sessions.map((s) => {
    const { id, label } = resolveIdLabel(s);
    return optionHtml(id, label, id === activeSessionId);
  }).join('');
  if (sessions.length > 0) {
    const firstId = typeof sessions[0] === 'string' ? sessions[0] : sessions[0].id;
    switchSession(firstId);
  }
}

async function createSession() {
  const res = await apiFetch('/api/chat/sessions', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ name: `Session ${sessions.length + 1}` }),
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
  msgs.forEach((m) => appendBubble(m.role, m.content));
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

function sendMessage() {
  const input = drawerEl.querySelector('.dc-input');
  const text = input.value.trim();
  if (!text) return;
  input.value = '';
  appendBubble('user', text);
  const model = drawerEl.querySelector('.dc-model-select').value;
  const bubble = appendBubble('assistant', '');
  const sendBtn = drawerEl.querySelector('.dc-send');
  sendBtn.disabled = true;
  const params = new URLSearchParams({ model, message: text });
  if (activeSessionId) params.set('session_id', activeSessionId);
  activeStream = streamSSE(`/api/chat/stream?${params}`, {
    onEvent(_type, data) {
      const token = typeof data === 'string' ? data : (data.token || data.content || '');
      bubble.textContent += token;
      drawerEl.querySelector('.dc-messages').scrollTop =
        drawerEl.querySelector('.dc-messages').scrollHeight;
    },
    onDone() { sendBtn.disabled = false; activeStream = null; },
    onError(err) {
      console.warn('Chat SSE error:', err.message);
      if (!bubble.textContent) bubble.textContent = '[Error: streaming failed]';
      sendBtn.disabled = false;
      activeStream = null;
    },
  });
}

/** Initialize drawer-chat: mount toggle button in command strip, bind Ctrl+Shift+C. */
export function initDrawerChat() {
  injectStyles();
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
