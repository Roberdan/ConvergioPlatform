/* ipc-messages.js — Message feed + send for IPC panel */
/* global dashlog */

(function () {
  'use strict';

  function esc(str) {
    if (!str) return '';
    return String(str).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
  }

  function timeShort(isoStr) {
    if (!isoStr) return '';
    try {
      const d = new Date(isoStr + 'Z');
      return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    } catch { return isoStr; }
  }

  function renderMessageFeed(messages, channels) {
    const el = document.getElementById('ipc-messages-container');
    if (!el) return;

    const channelTabs = (channels || []).map(ch =>
      `<button class="ipc-channel-tab" data-channel="${esc(ch.name)}">${esc(ch.name)}</button>`
    ).join('');

    const msgHtml = messages.length
      ? messages.map(m => `
          <div class="ipc-message">
            <div class="ipc-msg-header">
              <strong class="ipc-msg-sender">${esc(m.sender)}</strong>
              <span class="ipc-msg-channel">#${esc(m.channel)}</span>
              <span class="ipc-msg-time">${timeShort(m.created_at)}</span>
            </div>
            <div class="ipc-msg-content">${esc(m.content)}</div>
          </div>`).join('')
      : '<div class="ipc-empty">No messages yet</div>';

    el.innerHTML = `
      <div class="ipc-widget-header">
        <span class="header-icon" data-icon="chat"></span>
        <span>Messages</span>
        <span class="ipc-badge">${messages.length}</span>
      </div>
      ${channelTabs ? `<div class="ipc-channel-tabs">${channelTabs}</div>` : ''}
      <div class="ipc-message-list">${msgHtml}</div>
      <div class="ipc-send-bar">
        <input type="text" id="ipc-msg-input" class="ipc-input" placeholder="Send a message…"
               aria-label="Message content" />
        <button class="mn-btn mn-btn--accent mn-btn--sm" id="ipc-send-btn" aria-label="Send message">
          Send
        </button>
      </div>`;

    const sendBtn = el.querySelector('#ipc-send-btn');
    const msgInput = el.querySelector('#ipc-msg-input');
    if (sendBtn && msgInput) {
      const doSend = async () => {
        const content = msgInput.value.trim();
        if (!content) return;
        try {
          await fetch('/api/ipc/send', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ content, sender_name: 'dashboard', channel: 'general' }),
          });
          msgInput.value = '';
          if (typeof window.refreshIpc === 'function') window.refreshIpc();
        } catch (err) {
          if (typeof dashlog !== 'undefined') dashlog.error('ipc-messages', 'send failed', err);
        }
      };
      sendBtn.addEventListener('click', doSend);
      msgInput.addEventListener('keydown', e => { if (e.key === 'Enter') doSend(); });
    }
  }

  window.renderMessageFeed = renderMessageFeed;
})();
