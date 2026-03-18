/* dashlog.js — Centralized dashboard logger. No silent failures. */
;(function() {
  'use strict';
  const MAX_ENTRIES = 200;
  const LOG_LEVELS = { debug: 0, info: 1, warn: 2, error: 3 };
  const entries = [];
  let panelEl = null;
  let minLevel = LOG_LEVELS.info;

  function ts() { return new Date().toLocaleTimeString('en-GB', { hour12: false }); }

  function add(level, source, endpoint, detail) {
    if (LOG_LEVELS[level] < minLevel) return;
    const entry = { ts: ts(), level, source, endpoint, detail };
    entries.push(entry);
    if (entries.length > MAX_ENTRIES) entries.shift();
    console[level === 'error' ? 'error' : level === 'warn' ? 'warn' : 'log'](
      `[DashLog:${level.toUpperCase()}] ${source} ${endpoint} ${detail}`
    );
    renderPanel();
    if (level === 'error') showErrorBadge();
  }

  function showErrorBadge() {
    let badge = document.getElementById('dashlog-error-badge');
    if (!badge) {
      badge = document.createElement('button');
      badge.id = 'dashlog-error-badge';
      badge.title = 'Errors detected — click to open log';
      badge.style.cssText = 'position:fixed;bottom:12px;right:12px;z-index:99998;background:#ff3355;color:#fff;border:none;border-radius:50%;width:32px;height:32px;font:bold 13px monospace;cursor:pointer;box-shadow:0 2px 8px rgba(255,51,85,0.4);';
      badge.onclick = () => togglePanel();
      document.body.appendChild(badge);
    }
    const errCount = entries.filter(e => e.level === 'error').length;
    badge.textContent = errCount > 99 ? '99+' : errCount;
    badge.style.display = errCount > 0 ? 'block' : 'none';
  }

  function togglePanel() {
    if (panelEl) { panelEl.remove(); panelEl = null; return; }
    panelEl = document.createElement('div');
    panelEl.id = 'dashlog-panel';
    panelEl.style.cssText = 'position:fixed;bottom:50px;right:12px;width:600px;max-height:400px;overflow:auto;z-index:99999;background:#0a0a0f;border:1px solid #2a2a3a;border-radius:8px;font:11px/1.6 monospace;color:#c8d0e8;box-shadow:0 4px 20px rgba(0,0,0,0.6);';
    const header = document.createElement('div');
    header.style.cssText = 'display:flex;justify-content:space-between;align-items:center;padding:8px 12px;border-bottom:1px solid #2a2a3a;position:sticky;top:0;background:#0a0a0f;';
    header.innerHTML = '<span style="color:#ff3355;font-weight:bold;">DASHLOG</span>';
    const controls = document.createElement('span');
    const clearBtn = document.createElement('button');
    clearBtn.textContent = 'Clear';
    clearBtn.style.cssText = 'background:none;border:1px solid #444;color:#888;padding:2px 8px;border-radius:4px;cursor:pointer;font:10px monospace;margin-right:6px;';
    clearBtn.onclick = () => { entries.length = 0; renderPanel(); showErrorBadge(); };
    const closeBtn = document.createElement('button');
    closeBtn.textContent = '✕';
    closeBtn.style.cssText = 'background:none;border:none;color:#888;cursor:pointer;font:14px monospace;';
    closeBtn.onclick = () => togglePanel();
    controls.appendChild(clearBtn);
    controls.appendChild(closeBtn);
    header.appendChild(controls);
    panelEl.appendChild(header);
    const body = document.createElement('div');
    body.id = 'dashlog-body';
    body.style.cssText = 'padding:6px 12px;';
    panelEl.appendChild(body);
    document.body.appendChild(panelEl);
    renderPanel();
  }

  function renderPanel() {
    const body = document.getElementById('dashlog-body');
    if (!body) return;
    const colors = { error: '#ff3355', warn: '#ffb700', info: '#4EA8DE', debug: '#5a6080' };
    body.innerHTML = entries.slice(-50).reverse().map(e =>
      `<div style="border-bottom:1px solid #1a1a2a;padding:3px 0;"><span style="color:#5a6080">${e.ts}</span> <span style="color:${colors[e.level]};font-weight:bold">${e.level.toUpperCase().padEnd(5)}</span> <span style="color:#c8d0e8">${esc(e.source)}</span> ${e.endpoint ? `<span style="color:#888">${esc(e.endpoint)}</span>` : ''} ${e.detail ? `<span style="color:#aaa">${esc(e.detail)}</span>` : ''}</div>`
    ).join('');
  }

  function esc(s) { const d = document.createElement('span'); d.textContent = s || ''; return d.innerHTML; }

  // Capture unhandled errors
  const origOnError = window.onerror;
  window.onerror = function(msg, src, line, col, err) {
    const file = src ? src.split('/').pop().split('?')[0] : '?';
    add('error', `${file}:${line}`, '', String(msg));
    if (origOnError) return origOnError.apply(this, arguments);
  };
  const origOnReject = window.onunhandledrejection;
  window.onunhandledrejection = function(ev) {
    const reason = ev.reason;
    const msg = reason && reason.message ? reason.message : String(reason);
    const stack = reason && reason.stack ? reason.stack.split('\n')[1] || '' : '';
    add('error', 'Promise', stack.trim(), msg);
    if (origOnReject) return origOnReject.apply(this, arguments);
  };

  window.DashLog = {
    debug: (src, ep, detail) => add('debug', src, ep, detail),
    info: (src, ep, detail) => add('info', src, ep, detail),
    warn: (src, ep, detail) => add('warn', src, ep, detail),
    error: (src, ep, detail) => add('error', src, ep, detail),
    setLevel: (lvl) => { if (LOG_LEVELS[lvl] !== undefined) minLevel = LOG_LEVELS[lvl]; },
    toggle: togglePanel,
    entries: () => [...entries],
  };
})();
