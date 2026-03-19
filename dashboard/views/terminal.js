/**
 * Terminal view — xterm.js with multi-tab session support.
 * Uses global Terminal and FitAddon loaded in index.html.
 * WebSocket PTY connection via lib/ws.js.
 */

const STYLE_ID = 'mn-terminal-view-style';

function injectStyles() {
  if (document.getElementById(STYLE_ID)) return;
  const style = document.createElement('style');
  style.id = STYLE_ID;
  style.textContent = `
    .terminal-view {
      display: flex;
      flex-direction: column;
      height: 100%;
      min-height: 500px;
    }
    .terminal-tabs {
      display: flex;
      gap: 0;
      border-bottom: 2px solid var(--mn-border);
      padding: 0 0.5rem;
      flex-shrink: 0;
    }
    .terminal-tabs__tab {
      padding: 0.5rem 1rem;
      cursor: pointer;
      border: none;
      background: transparent;
      color: var(--mn-text-muted);
      font-size: 0.85rem;
      border-bottom: 2px solid transparent;
      margin-bottom: -2px;
      transition: color 0.15s, border-color 0.15s;
    }
    .terminal-tabs__tab:hover {
      color: var(--mn-text);
    }
    .terminal-tabs__tab--active {
      color: var(--mn-accent);
      border-bottom-color: var(--mn-accent);
    }
    .terminal-tabs__add {
      padding: 0.5rem 0.75rem;
      cursor: pointer;
      border: none;
      background: transparent;
      color: var(--mn-text-muted);
      font-size: 1rem;
    }
    .terminal-tabs__add:hover { color: var(--mn-accent); }
    .terminal-container {
      flex: 1;
      min-height: 400px;
      background: var(--mn-surface);
      border-radius: 0 0 8px 8px;
    }
  `;
  document.head.appendChild(style);
}

function getCSSVar(name) {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}

function buildTermTheme() {
  return {
    background: getCSSVar('--mn-surface') || '#1e1e1e',
    foreground: getCSSVar('--mn-text') || '#d4d4d4',
    cursor: getCSSVar('--mn-accent') || '#007acc',
    selectionBackground: getCSSVar('--mn-surface-raised') || '#264f78',
  };
}

/**
 * Create a single terminal session with PTY WebSocket.
 * @param {HTMLElement} termContainer - DOM element to mount xterm
 * @returns {{term: Terminal, cleanup: Function}}
 */
function createSession(termContainer) {
  const term = new Terminal({
    theme: buildTermTheme(),
    fontFamily: 'monospace',
    fontSize: 14,
    cursorBlink: true,
  });
  const fitAddon = new FitAddon.FitAddon();
  term.loadAddon(fitAddon);
  term.open(termContainer);
  fitAddon.fit();

  let wsHandle = null;

  // Lazy-load ws.js and connect PTY
  import('../lib/ws.js').then(({ connectPtyWS }) => {
    wsHandle = connectPtyWS(
      (msg) => term.write(typeof msg === 'string' ? msg : new Uint8Array(msg)),
      () => term.write('\x1b[32m[connected]\x1b[0m\r\n'),
      () => term.write('\x1b[31m[disconnected]\x1b[0m\r\n'),
    );
    term.onData((data) => wsHandle.send(data));
  });

  const ro = new ResizeObserver(() => fitAddon.fit());
  ro.observe(termContainer);

  return {
    term,
    cleanup() {
      ro.disconnect();
      if (wsHandle) wsHandle.close();
      term.dispose();
    },
  };
}

/**
 * Mount the terminal view with tab management.
 * @param {HTMLElement} container
 * @param {{api: object, store: object}} deps
 * @returns {Function} cleanup
 */
export default function terminal(container, { api, store }) {
  injectStyles();
  container.innerHTML = '';

  const wrapper = document.createElement('div');
  wrapper.className = 'terminal-view';

  // Tab bar
  const tabBar = document.createElement('div');
  tabBar.className = 'terminal-tabs';

  const addBtn = document.createElement('button');
  addBtn.className = 'terminal-tabs__add';
  addBtn.textContent = '+';
  addBtn.title = 'New terminal session';

  // Terminal mount point
  const termContainer = document.createElement('div');
  termContainer.className = 'terminal-container';

  wrapper.append(tabBar, termContainer);
  container.appendChild(wrapper);

  // Session management
  const sessions = [];
  let activeIdx = -1;

  function renderTabs() {
    // Remove old tab buttons (keep add button)
    tabBar.innerHTML = '';
    sessions.forEach((s, i) => {
      const btn = document.createElement('button');
      btn.className = 'terminal-tabs__tab';
      if (i === activeIdx) btn.classList.add('terminal-tabs__tab--active');
      btn.textContent = s.label;
      btn.addEventListener('click', () => switchSession(i));
      tabBar.appendChild(btn);
    });
    tabBar.appendChild(addBtn);
  }

  function switchSession(idx) {
    if (idx === activeIdx || idx < 0 || idx >= sessions.length) return;

    // Detach current terminal
    if (activeIdx >= 0 && sessions[activeIdx]) {
      termContainer.innerHTML = '';
    }

    activeIdx = idx;
    const session = sessions[idx];

    // Re-mount xterm element
    termContainer.innerHTML = '';
    const mount = document.createElement('div');
    mount.style.cssText = 'width:100%;height:100%';
    termContainer.appendChild(mount);

    // If session already created, reopen
    if (session.handle) {
      session.handle.term.open(mount);
    } else {
      session.handle = createSession(mount);
    }

    renderTabs();
  }

  function addSession() {
    const idx = sessions.length;
    sessions.push({
      label: `Shell ${idx + 1}`,
      handle: null,
    });
    switchSession(idx);
  }

  addBtn.addEventListener('click', addSession);

  // Start with one session
  addSession();

  return () => {
    for (const s of sessions) {
      if (s.handle) s.handle.cleanup();
    }
    container.innerHTML = '';
  };
}
