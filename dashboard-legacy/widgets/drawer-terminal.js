/**
 * Terminal session factory — shared by drawer-bottom and views/terminal.
 * Creates xterm.js sessions with PTY WebSocket connections.
 */

function getCSSVar(name) {
  return getComputedStyle(document.documentElement)
    .getPropertyValue(name).trim();
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
export function createSession(termContainer) {
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

  import('../lib/ws.js').then(({ connectPtyWS }) => {
    wsHandle = connectPtyWS(
      (msg) => term.write(
        typeof msg === 'string' ? msg : new Uint8Array(msg)
      ),
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
