// WebSocket and SSE connection manager for ConvergioPlatform dashboard.
// No external dependencies. All URLs derived from window.location.

const MAX_RECONNECT = 10;
const MAX_BACKOFF_MS = 30000;

/**
 * Internal: create a WebSocket with exponential-backoff reconnect.
 * @param {string} path - WS endpoint path (e.g. '/ws/dashboard')
 * @param {object} opts - { onMessage, onOpen?, onClose?, onBinary?, binary? }
 * @returns {{ close: function, send: function }}
 */
function createWS(path, { onMessage, onOpen, onClose, onBinary, binary = false }) {
  const proto = location.protocol === 'https:' ? 'wss' : 'ws';
  const url = `${proto}://${location.host}${path}`;
  let ws = null;
  let attempts = 0;
  let closed = false;
  let reconnectTimer = null;

  function connect() {
    if (closed) return;
    ws = new WebSocket(url);
    if (binary) ws.binaryType = 'arraybuffer';

    ws.onopen = () => {
      attempts = 0;
      if (onOpen) onOpen();
    };

    ws.onmessage = (evt) => {
      if (binary && evt.data instanceof ArrayBuffer) {
        if (onBinary) onBinary(evt.data);
        return;
      }
      let data = evt.data;
      try { data = JSON.parse(evt.data); } catch (_) { /* keep string */ }
      onMessage(data);
    };

    ws.onclose = (evt) => {
      if (onClose) onClose(evt.code, evt.reason);
      scheduleReconnect();
    };

    // onerror always followed by onclose; no extra handling needed
    ws.onerror = () => {};
  }

  function scheduleReconnect() {
    if (closed || attempts >= MAX_RECONNECT) return;
    const delay = Math.min(1000 * Math.pow(2, attempts), MAX_BACKOFF_MS);
    attempts++;
    reconnectTimer = setTimeout(connect, delay);
  }

  function close() {
    closed = true;
    clearTimeout(reconnectTimer);
    if (ws) {
      ws.onclose = null; // prevent reconnect on intentional close
      ws.close();
      ws = null;
    }
  }

  function send(data) {
    if (!ws || ws.readyState !== WebSocket.OPEN) return;
    ws.send(typeof data === 'string' ? data : JSON.stringify(data));
  }

  connect();
  return { close, send };
}

/** Connect to /ws/dashboard. Returns { close, send }. */
export function connectDashboardWS(onMessage, onOpen, onClose) {
  return createWS('/ws/dashboard', { onMessage, onOpen, onClose });
}

/** Connect to /ws/brain. Returns { close, send }. */
export function connectBrainWS(onMessage, onOpen, onClose) {
  return createWS('/ws/brain', { onMessage, onOpen, onClose });
}

/** Connect to /ws/pty in binary mode. Returns { close, send }. */
export function connectPtyWS(onMessage, onOpen, onClose, onBinary) {
  return createWS('/ws/pty', { onMessage, onOpen, onClose, onBinary, binary: true });
}

/**
 * Connect to an SSE endpoint. Parses named events (event:/data: pairs).
 * @param {string} url - SSE endpoint path (e.g. '/api/stream/agents')
 * @param {object} handlers - { onEvent(type, data), onDone?(data), onError?(err) }
 * @returns {{ close: function }}
 */
export function streamSSE(url, handlers) {
  const { onEvent, onDone, onError } = handlers;
  const controller = new AbortController();

  (async () => {
    try {
      const res = await fetch(url, {
        headers: { 'Accept': 'text/event-stream' },
        signal: controller.signal,
      });
      if (!res.ok) {
        if (onError) onError(new Error(`SSE ${res.status} ${res.statusText}`));
        return;
      }

      const reader = res.body.getReader();
      const decoder = new TextDecoder();
      let buffer = '';
      let currentEvent = 'message';
      let currentData = '';

      function dispatch(evtName, raw) {
        let parsed = raw;
        try { parsed = JSON.parse(raw); } catch (_) { /* keep string */ }
        if (evtName === 'done') {
          if (onDone) onDone(parsed);
        } else {
          if (onEvent) onEvent(evtName, parsed);
        }
      }

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        const lines = buffer.split('\n');
        buffer = lines.pop(); // keep incomplete last line

        for (const line of lines) {
          if (line.startsWith('event:')) {
            currentEvent = line.slice(6).trim();
          } else if (line.startsWith('data:')) {
            currentData += (currentData ? '\n' : '') + line.slice(5).trim();
          } else if (line === '') {
            if (currentData) dispatch(currentEvent, currentData);
            currentEvent = 'message';
            currentData = '';
          }
        }
      }

      // Stream ended — flush remaining data
      if (currentData) dispatch(currentEvent, currentData);
    } catch (err) {
      if (err.name === 'AbortError') return;
      if (onError) onError(err);
    }
  })();

  return { close: () => controller.abort() };
}
