/**
 * Brain view — neural visualization wrapper with controls and legend.
 * Connects to /ws/brain and delegates rendering to brain-canvas.js.
 */

import { connectBrainWS } from '../lib/ws.js';

const { StateScaffold } = window.Maranello;

const NODE_TYPES = [
  { key: 'session', label: 'Session', cssVar: '--mn-brain-session' },
  { key: 'plan', label: 'Plan', cssVar: '--mn-brain-plan' },
  { key: 'task', label: 'Task', cssVar: '--mn-brain-task' },
  { key: 'agent', label: 'Agent', cssVar: '--mn-brain-agent' },
  { key: 'mesh', label: 'Mesh Node', cssVar: '--mn-brain-mesh' },
];

/**
 * Build legend HTML from node type definitions.
 * Reads CSS custom properties for colors — no hardcoded values.
 * @returns {string}
 */
function buildLegend() {
  return NODE_TYPES.map(({ label, cssVar }) =>
    `<span class="brain-legend__item" style="display:inline-flex;align-items:center;gap:0.35rem;font-size:0.8rem;color:var(--mn-text-muted)">
      <span style="width:10px;height:10px;border-radius:50%;background:var(${cssVar},var(--mn-text-muted))"></span>
      ${label}
    </span>`
  ).join('');
}

/**
 * Brain view factory.
 * @param {HTMLElement} container — mount target
 * @param {{api: object, store: object}} deps
 * @returns {Function} teardown callback
 */
export default function brain(container, { api, store }) {
  const scaffold = new StateScaffold(container, {
    state: 'loading',
    onRetry: () => initConnection(),
  });

  container.innerHTML = `
    <div class="brain-controls" style="display:flex;gap:0.5rem;margin-bottom:1rem;align-items:center">
      <button class="mn-btn mn-btn--sm" id="brain-pause">Pause</button>
      <button class="mn-btn mn-btn--sm" id="brain-reset">Reset</button>
      <label style="color:var(--mn-text-muted);font-size:0.875rem">
        Zoom: <input type="range" id="brain-zoom" min="0.5" max="3" step="0.1" value="1">
      </label>
      <span id="brain-status" class="mn-text--sm" style="margin-left:auto;color:var(--mn-text-muted)">
        Connecting...
      </span>
    </div>
    <div id="brain-canvas-host" style="flex:1;min-height:400px;position:relative"></div>
    <div class="brain-legend" style="display:flex;gap:1rem;margin-top:0.75rem;flex-wrap:wrap">
      ${buildLegend()}
    </div>
  `;

  const host = container.querySelector('#brain-canvas-host');
  const statusEl = container.querySelector('#brain-status');
  const pauseBtn = container.querySelector('#brain-pause');
  const resetBtn = container.querySelector('#brain-reset');
  const zoomInput = container.querySelector('#brain-zoom');

  let brainInstance = null;
  let ws = null;
  let paused = false;

  /**
   * Lazy-load brain-canvas.js and initialise the renderer.
   * Uses the global initBrain exposed by brain-canvas.js IIFE.
   */
  async function initRenderer() {
    if (typeof window.initBrain === 'function') {
      return window.initBrain(host);
    }
    // Fallback: brain-canvas.js may not expose global; warn visibly
    console.warn('[brain] initBrain not available — brain-canvas.js may not be loaded');
    return null;
  }

  /** Open WS and wire message routing to renderer. */
  async function initConnection() {
    scaffold.state = 'loading';
    brainInstance = await initRenderer();

    if (!brainInstance) {
      scaffold.state = 'error';
      return;
    }

    ws = connectBrainWS(
      (msg) => {
        if (!paused && brainInstance) {
          brainInstance.onMessage(msg);
        }
      },
      () => {
        statusEl.textContent = 'Connected';
        scaffold.state = 'ready';
      },
      (_code, _reason) => {
        statusEl.textContent = 'Disconnected';
      },
    );
  }

  // Wire control buttons
  pauseBtn.onclick = () => {
    paused = !paused;
    pauseBtn.textContent = paused ? 'Resume' : 'Pause';
    if (brainInstance?.togglePause) brainInstance.togglePause();
  };

  resetBtn.onclick = () => {
    if (brainInstance?.reset) brainInstance.reset();
    zoomInput.value = '1';
  };

  zoomInput.oninput = (e) => {
    const zoom = parseFloat(e.target.value);
    if (brainInstance?.setZoom) brainInstance.setZoom(zoom);
  };

  initConnection();

  // Refresh on store events (e.g. new plan/task activity)
  const unsub = store.subscribe('brain', () => {
    if (brainInstance?.refresh) brainInstance.refresh();
  });

  return () => {
    unsub();
    if (ws) ws.close();
    if (brainInstance?.destroy) brainInstance.destroy();
    container.innerHTML = '';
  };
}
