// drawer-chat-styles.js — CSS for the chat drawer widget.
// Extracted to keep drawer-chat.js under 250 lines.

const STYLE_ID = 'mn-drawer-chat-style';

export function injectChatStyles() {
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
      background:var(--mn-surface-raised); color:var(--mn-text); max-width:140px; }
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
    .dc-msg--typing { border-right:2px solid var(--mn-accent);
      animation:dc-blink 0.6s step-end infinite; }
    @keyframes dc-blink { 50% { border-color:transparent; } }
    .dc-usage { font-size:0.7rem; color:var(--mn-text-muted); padding:0.15rem 0.75rem;
      align-self:flex-start; }
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
    .dc-close:hover { color:var(--mn-text); }
    .dc-badge { display:inline-block; font-size:0.6rem; padding:0.05rem 0.3rem;
      border-radius:3px; background:var(--mn-accent); color:#fff;
      margin-left:0.25rem; vertical-align:middle; }`;
  document.head.appendChild(s);
}
