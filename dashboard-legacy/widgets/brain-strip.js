/**
 * Brain strip widget — always-visible compact brain canvas.
 * EMBEDDED mode: 200px height, fire effects + node dots only.
 * IMMERSIVE mode: full viewport with controls and labels.
 * Toggle state synced with brain/canvas.js via window globals.
 */
'use strict';
import { BRAIN_EMBEDDED, BRAIN_IMMERSIVE } from '../brain/config.js';

const EMBEDDED_HEIGHT = 200;
const STYLE_ID = 'mn-brain-strip-widget-style';
const TRANSITION_MS = 300;

function injectStyles() {
  if (document.getElementById(STYLE_ID)) return;
  const s = document.createElement('style');
  s.id = STYLE_ID;
  s.textContent = `
    .brain-strip-widget{position:fixed;bottom:0;left:0;right:0;z-index:90;
      background:var(--mn-surface,#0a0a0a);border-top:1px solid var(--mn-border,#222);
      transition:height ${TRANSITION_MS}ms ease}
    .brain-strip-widget--embedded{height:${EMBEDDED_HEIGHT}px}
    .brain-strip-widget--immersive{height:100vh;height:100dvh}
    .brain-strip-widget__canvas-wrap{width:100%;height:100%;position:relative;overflow:hidden}
    .brain-strip-widget__toolbar{position:absolute;top:8px;right:12px;z-index:95;
      display:flex;gap:6px;align-items:center}
    .brain-strip-widget__btn{background:var(--mn-surface-raised,#1a1a1a);
      border:1px solid var(--mn-border,#333);border-radius:6px;color:var(--mn-text,#ccc);
      padding:4px 10px;font:11px/1 var(--mn-font-mono,monospace);cursor:pointer;
      transition:background 150ms}
    .brain-strip-widget__btn:hover{background:var(--mn-surface-hover,#2a2a2a)}
    .brain-strip-widget__btn:focus-visible{outline:2px solid var(--mn-focus,#4EA8DE);
      outline-offset:2px}
    .brain-strip-widget__stats{font:10px/1 var(--mn-font-mono,monospace);
      color:var(--mn-text-muted,#666);padding:0 8px}`;
  document.head.appendChild(s);
}

/** @returns {'embedded'|'immersive'} */
function loadPersistedMode() {
  const v = localStorage.getItem('cr-brain-strip-mode');
  return v === 'immersive' ? 'immersive' : 'embedded';
}

function persistMode(mode) {
  localStorage.setItem('cr-brain-strip-mode', mode);
}

/**
 * Apply EMBEDDED or IMMERSIVE config to the brain canvas.
 * WHY sync here: brain/canvas.js reads mode from container size,
 * but the strip widget controls the container size.
 */
function applyModeToCanvas(container, mode) {
  const isImmersive = mode === 'immersive';
  container.classList.toggle('brain-strip-widget--embedded', !isImmersive);
  container.classList.toggle('brain-strip-widget--immersive', isImmersive);

  // Notify brain canvas of resize so it recalculates layout
  if (typeof window.brainResize === 'function') {
    requestAnimationFrame(() => window.brainResize());
  }
}

function createToggleButton(mode) {
  const btn = document.createElement('button');
  btn.className = 'brain-strip-widget__btn';
  btn.setAttribute('aria-label', 'Toggle brain view mode');
  updateToggleLabel(btn, mode);
  return btn;
}

function updateToggleLabel(btn, mode) {
  const isEmbedded = mode === 'embedded';
  btn.textContent = isEmbedded ? 'Expand' : 'Collapse';
  btn.setAttribute('aria-expanded', String(!isEmbedded));
}

/**
 * Mount brain strip widget into the target container.
 * @param {string|HTMLElement} target - element or ID to mount into
 */
export default function mountBrainStrip(target) {
  injectStyles();

  const host = typeof target === 'string'
    ? document.getElementById(target)
    : target;
  if (!host) {
    console.warn('[brain-strip-widget] Target element not found');
    return null;
  }

  let currentMode = loadPersistedMode();

  // Build DOM
  const widget = document.createElement('div');
  widget.className = 'brain-strip-widget';
  widget.id = 'brain-strip-widget';

  const canvasWrap = document.createElement('div');
  canvasWrap.className = 'brain-strip-widget__canvas-wrap';
  canvasWrap.id = 'brain-strip-canvas-wrap';

  const toolbar = document.createElement('div');
  toolbar.className = 'brain-strip-widget__toolbar';

  const stats = document.createElement('span');
  stats.className = 'brain-strip-widget__stats';
  stats.id = 'brain-strip-stats';

  const toggleBtn = createToggleButton(currentMode);

  const pauseBtn = document.createElement('button');
  pauseBtn.className = 'brain-strip-widget__btn';
  pauseBtn.textContent = 'Pause';
  pauseBtn.setAttribute('aria-label', 'Pause brain animation');

  toolbar.append(stats, toggleBtn, pauseBtn);
  widget.append(canvasWrap, toolbar);
  host.appendChild(widget);

  // Apply initial mode
  applyModeToCanvas(widget, currentMode);

  // Move existing brain canvas into our wrap, or init fresh
  const existingContainer = document.getElementById('brain-canvas-container');
  if (existingContainer) {
    if (typeof window.brainExpandTo === 'function') {
      window.brainExpandTo(canvasWrap);
    } else {
      canvasWrap.appendChild(existingContainer);
    }
  } else {
    const bc = document.createElement('div');
    bc.id = 'brain-canvas-container';
    bc.style.cssText = 'width:100%;height:100%';
    canvasWrap.appendChild(bc);
    if (typeof window.initBrainCanvas === 'function') {
      window.initBrainCanvas('brain-canvas-container');
    }
  }

  // Toggle handler
  toggleBtn.addEventListener('click', () => {
    currentMode = currentMode === 'embedded' ? 'immersive' : 'embedded';
    persistMode(currentMode);
    applyModeToCanvas(widget, currentMode);
    updateToggleLabel(toggleBtn, currentMode);

    // Dispatch custom event so other components can react
    widget.dispatchEvent(new CustomEvent('brain-strip-mode', {
      bubbles: true,
      detail: {
        mode: currentMode,
        config: currentMode === 'embedded' ? BRAIN_EMBEDDED : BRAIN_IMMERSIVE,
      },
    }));
  });

  // Pause handler
  pauseBtn.addEventListener('click', () => {
    if (typeof window.toggleBrainFreeze === 'function') {
      window.toggleBrainFreeze();
    }
    const paused = pauseBtn.textContent === 'Pause';
    pauseBtn.textContent = paused ? 'Resume' : 'Pause';
  });

  // Click on canvas area expands when in embedded mode
  canvasWrap.addEventListener('dblclick', () => {
    if (currentMode === 'embedded') {
      toggleBtn.click();
    }
  });

  // Keyboard: Escape collapses from immersive
  function onKeyDown(e) {
    if (e.key === 'Escape' && currentMode === 'immersive') {
      toggleBtn.click();
    }
  }
  document.addEventListener('keydown', onKeyDown);

  // Cleanup function
  return function teardown() {
    document.removeEventListener('keydown', onKeyDown);
    widget.remove();
  };
}

export { BRAIN_EMBEDDED, BRAIN_IMMERSIVE };
